use crate::app::App;
use crate::ui::widgets;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, Wrap};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header info
            Constraint::Min(0),   // Category table
            Constraint::Length(3), // Footer
        ])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Step 1: Scan ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(block, area);

    // Header
    if let Some(ref result) = app.scan_result {
        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  ~/.claude/", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("                                    "),
                Span::styled(
                    format!("Total: {}  {} files", widgets::format_size(result.total_size), result.total_files),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
        ]);
        f.render_widget(header, chunks[0]);

        // Category table
        let header_row = Row::new(vec!["Category", "Size", "Files", "Oldest", "Usage"])
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let rows: Vec<Row> = result
            .categories
            .iter()
            .map(|cat| {
                let pct = cat.size_percentage(result.total_size);
                let bar = widgets::bar_chart(pct / 100.0, 16);
                let age = cat
                    .oldest_modified
                    .as_ref()
                    .map(|d| widgets::format_age(d))
                    .unwrap_or_else(|| "-".to_string());

                Row::new(vec![
                    cat.category.to_string(),
                    widgets::format_size(cat.size),
                    format!("{}", cat.file_count),
                    age,
                    format!("{} {:>4.0}%", bar, pct),
                ])
                .style(if cat.size > 100_000_000 {
                    Style::default().fg(Color::Red)
                } else if cat.size > 10_000_000 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                })
            })
            .collect();

        let table = Table::new(
            rows,
            &[
                Constraint::Length(16),
                Constraint::Length(10),
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Min(22),
            ],
        )
        .header(header_row)
        .block(Block::default());

        f.render_widget(table, chunks[1]);

        // Footer: reclaimable
        let reclaimable = result.reclaimable_size(app.settings.expiry_days);
        let footer = Paragraph::new(vec![Line::from(vec![
            Span::raw("  Reclaimable (current settings): "),
            Span::styled(
                format!("~{}", widgets::format_size(reclaimable)),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled("[S]", Style::default().fg(Color::Yellow)),
            Span::raw(" Scan  "),
            Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
            Span::raw(" Browse  "),
            Span::styled("[?]", Style::default().fg(Color::Yellow)),
            Span::raw(" Help"),
        ])]);
        f.render_widget(footer, chunks[2]);
    } else {
        let msg = if app.scanning {
            app.scan_progress.clone().unwrap_or_else(|| "Scanning...".into())
        } else {
            "Press [S] to scan ~/.claude/ directory".into()
        };
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(msg, Style::default().fg(Color::DarkGray))),
        ])
        .wrap(Wrap { trim: false })
        .block(Block::default());
        f.render_widget(p, chunks[1]);
    }
}
