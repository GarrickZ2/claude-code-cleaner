mod app;
mod cleaner;
mod event;
mod model;
mod scanner;
mod ui;

use app::{App, Screen};
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use event::Event;
use model::Category;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal).await;

    // Teardown terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> color_eyre::Result<()> {
    let claude_dir = dirs::home_dir()
        .ok_or_else(|| color_eyre::eyre::eyre!("Cannot find home directory"))?
        .join(".claude");

    let mut app = App::new(&claude_dir);

    let (mut event_handler, event_tx) = event::EventHandler::new(Duration::from_millis(100));

    // Auto-scan on startup
    start_scan(&claude_dir, &event_tx, &mut app);

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        // Batch-drain: wait for first event, then grab everything buffered.
        // This avoids rendering after every single event when input floods in.
        let batch = event_handler.next_batch().await;
        if batch.is_empty() {
            break; // channel closed
        }

        // Coalesce repeated navigation keys: count net direction instead of
        // processing each Up/Down individually when hundreds are queued.
        let coalesced = coalesce_events(batch);

        for evt in coalesced {
            match evt {
                Event::Key(key) => {
                    handle_key_event(&mut app, key, &claude_dir, &event_tx);
                }
                Event::Tick => {}
                Event::ScanMessage(msg) => app.handle_scan_message(msg),
                Event::CleanMessage(msg) => {
                    let is_complete = matches!(&msg, cleaner::CleanMessage::Complete { .. });
                    app.handle_clean_message(msg);
                    if is_complete {
                        app.save_preferences(&claude_dir);
                    }
                }
            }
            if !app.running {
                break;
            }
        }

        if !app.running {
            break;
        }
    }

    Ok(())
}

/// Coalesce a batch of events: collapse consecutive same-direction navigation
/// keys into a single key with a repeat count encoded via synthetic events.
/// This prevents UI freeze when thousands of Up/Down/scroll events are queued.
fn coalesce_events(events: Vec<Event>) -> Vec<Event> {
    use crossterm::event::{KeyCode, KeyEventKind};

    let mut result: Vec<Event> = Vec::new();

    for evt in events {
        match &evt {
            Event::Key(key) => {
                // Only coalesce press events for navigation keys
                if key.kind != KeyEventKind::Press {
                    continue; // drop Release/Repeat events entirely
                }
                match key.code {
                    KeyCode::Up | KeyCode::Down
                    | KeyCode::Char('j') | KeyCode::Char('k')
                    | KeyCode::Left | KeyCode::Right
                    | KeyCode::Char('h') | KeyCode::Char('l') => {
                        // If last event in result is the same key, skip (we'll
                        // handle the net effect). But keep at most ~1 event per
                        // direction change so cursor still responds smoothly.
                        if let Some(Event::Key(last)) = result.last() {
                            if last.code == key.code && last.modifiers == key.modifiers {
                                // Same key repeated — collapse by incrementing count
                                // We store the count by just keeping one representative
                                // and will apply it via saturating arithmetic in handler.
                                // For simplicity, keep at most a handful of same-key events.
                                let same_count = result.iter().rev().take_while(|e| {
                                    matches!(e, Event::Key(k) if k.code == key.code && k.modifiers == key.modifiers)
                                }).count();
                                // Cap at 20 to keep responsiveness without over-scrolling
                                if same_count >= 20 {
                                    continue; // drop excess
                                }
                            }
                        }
                        result.push(evt);
                    }
                    _ => result.push(evt),
                }
            }
            Event::Tick => {
                // Keep only the last Tick — no point processing multiple ticks
                if !result.iter().any(|e| matches!(e, Event::Tick)) {
                    result.push(evt);
                }
            }
            _ => result.push(evt), // ScanMessage/CleanMessage: always keep
        }
    }

    result
}

