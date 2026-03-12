use crate::app::App;
use crate::ui::widgets;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
};
use ratatui::Frame;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Filter bar
            Constraint::Min(0),    // Table
            Constraint::Length(2), // Summary
        ])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Projects ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(block, area);

    // Filter bar (render inside the border, offset by 1)
    let filter_area = Rect {
        x: chunks[0].x + 1,
        y: chunks[0].y + 1,
        width: chunks[0].width.saturating_sub(2),
        height: chunks[0].height,
    };
    let filter_line = if app.project_filtering {
        Line::from(vec![
            Span::styled(" /", Style::default().fg(Color::Yellow)),
            Span::raw(&app.project_filter),
            Span::styled(
                "_",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ])
    } else if !app.project_filter.is_empty() {
        Line::from(vec![
            Span::styled(" Filter: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.project_filter, Style::default().fg(Color::Yellow)),
            Span::styled(
                "  (/ to edit, Esc to clear)",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        Line::from(Span::styled(
            " [Space] Toggle  [a] Select All  [o] Orphans Only  [n] Unselect All  [/] Search",
            Style::default().fg(Color::DarkGray),
        ))
    };
    f.render_widget(Paragraph::new(filter_line), filter_area);

    // Table area inside border
    let table_area = Rect {
        x: chunks[1].x + 1,
        y: chunks[1].y,
        width: chunks[1].width.saturating_sub(3), // leave room for scrollbar
        height: chunks[1].height,
    };

    // Scrollbar area
    let scrollbar_area = Rect {
        x: chunks[1].x + chunks[1].width.saturating_sub(2),
        y: chunks[1].y,
        width: 1,
        height: chunks[1].height,
    };

    // Summary area inside border
    let summary_area = Rect {
        x: chunks[2].x + 1,
        y: chunks[2].y,
        width: chunks[2].width.saturating_sub(2),
        height: chunks[2].height,
    };

    if let Some(ref result) = app.scan_result {
        let filtered = app.filtered_projects();
        let total_items = filtered.len();
        let expiry_days = app.settings.expiry_days;

        // Calculate visible rows: table_area height minus 2 for header + margin
        let header_rows: u16 = 2; // header row + bottom_margin
        let visible_rows = table_area.height.saturating_sub(header_rows) as usize;

        let header = Row::new(vec![
            "",
            "Project Path",
            "Status",
            "Reclaimable",
            "Last Modified",
        ])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

        // Only render the visible window of rows
        let scroll = app.project_scroll;
        let end = (scroll + visible_rows).min(total_items);

        let rows: Vec<Row> = filtered[scroll..end]
            .iter()
            .enumerate()
            .map(|(window_idx, &proj_idx)| {
                let display_idx = scroll + window_idx;
                let proj = &result.projects[proj_idx];
                let is_cursor = display_idx == app.project_cursor;

                let checkbox = if proj.selected { "[x]" } else { "[ ]" };
                let cursor_str = if is_cursor { ">" } else { " " };

                let age = proj
                    .last_modified
                    .as_ref()
                    .map(widgets::format_age)
                    .unwrap_or_else(|| "-".into());

                let path_display = proj.original_path.to_string_lossy().to_string();
                let max_path_len = table_area.width.saturating_sub(45) as usize;
                let path_short = if path_display.len() > max_path_len && max_path_len > 4 {
                    format!(
                        "...{}",
                        &path_display[path_display.len() - (max_path_len - 3)..]
                    )
                } else {
                    path_display
                };

                let (status, _status_color) = if proj.is_orphan {
                    ("ORPHAN", Color::Red)
                } else {
                    ("active", Color::Green)
                };

                // For orphans: show full size (will delete all)
                // For active: show expired file size
                let reclaimable = proj.expired_size(expiry_days);
                let reclaimable_str = if proj.is_orphan {
                    format!("{} (all)", widgets::format_size(reclaimable))
                } else if reclaimable > 0 {
                    let expired_count = proj.expired_count(expiry_days);
                    format!("{} ({}f)", widgets::format_size(reclaimable), expired_count)
                } else {
                    "-".into()
                };

                let row_style = if is_cursor {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if proj.is_orphan {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };

                Row::new(vec![
                    format!("{} {}", cursor_str, checkbox),
                    path_short,
                    format!("{}", status),
                    reclaimable_str,
                    age,
                ])
                .style(row_style)
            })
            .collect();

        let table = Table::new(
            rows,
            &[
                Constraint::Length(6),
                Constraint::Min(20),
                Constraint::Length(8),
                Constraint::Length(16),
                Constraint::Length(12),
            ],
        )
        .header(header)
        .block(Block::default());

        f.render_widget(table, table_area);

        // Scrollbar
        if total_items > visible_rows {
            let mut scrollbar_state =
                ScrollbarState::new(total_items.saturating_sub(visible_rows)).position(scroll);
            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("\u{2191}")) // ↑
                    .end_symbol(Some("\u{2193}")) // ↓
                    .track_symbol(Some("\u{2502}")) // │
                    .thumb_symbol("\u{2588}"), // █
                scrollbar_area,
                &mut scrollbar_state,
            );
        }

        // Summary
        let selected_count = result.projects.iter().filter(|p| p.selected).count();
        let selected_size: u64 = result
            .projects
            .iter()
            .filter(|p| p.selected)
            .map(|p| p.expired_size(expiry_days))
            .sum();
        let orphan_count = result.projects.iter().filter(|p| p.is_orphan).count();
        let active_count = result.projects.len() - orphan_count;

        let pos_info = if total_items > 0 {
            format!(" [{}/{}]", app.project_cursor + 1, total_items)
        } else {
            String::new()
        };

        let summary = Paragraph::new(Line::from(vec![Span::raw(format!(
            "  {} projects ({} orphan, {} active) | Selected: {} ({}){}",
            result.projects.len(),
            orphan_count,
            active_count,
            selected_count,
            widgets::format_size(selected_size),
            pos_info,
        ))]))
        .style(Style::default().fg(Color::DarkGray));
        f.render_widget(summary, summary_area);
    } else {
        let p = Paragraph::new("No scan data.").style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, table_area);
    }
}
