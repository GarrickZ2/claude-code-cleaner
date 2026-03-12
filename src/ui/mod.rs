pub mod categories;
pub mod cleaning;
pub mod dashboard;
pub mod preview;
pub mod projects;
pub mod widgets;

use crate::app::{App, Screen};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

const LOGO: &[&str] = &[
    r" ╔═╗┬  ┌─┐┬ ┬┌┬┐┌─┐  ╔═╗┌─┐┌┬┐┌─┐  ╔═╗┬  ┌─┐┌─┐┌┐┌┌─┐┬─┐",
    r" ║  │  ├─┤│ │ ││├┤   ║  │ │ ││├┤   ║  │  ├┤ ├─┤│││├┤ ├┬┘",
    r" ╚═╝┴─┘┴ ┴└─┘─┴┘└─┘  ╚═╝└─┘─┴┘└─┘  ╚═╝┴─┘└─┘┴ ┴┘└┘└─┘┴└─",
];

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Decide logo size based on terminal height
    let use_full_logo = area.height >= 28;
    let logo_height = if use_full_logo { LOGO.len() as u16 } else { 1 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(logo_height), // ASCII art header
            Constraint::Length(1),           // Horizontal rule
            Constraint::Length(1),           // Progress bar
            Constraint::Min(0),              // Content
            Constraint::Length(1),           // Status bar
        ])
        .split(area);

    render_header(f, app, chunks[0], use_full_logo);
    render_hr(f, area.width, chunks[1]);
    render_progress_bar(f, app, chunks[2]);

    match app.screen {
        Screen::Dashboard => dashboard::render(f, app, chunks[3]),
        Screen::Categories => categories::render(f, app, chunks[3]),
        Screen::Projects => projects::render(f, app, chunks[3]),
        Screen::Preview => preview::render(f, app, chunks[3]),
        Screen::Cleaning => cleaning::render(f, app, chunks[3]),
    }

    render_status_bar(f, app, chunks[4]);

    if app.show_help {
        render_help_overlay(f, f.area());
    }

    if app.show_confirm {
        render_confirm_dialog(f, f.area(), app.settings.dry_run);
    }
}

