use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;

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
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
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
            let is_selected = app.selection.is_selected(&entry.path);
            let checkbox = if is_selected { "[x]" } else { "[ ]" };
            let name = if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            };

            let cursor = if i == app.browser.cursor { "> " } else { "  " };

            let style = if i == app.browser.cursor {
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

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Files"));

    frame.render_widget(list, area);
}

fn render_selection_list(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!("Selected ({})", app.selection.count());

    let mut paths: Vec<String> = app
        .selection
        .iter()
        .map(|p| {
            p.strip_prefix(&app.base_dir)
                .map(|rel| format!("./{}", rel.display()))
                .unwrap_or_else(|_| p.display().to_string())
        })
        .collect();
    paths.sort();

    let items: Vec<ListItem> = paths
        .into_iter()
        .map(|p| ListItem::new(format!(" {}", p)))
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));

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
        Span::styled(" j/k ", key_style),
        Span::styled(" up/down ", desc_style),
        Span::styled("│", sep_style),
        Span::styled(" h/l ", key_style),
        Span::styled(" nav ", desc_style),
        Span::styled("│", sep_style),
        Span::styled(" Space ", key_style),
        Span::styled(" select ", desc_style),
        Span::styled("│", sep_style),
        Span::styled(" Enter ", key_style),
        Span::styled(" confirm ", desc_style),
        Span::styled("│", sep_style),
        Span::styled(" . ", key_style),
        Span::styled(" hidden ", desc_style),
        Span::styled("│", sep_style),
        Span::styled(" q ", key_style),
        Span::styled(" quit ", desc_style),
    ]);

    let paragraph = Paragraph::new(legend);
    frame.render_widget(paragraph, area);
}
