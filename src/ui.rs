use std::path::Path;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, FocusedPane};

/// Style constants
mod styles {
    use super::*;

    pub const CURSOR: &str = "> ";
    pub const NO_CURSOR: &str = "  ";
    pub const CHECKED: &str = "[x] ";
    pub const UNCHECKED: &str = "[ ] ";

    pub fn focused_border() -> Style {
        Style::default().fg(Color::Cyan)
    }

    pub fn unfocused_border() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    pub fn cursor_style() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    pub fn invalid_style() -> Style {
        Style::default().fg(Color::Red)
    }

    pub fn invalid_cursor_style() -> Style {
        Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD)
    }

    pub fn directory_style() -> Style {
        Style::default().fg(Color::Blue)
    }

    pub fn normal_style() -> Style {
        Style::default()
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let [status_area, main_area, legend_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(frame.area());

    render_status_bar(frame, app, status_area);
    render_main_panels(frame, app, main_area);
    render_legend(frame, app, legend_area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    if app.search_mode {
        let search_text = format!("/{}", app.search_query);
        let status = Paragraph::new(search_text).style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(status, area);
        return;
    }

    let current_dir = app
        .browser
        .current_dir
        .strip_prefix(&app.base_dir)
        .map(|p| format!("./{}", p.display()))
        .unwrap_or_else(|_| app.browser.current_dir.display().to_string());

    let hidden_indicator = if app.browser.show_hidden { "[H]" } else { "[ ]" };
    let status_text = format!(" {}  {}", current_dir, hidden_indicator);

    let status = Paragraph::new(status_text).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(status, area);
}

fn render_main_panels(frame: &mut Frame, app: &mut App, area: Rect) {
    let [files_area, selected_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .areas(area);

    render_file_list(frame, app, files_area);
    render_selection_list(frame, app, selected_area);
}

fn render_file_list(frame: &mut Frame, app: &mut App, area: Rect) {
    // Calculate visible height (area minus borders)
    let visible_height = area.height.saturating_sub(2) as usize;
    app.browser.adjust_scroll(visible_height);

    let items: Vec<ListItem> = app
        .browser
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_cursor = i == app.browser.cursor;
            let is_selected = if entry.is_invalid {
                app.selection.is_invalid_selected(&entry.path)
            } else {
                app.selection.is_selected(&entry.path)
            };

            let name = format_entry_name(entry, app);
            let cursor = if is_cursor { styles::CURSOR } else { styles::NO_CURSOR };
            let checkbox = if is_selected { styles::CHECKED } else { styles::UNCHECKED };

            let style = entry_style(entry.is_invalid, entry.is_dir, is_cursor);

            ListItem::new(Line::from(vec![
                Span::styled(cursor, style),
                Span::styled(checkbox, style),
                Span::styled(name, style),
            ]))
        })
        .collect();

    let is_focused = app.focused_pane == FocusedPane::Files;
    let border_style = if is_focused {
        styles::focused_border()
    } else {
        styles::unfocused_border()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Files")
            .border_style(border_style),
    );

    let mut state = ListState::default()
        .with_selected(Some(app.browser.cursor))
        .with_offset(app.browser.scroll_offset);
    frame.render_stateful_widget(list, area, &mut state);
}

fn format_entry_name(entry: &crate::file_browser::FileEntry, app: &App) -> String {
    if entry.is_dir {
        let count = count_selected_in_dir(&entry.path, app);
        if count > 0 {
            format!("{}/ ({})", entry.name, count)
        } else {
            format!("{}/", entry.name)
        }
    } else {
        entry.name.clone()
    }
}

fn entry_style(is_invalid: bool, is_dir: bool, is_cursor: bool) -> Style {
    match (is_invalid, is_cursor) {
        (true, true) => styles::invalid_cursor_style(),
        (true, false) => styles::invalid_style(),
        (false, true) => styles::cursor_style(),
        (false, false) if is_dir => styles::directory_style(),
        (false, false) => styles::normal_style(),
    }
}

fn count_selected_in_dir(dir_path: &Path, app: &App) -> usize {
    let Ok(dir_canonical) = dir_path.canonicalize() else {
        return 0;
    };

    let valid_count = app
        .selection
        .iter_valid()
        .filter(|p| p.starts_with(&dir_canonical))
        .count();

    let invalid_count = app
        .selection
        .iter_invalid()
        .filter(|p| {
            let full_path = if p.is_absolute() {
                p.to_path_buf()
            } else {
                app.base_dir.join(p)
            };
            full_path.starts_with(&dir_canonical)
        })
        .count();

    valid_count + invalid_count
}

fn render_selection_list(frame: &mut Frame, app: &mut App, area: Rect) {
    // Calculate visible height and adjust scroll
    let visible_height = area.height.saturating_sub(2) as usize;
    app.adjust_selected_scroll(visible_height);

    let title = format!("Selected ({})", app.selection.count());
    let is_focused = app.focused_pane == FocusedPane::Selected;

    let all_paths = collect_display_paths(app);

    let items: Vec<ListItem> = all_paths
        .into_iter()
        .enumerate()
        .map(|(i, (display, is_valid))| {
            let is_cursor = is_focused && i == app.selected_cursor;
            let cursor = if is_cursor { styles::CURSOR } else { styles::NO_CURSOR };

            let style = match (is_valid, is_cursor) {
                (_, true) if !is_valid => styles::invalid_cursor_style(),
                (_, true) => styles::cursor_style(),
                (false, false) => styles::invalid_style(),
                (true, false) => styles::normal_style(),
            };

            ListItem::new(Line::from(Span::styled(format!("{}{}", cursor, display), style)))
        })
        .collect();

    let border_style = if is_focused {
        styles::focused_border()
    } else {
        styles::unfocused_border()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style),
    );

    let selected = if is_focused { Some(app.selected_cursor) } else { None };
    let mut state = ListState::default()
        .with_selected(selected)
        .with_offset(app.selected_scroll_offset);
    frame.render_stateful_widget(list, area, &mut state);
}

fn collect_display_paths(app: &App) -> Vec<(String, bool)> {
    let mut paths: Vec<(String, bool)> = app
        .selection
        .iter_valid()
        .map(|p| (app.format_path_for_display(p, true), true))
        .chain(
            app.selection
                .iter_invalid()
                .map(|p| (app.format_path_for_display(p, false), false)),
        )
        .collect();

    paths.sort_by(|a, b| a.0.cmp(&b.0));
    paths
}

fn render_legend(frame: &mut Frame, app: &App, area: Rect) {
    let key_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Gray)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::Gray);
    let sep_style = Style::default().fg(Color::DarkGray);

    let mut bindings = vec![
        ("Tab", "pane"),
        ("Space", "sel"),
        ("a", "all"),
        ("r", "rec"),
        ("/", "search"),
    ];

    if app.can_save() {
        bindings.push(("s", "save"));
    }

    bindings.push(("Enter", "ok"));
    bindings.push(("q", "quit"));

    let mut spans = Vec::new();
    for (i, (key, desc)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("│", sep_style));
        }
        spans.push(Span::styled(format!(" {} ", key), key_style));
        spans.push(Span::styled(format!(" {} ", desc), desc_style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
