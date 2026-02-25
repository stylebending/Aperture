use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::state::controller::ControllerState;

pub fn render(f: &mut Frame, state: &mut ControllerState, search_query: &str, area: Rect) {
    let filtered = state.filtered_services(search_query);

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(_, s)| {
            let status_color = match s.status.as_str() {
                "Running" => Color::Green,
                "Stopped" => Color::Red,
                _ => Color::Yellow,
            };
            ListItem::new(format!(
                "{:40} {:10} {:12} {}",
                s.display_name, s.status, s.start_type, s.service_type
            ))
            .style(Style::default().fg(status_color))
        })
        .collect();

    // Build title with filter and sort info
    let total = state.services.len();
    let showing = filtered.len();
    let sort_info = format!("{} {}", state.sort_key.as_str(), state.sort_order.as_str());
    let title = format!(
        " Services (Controller) [{}/{} | {}] ",
        showing, total, sort_info
    );

    // Create inner area inside the border for the header
    let inner_area = area.inner(Margin::new(1, 1));

    // Split inner area into header (1 line) and list (remaining space)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner_area);

    // Render header as non-selectable text in the first line of inner area
    let header_text = format!(
        "{:40} {:10} {:12} {}",
        "Name", "Status", "Start Type", "Type"
    );
    let header = Paragraph::new(Line::from(vec![Span::styled(
        header_text,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]));
    f.render_widget(header, chunks[0]);

    // Render list block with border (full area)
    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan));
    f.render_widget(list_block.clone(), area);

    // Render list items in the remaining space (below header, inside border)
    let list = List::new(items).highlight_style(Style::default().bg(Color::DarkGray));

    // Pass mutable reference directly (not cloned) so selection is preserved
    f.render_stateful_widget(list, chunks[1], &mut state.list_state);
}
