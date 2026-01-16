use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, FocusedPane};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_status_bar(frame, app, chunks[0]);
    render_main_panels(frame, app, chunks[1]);
    render_legend(frame, chunks[2]);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
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

fn render_main_panels(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_file_list(frame, app, chunks[0]);
    render_selection_list(frame, app, chunks[1]);
}

fn render_file_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .browser
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = if entry.is_invalid {
                app.selection.is_invalid_selected(&entry.path)
            } else {
                app.selection.is_selected(&entry.path)
            };
            let checkbox = if is_selected { "[x]" } else { "[ ]" };

            // For directories, count selected files inside
            let name = if entry.is_dir {
                let count = count_selected_in_dir(&entry.path, app);
                if count > 0 {
                    format!("{}/ ({})", entry.name, count)
                } else {
                    format!("{}/", entry.name)
                }
            } else {
                entry.name.clone()
            };

            let cursor = if i == app.browser.cursor { "> " } else { "  " };

            let style = if entry.is_invalid {
                // Invalid files are always red
                if i == app.browser.cursor {
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Red)
                }
            } else if i == app.browser.cursor {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_dir {
                Style::default().fg(Color::Blue)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(cursor, style),
                Span::styled(format!("{} ", checkbox), style),
                Span::styled(name, style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let border_style = if app.focused_pane == FocusedPane::Files {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Files")
            .border_style(border_style),
    );

    frame.render_widget(list, area);
}

/// Count how many selected files are inside a directory (including subdirectories)
fn count_selected_in_dir(dir_path: &std::path::PathBuf, app: &App) -> usize {
    let dir_canonical = match dir_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return 0,
    };

    // Count valid selected paths that start with this directory
    let valid_count = app
        .selection
        .iter_valid()
        .filter(|p| p.starts_with(&dir_canonical))
        .count();

    // Count invalid selected paths that would be in this directory
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

fn render_selection_list(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!("Selected ({})", app.selection.count());

    // Collect valid paths
    let valid_paths: Vec<(String, bool)> = app
        .selection
        .iter_valid()
        .map(|p| {
            let display = p
                .strip_prefix(&app.base_dir)
                .map(|rel| format!("./{}", rel.display()))
                .unwrap_or_else(|_| p.display().to_string());
            (display, true) // true = valid
        })
        .collect();

    // Collect invalid paths
    let invalid_paths: Vec<(String, bool)> = app
        .selection
        .iter_invalid()
        .map(|p| {
            let s = p.to_string_lossy();
            let display = if s.starts_with("./") || s.starts_with('/') {
                s.to_string()
            } else {
                format!("./{}", s)
            };
            (display, false) // false = invalid
        })
        .collect();

    // Combine and sort
    let mut all_paths: Vec<(String, bool)> = valid_paths;
    all_paths.extend(invalid_paths);
    all_paths.sort_by(|a, b| a.0.cmp(&b.0));

    let is_focused = app.focused_pane == FocusedPane::Selected;

    let items: Vec<ListItem> = all_paths
        .into_iter()
        .enumerate()
        .map(|(i, (p, is_valid))| {
            let is_cursor = is_focused && i == app.selected_cursor;
            let cursor = if is_cursor { "> " } else { "  " };

            let style = if is_cursor {
                if is_valid {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD)
                }
            } else if is_valid {
                Style::default()
            } else {
                Style::default().fg(Color::Red)
            };

            ListItem::new(Line::from(Span::styled(format!("{}{}", cursor, p), style)))
        })
        .collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style),
    );

    frame.render_widget(list, area);
}

fn render_legend(frame: &mut Frame, area: Rect) {
    let key_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Gray)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::Gray);
    let sep_style = Style::default().fg(Color::DarkGray);

    let legend = Line::from(vec![
        Span::styled(" Tab ", key_style),
        Span::styled(" pane ", desc_style),
        Span::styled("│", sep_style),
        Span::styled(" Space ", key_style),
        Span::styled(" sel ", desc_style),
        Span::styled("│", sep_style),
        Span::styled(" a ", key_style),
        Span::styled(" all ", desc_style),
        Span::styled("│", sep_style),
        Span::styled(" r ", key_style),
        Span::styled(" rec ", desc_style),
        Span::styled("│", sep_style),
        Span::styled(" Enter ", key_style),
        Span::styled(" ok ", desc_style),
        Span::styled("│", sep_style),
        Span::styled(" q ", key_style),
        Span::styled(" quit ", desc_style),
    ]);

    let paragraph = Paragraph::new(legend);
    frame.render_widget(paragraph, area);
}