fn render_header(f: &mut Frame, _app: &App, area: Rect, use_full_logo: bool) {
    let width = area.width as usize;

    if use_full_logo {
        let widest = LOGO.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let base_pad = if width > widest {
            (width - widest) / 2
        } else {
            0
        };
        let padding = " ".repeat(base_pad);

        let lines: Vec<Line> = LOGO
            .iter()
            .map(|line| {
                Line::from(vec![
                    Span::raw(padding.clone()),
                    Span::styled(*line, Style::default().fg(Color::Cyan)),
                ])
            })
            .collect();

        f.render_widget(Paragraph::new(lines), area);
    } else {
        let text = "Claude Code Cleaner";
        let pad = if width > text.len() {
            (width - text.len()) / 2
        } else {
            0
        };
        let line = Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled(
                text,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        f.render_widget(Paragraph::new(line), area);
    }
}

fn render_hr(f: &mut Frame, width: u16, area: Rect) {
    let line_str = "\u{2500}".repeat(width as usize); // ─
    let line = Line::from(Span::styled(line_str, Style::default().fg(Color::DarkGray)));
    f.render_widget(Paragraph::new(line), area);
}

fn render_progress_bar(f: &mut Frame, app: &App, area: Rect) {
    let current = app.screen.index();
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::raw("  "));

    for (i, screen) in Screen::ALL.iter().enumerate() {
        let is_current = i == current;
        let is_done = i < current;

        // Step indicator
        let (dot, dot_style) = if is_current {
            (
                "\u{25cf}",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ) // ●
        } else if is_done {
            ("\u{25cf}", Style::default().fg(Color::Green)) // ●
        } else {
            ("\u{25cb}", Style::default().fg(Color::DarkGray)) // ○
        };

        spans.push(Span::styled(dot, dot_style));
        spans.push(Span::raw(" "));

        // Step label
        let label_style = if is_current {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if is_done {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(screen.title(), label_style));

        // Connector line between steps
        if i < Screen::ALL.len() - 1 {
            let line_style = if is_done {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            spans.push(Span::styled(" \u{2500}\u{2500}\u{2500} ", line_style)); // ───
        }
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status = if app.scanning {
        app.scan_progress
            .clone()
            .unwrap_or_else(|| "Scanning...".into())
    } else if app.cleaning {
        if app.settings.dry_run {
            "DRY RUN in progress (no files will be deleted)...".into()
        } else {
            "Cleaning in progress...".into()
        }
    } else {
        let nav_hint = match app.screen {
            Screen::Dashboard => "Enter:Next  s:Rescan  ?:Help  q:Quit",
            Screen::Categories => {
                "Esc:Back  Enter:Next  Space:Toggle  Left/Right:Adjust  a/n/d:Select"
            }
            Screen::Projects => "Esc:Back  Enter:Next  Space:Toggle  /:Search",
            Screen::Preview => "Esc:Back  Enter:Execute  ?:Help",
            Screen::Cleaning => "s:Rescan  q:Quit",
        };
        let dry_run_hint = if app.settings.dry_run {
            "  [DRY RUN]"
        } else {
            ""
        };
        format!("{}{}", nav_hint, dry_run_hint)
    };

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(status, Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(bar, area);
}

fn render_help_overlay(f: &mut Frame, area: Rect) {
    let help_area = centered_rect(60, 70, area);
    f.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw("            Next step"),
        ]),
        Line::from(vec![
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw("              Previous step"),
        ]),
        Line::from(vec![
            Span::styled("1-5", Style::default().fg(Color::Yellow)),
            Span::raw("              Jump to step"),
        ]),
        Line::from(vec![
            Span::styled("j/k or Up/Down", Style::default().fg(Color::Yellow)),
            Span::raw("  Navigate lists"),
        ]),
        Line::from(vec![
            Span::styled("Space", Style::default().fg(Color::Yellow)),
            Span::raw("            Toggle selection"),
        ]),
        Line::from(vec![
            Span::styled("s", Style::default().fg(Color::Yellow)),
            Span::raw("                Start scan"),
        ]),
        Line::from(vec![
            Span::styled("a", Style::default().fg(Color::Yellow)),
            Span::raw("                Select all"),
        ]),
        Line::from(vec![
            Span::styled("n", Style::default().fg(Color::Yellow)),
            Span::raw("                Select none"),
        ]),
        Line::from(vec![
            Span::styled("d", Style::default().fg(Color::Yellow)),
            Span::raw("                Default selection"),
        ]),
        Line::from(vec![
            Span::styled("Left/Right", Style::default().fg(Color::Yellow)),
            Span::raw("       Adjust settings value"),
        ]),
        Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw("                Search filter (Projects)"),
        ]),
        Line::from(vec![
            Span::styled("q / Ctrl-C", Style::default().fg(Color::Yellow)),
            Span::raw("       Quit"),
        ]),
        Line::from(vec![
            Span::styled("?", Style::default().fg(Color::Yellow)),
            Span::raw("                Toggle this help"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press ? or Esc to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help ")
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(help, help_area);
}

fn render_confirm_dialog(f: &mut Frame, area: Rect, dry_run: bool) {
    let dialog_area = centered_rect(50, 30, area);
    f.render_widget(Clear, dialog_area);

    let mut text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Are you sure you want to proceed with cleaning?",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if dry_run {
        text.push(Line::from(Span::styled(
            "DRY RUN: No files will actually be deleted.",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
    } else {
        text.push(Line::from(Span::styled(
            "WARNING: This action cannot be undone!",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }

    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled(
            "  Enter",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" = Confirm    "),
        Span::styled(
            "Esc",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" = Cancel"),
    ]));

    let title = if dry_run {
        " Confirm Dry Run "
    } else {
        " Confirm Clean "
    };
    let title_color = if dry_run { Color::Cyan } else { Color::Red };

    let dialog = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Style::default().fg(title_color)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(dialog, dialog_area);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
