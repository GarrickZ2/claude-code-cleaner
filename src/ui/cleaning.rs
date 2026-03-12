use crate::app::App;
use crate::cleaner::CleanMessage;
use crate::ui::widgets;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Progress bar + status
            Constraint::Min(0),   // Log
            Constraint::Length(3), // Summary (if complete)
        ])
        .split(area);

    let dry_run = app.settings.dry_run;
    let block_title = if dry_run { " Clean [DRY RUN] " } else { " Clean " };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(block_title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(block, area);

    // Progress bar + status area
    if app.clean_complete {
        // Completed: full green bar
        let bar_width = chunks[0].width.saturating_sub(2) as usize;
        let bar_line = Line::from(vec![
            Span::styled(
                "\u{2588}".repeat(bar_width),
                Style::default().fg(if dry_run { Color::Cyan } else { Color::Green }),
            ),
        ]);
        let status_line = if dry_run {
            Line::from(vec![
                Span::styled(" DRY RUN COMPLETE ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(format!(
                    " Would free {} | {} errors",
                    widgets::format_size(app.clean_total_freed),
                    app.clean_total_errors.len(),
                )),
            ])
        } else {
            Line::from(vec![
                Span::styled(" COMPLETE ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(format!(
                    " Freed {} | {} errors",
                    widgets::format_size(app.clean_total_freed),
                    app.clean_total_errors.len(),
                )),
            ])
        };
        let bar = Paragraph::new(vec![status_line, bar_line]).block(Block::default());
        f.render_widget(bar, chunks[0]);
    } else if app.cleaning {
        // In progress: animated bar
        let ratio = if app.clean_expected_size > 0 {
            (app.clean_freed_so_far as f64 / app.clean_expected_size as f64).min(1.0)
        } else {
            0.0
        };
        let pct = (ratio * 100.0) as u32;

        let bar_width = chunks[0].width.saturating_sub(2) as usize;
        let filled = ((ratio * bar_width as f64).round() as usize).min(bar_width);
        let empty = bar_width.saturating_sub(filled);

        let bar_color = if dry_run { Color::Cyan } else { Color::Green };
        let bar_line = Line::from(vec![
            Span::styled("\u{2588}".repeat(filled), Style::default().fg(bar_color)),
            Span::styled("\u{2591}".repeat(empty), Style::default().fg(Color::DarkGray)),
        ]);

        let status_line = Line::from(vec![
            Span::styled(
                if dry_run { " DRY RUN... " } else { " CLEANING... " },
                Style::default().fg(bar_color).add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK),
            ),
            Span::raw(format!(
                " {}% | {} / {}",
                pct,
                widgets::format_size(app.clean_freed_so_far),
                widgets::format_size(app.clean_expected_size),
            )),
        ]);

        let bar = Paragraph::new(vec![status_line, bar_line]).block(Block::default());
        f.render_widget(bar, chunks[0]);
    } else {
        let status_text = Line::from(Span::styled(
            " Ready. Go to Preview and press Enter to start.",
            Style::default().fg(Color::DarkGray),
        ));
        f.render_widget(Paragraph::new(status_text), chunks[0]);
    }

    // Log messages — show latest entries (auto-scroll to bottom)
    let log_height = chunks[1].height as usize;
    let items: Vec<ListItem> = app
        .clean_messages
        .iter()
        .filter_map(|msg| match msg {
            CleanMessage::Progress {
                category,
                current_file,
                bytes_freed: _,
                files_done,
                total_files,
            } => Some(ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" [{}/{}] ", files_done, total_files),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(category, Style::default().fg(Color::Cyan)),
                Span::raw(": "),
                Span::raw(current_file),
            ]))),
            CleanMessage::CategoryDone {
                category,
                freed,
                errors,
            } => {
                let style = if errors.is_empty() {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Yellow)
                };
                let done_label = if dry_run { " WOULD " } else { " DONE " };
                let freed_label = if dry_run { " would free " } else { " freed " };
                Some(ListItem::new(Line::from(vec![
                    Span::styled(done_label, style),
                    Span::styled(category, Style::default().fg(Color::White)),
                    Span::raw(format!("{}{}", freed_label, widgets::format_size(*freed))),
                    if !errors.is_empty() {
                        Span::styled(
                            format!(" ({} errors)", errors.len()),
                            Style::default().fg(Color::Red),
                        )
                    } else {
                        Span::raw("")
                    },
                ])))
            }
            CleanMessage::Complete { .. } => None,
            CleanMessage::Error(e) => Some(ListItem::new(Line::from(Span::styled(
                format!(" ERROR: {}", e),
                Style::default().fg(Color::Red),
            )))),
        })
        .collect();

    // Auto-scroll: only show the last N items that fit in the log area
    let visible_items = if items.len() > log_height {
        items[items.len() - log_height..].to_vec()
    } else {
        items
    };

    let list = List::new(visible_items).block(Block::default());
    f.render_widget(list, chunks[1]);

    // Summary footer
    if app.clean_complete {
        let summary_text = if dry_run {
            format!("Would free: {} (no files were deleted)", widgets::format_size(app.clean_total_freed))
        } else {
            format!("Total freed: {}", widgets::format_size(app.clean_total_freed))
        };
        let summary_color = if dry_run { Color::Cyan } else { Color::Green };
        let summary = Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                summary_text,
                Style::default().fg(summary_color).add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled("[q]", Style::default().fg(Color::Yellow)),
            Span::raw(" Quit  "),
            Span::styled("[s]", Style::default().fg(Color::Yellow)),
            Span::raw(" Rescan"),
        ]));
        f.render_widget(summary, chunks[2]);
    }
}
