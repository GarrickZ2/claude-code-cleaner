use crate::app::{App, SelectSection};
use crate::model::CleanSettings;
use crate::ui::widgets;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(0),    // All items
            Constraint::Length(3), // Summary
        ])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Select & Configure ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(block, area);

    if let Some(ref result) = app.scan_result {
        let (cur_section, cur_local) = app.select_cursor_section();
        let mut items: Vec<ListItem> = Vec::new();

        // ── Section 1: Categories (directories under ~/.claude/) ──
        for (i, cat) in result.categories.iter().enumerate() {
            let checkbox = if cat.selected { "[x]" } else { "[ ]" };
            let is_cursor = cur_section == SelectSection::Categories && cur_local == i;
            let style = if is_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if cat.selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            let cursor = if is_cursor { ">" } else { " " };

            let exp_days = app.settings.expiry_days;
            let exp_size = cat.expired_size(exp_days);
            let exp_count = cat.expired_count(exp_days);
            let has_expiry = !cat.file_ages.is_empty();

            let detail = if has_expiry && exp_count < cat.file_count {
                // Show expired subset vs total
                format!(
                    "{:>10}  {:>5}/{} files (>{} days)",
                    widgets::format_size(exp_size),
                    exp_count,
                    cat.file_count,
                    exp_days,
                )
            } else {
                format!(
                    "{:>10}  {:>5} files",
                    widgets::format_size(exp_size),
                    exp_count
                )
            };

            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{} {} ", cursor, checkbox), style),
                Span::styled(format!("{:<18}", cat.category.to_string()), style),
                Span::styled(detail, Style::default().fg(Color::DarkGray)),
            ])));
        }

        // ── Separator: ~/.claude.json cleanup ──
        items.push(ListItem::new(Line::from(Span::styled(
            "  ── ~/.claude.json Cleanup ────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        ))));

        // ── Section 2: ConfigJson cleanup items ──
        let cj = &result.config_json;
        let cj_items: [(bool, &str, &str, usize, u64); 3] = [
            (
                cj.orphan_projects_selected,
                "Orphan projects in JSON",
                "Remove entries for deleted project paths",
                cj.orphan_projects_count,
                cj.orphan_projects_size,
            ),
            (
                cj.metrics_selected,
                "Session metrics",
                "Strip lastModel/Cost/Duration/FPS/Lines data",
                cj.metrics_entries_count,
                cj.metrics_size,
            ),
            (
                cj.cache_selected,
                "Cached flags & data",
                "Remove statsig/growth/telemetry caches",
                cj.cache_keys_count,
                cj.cache_size,
            ),
        ];

        for (i, (selected, label, desc, count, size)) in cj_items.iter().enumerate() {
            let is_cursor = cur_section == SelectSection::ConfigJson && cur_local == i;
            let checkbox = if *selected { "[x]" } else { "[ ]" };
            let cursor = if is_cursor { ">" } else { " " };

            let style = if is_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if *selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{} {} ", cursor, checkbox), style),
                Span::styled(format!("{:<18}", label), style),
                Span::styled(
                    format!("{:>10}  {:>5} items", widgets::format_size(*size), count),
                    Style::default().fg(Color::DarkGray),
                ),
            ])));

            // Show description on a second line if cursor is on this item
            if is_cursor {
                items.push(ListItem::new(Line::from(vec![
                    Span::raw("        "),
                    Span::styled(format!("↳ {}", desc), Style::default().fg(Color::DarkGray)),
                ])));
            }
        }

        // ── Separator: Settings ──
        items.push(ListItem::new(Line::from(Span::styled(
            "  ── Settings ──────────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        ))));

        // ── Section 3: Settings ──
        for i in 0..CleanSettings::FIELD_COUNT {
            let is_cursor = cur_section == SelectSection::Settings && cur_local == i;
            let name = CleanSettings::field_name(i);
            let value = app.settings.field_value(i);
            let cursor = if is_cursor { ">" } else { " " };

            let style = if is_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let value_style = if is_cursor {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Yellow)
            };
            let arrows = if is_cursor {
                " \u{25c0} \u{25b6} "
            } else {
                "     "
            };

            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", cursor), style),
                Span::styled(format!("{:<25}", name), style),
                Span::styled(arrows, Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:>8}", value), value_style),
            ])));
        }

        let list = List::new(items).block(Block::default());
        f.render_widget(list, chunks[0]);

        // ── Summary ──
        let exp_days = app.settings.expiry_days;
        let selected_size: u64 = result
            .categories
            .iter()
            .filter(|c| c.selected)
            .map(|c| c.expired_size(exp_days))
            .sum();
        let selected_count = result.categories.iter().filter(|c| c.selected).count();
        let cj_size = cj.reclaimable_size();

        let summary = Paragraph::new(vec![Line::from(vec![
            Span::raw("  Selected: "),
            Span::styled(
                format!(
                    "{} categories + ~{} JSON cleanup",
                    selected_count,
                    widgets::format_size(cj_size),
                ),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  (~{})", widgets::format_size(selected_size + cj_size)),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw("    "),
            Span::styled("[a]", Style::default().fg(Color::Yellow)),
            Span::raw("ll  "),
            Span::styled("[n]", Style::default().fg(Color::Yellow)),
            Span::raw("one  "),
            Span::styled("[d]", Style::default().fg(Color::Yellow)),
            Span::raw("efault"),
        ])]);
        f.render_widget(summary, chunks[1]);
    } else {
        let p = Paragraph::new("No scan data. Press [S] on Dashboard to scan first.")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, chunks[0]);
    }
}
