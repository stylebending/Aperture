use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::state::env::EnvState;

pub fn render(f: &mut Frame, state: &mut EnvState, _search_query: &str, area: Rect) {
    let filtered = state.filtered_entries();

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(_, e)| {
            let scope_color = match e.scope {
                crate::state::env::EnvScope::System => Color::Cyan,
                crate::state::env::EnvScope::User => Color::Green,
                crate::state::env::EnvScope::Process => Color::Yellow,
            };
            let style = if e.overridden {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(scope_color)
            };
            ListItem::new(format!(
                "{:35} {:70} {:8}",
                e.name,
                e.value,
                e.scope.as_str()
            ))
            .style(style)
        })
        .collect();

    let total = state.entries.len();
    let showing = filtered.len();
    let sort_info = format!("{} {}", state.sort_key.as_str(), state.sort_order.as_str());
    let title = format!(" Environment [{}/{} | {}] ", showing, total, sort_info);

    let inner_area = area.inner(Margin::new(1, 1));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner_area);

    let header_text = format!(
        "{:35} {:70} {:8}",
        "Name", "Value", "Scope"
    );
    let header = Paragraph::new(Line::from(vec![Span::styled(
        header_text,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]));
    f.render_widget(header, chunks[0]);

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan));
    f.render_widget(list_block.clone(), area);

    let list = List::new(items).highlight_style(Style::default().bg(Color::DarkGray));
    f.render_stateful_widget(list, chunks[1], &mut state.list_state);
}