fn handle_key_event(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    claude_dir: &std::path::Path,
    event_tx: &mpsc::UnboundedSender<Event>,
) {
    // Global: Ctrl-C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.running = false;
        return;
    }

    // Only handle Press events (Release/Repeat already filtered by coalesce,
    // but guard here too for safety)
    if key.kind != crossterm::event::KeyEventKind::Press {
        return;
    }

    // Handle confirm dialog
    if app.show_confirm {
        match key.code {
            KeyCode::Enter => {
                app.show_confirm = false;
                // Record expected size for progress bar
                if let Some(ref result) = app.scan_result {
                    app.clean_expected_size = result.reclaimable_size(app.settings.expiry_days);
                }
                app.clean_freed_so_far = 0;
                start_clean(app, event_tx);
                app.cleaning = true;
                app.clean_complete = false;
                app.clean_messages.clear();
                app.screen = Screen::Cleaning;
            }
            KeyCode::Esc => {
                app.show_confirm = false;
            }
            _ => {}
        }
        return;
    }

    // Handle help overlay
    if app.show_help {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc => {
                app.show_help = false;
            }
            _ => {}
        }
        return;
    }

    // Handle project filter input mode
    if app.project_filtering {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                app.project_filtering = false;
            }
            KeyCode::Backspace => {
                app.project_filter.pop();
            }
            KeyCode::Char(c) => {
                app.project_filter.push(c);
            }
            _ => {}
        }
        return;
    }

    // Global keys
    match key.code {
        KeyCode::Char('q') => { app.running = false; return; }
        KeyCode::Char('?') => { app.show_help = !app.show_help; return; }
        // Number keys to jump to specific step
        KeyCode::Char('1') => { app.screen = Screen::Dashboard; return; }
        KeyCode::Char('2') => { app.screen = Screen::Categories; return; }
        KeyCode::Char('3') => { app.screen = Screen::Projects; return; }
        KeyCode::Char('4') => { app.screen = Screen::Preview; return; }
        KeyCode::Char('5') => { app.screen = Screen::Cleaning; return; }
        _ => {}
    }

    // Screen-specific keys (Enter/Esc handled per-screen for flow control)
    match app.screen {
        Screen::Dashboard => handle_dashboard_keys(app, key.code, claude_dir, event_tx),
        Screen::Categories => handle_categories_keys(app, key.code),
        Screen::Projects => handle_projects_keys(app, key.code),
        Screen::Preview => handle_preview_keys(app, key.code),
        Screen::Cleaning => handle_cleaning_keys(app, key.code, claude_dir, event_tx),
    }
}

fn start_scan(
    claude_dir: &std::path::Path,
    event_tx: &mpsc::UnboundedSender<Event>,
    app: &mut App,
) {
    app.scanning = true;
    app.scan_progress = Some("Starting scan...".into());
    let dir = claude_dir.to_path_buf();
    let tx = event_tx.clone();
    tokio::spawn(async move {
        let (scan_tx, mut scan_rx) = mpsc::unbounded_channel();
        tokio::spawn(scanner::deep_scan(dir, scan_tx));
        while let Some(msg) = scan_rx.recv().await {
            if tx.send(Event::ScanMessage(msg)).is_err() {
                break;
            }
        }
    });
}

fn start_clean(app: &App, event_tx: &mpsc::UnboundedSender<Event>) {
    if let Some(ref result) = app.scan_result {
        let result_clone = result.clone();
        let settings_clone = app.settings.clone();
        let tx = event_tx.clone();
        tokio::spawn(async move {
            let (clean_tx, mut clean_rx) = mpsc::unbounded_channel();
            tokio::spawn(cleaner::execute_clean(
                Box::leak(Box::new(result_clone)),
                Box::leak(Box::new(settings_clone)),
                clean_tx,
            ));
            while let Some(msg) = clean_rx.recv().await {
                if tx.send(Event::CleanMessage(msg)).is_err() {
                    break;
                }
            }
        });
    }
}

fn handle_dashboard_keys(
    app: &mut App,
    key: KeyCode,
    claude_dir: &std::path::Path,
    event_tx: &mpsc::UnboundedSender<Event>,
) {
    match key {
        KeyCode::Char('s') if !app.scanning => {
            start_scan(claude_dir, event_tx, app);
        }
        KeyCode::Enter => {
            if app.scan_result.is_some() {
                app.next_screen();
            }
        }
        _ => {}
    }
}

/// Check if the Projects category is selected
fn is_projects_selected(app: &App) -> bool {
    app.scan_result
        .as_ref()
        .map(|r| {
            r.categories
                .iter()
                .any(|c| c.category == Category::Projects && c.selected)
        })
        .unwrap_or(false)
}

fn handle_categories_keys(app: &mut App, key: KeyCode) {
    use crate::app::SelectSection;

    let total = app.select_total_items();
    if total == 0 {
        return;
    }

    match key {
        KeyCode::Enter => {
            if is_projects_selected(app) {
                // Auto-select: orphans get selected, active projects with expired data too
                if let Some(ref mut result) = app.scan_result {
                    let expiry = app.settings.expiry_days;
                    for proj in &mut result.projects {
                        proj.selected = proj.is_orphan || proj.expired_size(expiry) > 0;
                    }
                }
                app.screen = Screen::Projects;
            } else {
                app.screen = Screen::Preview;
            }
            return;
        }
        KeyCode::Esc => {
            app.prev_screen();
            return;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.category_cursor > 0 {
                app.category_cursor -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.category_cursor < total - 1 {
                app.category_cursor += 1;
            }
        }
        KeyCode::Char(' ') => {
            let (section, idx) = app.select_cursor_section();
            match section {
                SelectSection::Categories => {
                    if let Some(ref mut result) = app.scan_result {
                        if let Some(cat) = result.categories.get_mut(idx) {
                            cat.selected = !cat.selected;
                        }
                    }
                }
                SelectSection::ConfigJson => {
                    if let Some(ref mut result) = app.scan_result {
                        match idx {
                            0 => result.config_json.orphan_projects_selected = !result.config_json.orphan_projects_selected,
                            1 => result.config_json.metrics_selected = !result.config_json.metrics_selected,
                            2 => result.config_json.cache_selected = !result.config_json.cache_selected,
                            _ => {}
                        }
                    }
                }
                SelectSection::Settings => {
                    app.settings.increment(idx);
                }
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            let (section, idx) = app.select_cursor_section();
            if section == SelectSection::Settings {
                app.settings.increment(idx);
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            let (section, idx) = app.select_cursor_section();
            if section == SelectSection::Settings {
                app.settings.decrement(idx);
            }
        }
        KeyCode::Char('a') => {
            if let Some(ref mut result) = app.scan_result {
                for cat in &mut result.categories {
                    cat.selected = true;
                }
                result.config_json.orphan_projects_selected = true;
                result.config_json.metrics_selected = true;
                result.config_json.cache_selected = true;
            }
        }
        KeyCode::Char('n') => {
            if let Some(ref mut result) = app.scan_result {
                for cat in &mut result.categories {
                    cat.selected = false;
                }
                result.config_json.orphan_projects_selected = false;
                result.config_json.metrics_selected = false;
                result.config_json.cache_selected = false;
            }
        }
        KeyCode::Char('d') => {
            if let Some(ref mut result) = app.scan_result {
                for cat in &mut result.categories {
                    cat.selected = Category::DEFAULT_SELECTED.contains(&cat.category);
                }
                result.config_json.orphan_projects_selected = true;
                result.config_json.metrics_selected = true;
                result.config_json.cache_selected = true;
            }
        }
        _ => {}
    }
}

const PROJECT_VISIBLE_ROWS: usize = 20;

fn handle_projects_keys(app: &mut App, key: KeyCode) {
    let filtered = app.filtered_projects();
    let count = filtered.len();

    match key {
        KeyCode::Enter => {
            app.screen = Screen::Preview;
            return;
        }
        KeyCode::Esc => {
            app.screen = Screen::Categories;
            return;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.project_cursor > 0 {
                app.project_cursor -= 1;
                app.adjust_project_scroll(PROJECT_VISIBLE_ROWS);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if count > 0 && app.project_cursor < count - 1 {
                app.project_cursor += 1;
                app.adjust_project_scroll(PROJECT_VISIBLE_ROWS);
            }
        }
        KeyCode::Char(' ') => {
            if let Some(&proj_idx) = filtered.get(app.project_cursor) {
                if let Some(ref mut result) = app.scan_result {
                    if let Some(proj) = result.projects.get_mut(proj_idx) {
                        proj.selected = !proj.selected;
                    }
                }
            }
        }
        KeyCode::Char('a') => {
            // Select all projects
            if let Some(ref mut result) = app.scan_result {
                for proj in &mut result.projects {
                    proj.selected = true;
                }
            }
        }
        KeyCode::Char('o') => {
            // Select orphan projects only
            if let Some(ref mut result) = app.scan_result {
                for proj in &mut result.projects {
                    proj.selected = proj.is_orphan;
                }
            }
        }
        KeyCode::Char('n') => {
            // Unselect all
            if let Some(ref mut result) = app.scan_result {
                for proj in &mut result.projects {
                    proj.selected = false;
                }
            }
        }
        KeyCode::Char('/') => {
            app.project_filtering = true;
        }
        _ => {}
    }
}

fn handle_preview_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => {
            app.show_confirm = true;
        }
        KeyCode::Esc => {
            // Go back to Projects if Projects category is selected, else Categories
            if is_projects_selected(app) {
                app.screen = Screen::Projects;
            } else {
                app.screen = Screen::Categories;
            }
        }
        _ => {}
    }
}

fn handle_cleaning_keys(
    app: &mut App,
    key: KeyCode,
    claude_dir: &std::path::Path,
    event_tx: &mpsc::UnboundedSender<Event>,
) {
    if app.clean_complete {
        match key {
            KeyCode::Char('s') => {
                // Rescan
                app.clean_complete = false;
                app.clean_messages.clear();
                start_scan(claude_dir, event_tx, app);
                app.screen = Screen::Dashboard;
            }
            _ => {}
        }
    }
}
