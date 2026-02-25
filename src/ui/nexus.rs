use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::state::nexus::NexusState;

pub fn render(f: &mut Frame, state: &mut NexusState, search_query: &str, area: Rect) {
    let filtered = state.filtered_connections(search_query);

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(_, c)| {
            let proto_color = match c.protocol.as_str() {
                "TCP" => Color::Green,
                "UDP" => Color::Yellow,
                _ => Color::White,
            };
            ListItem::new(format!(
                "{:6} {:5} {:22} {:22} {:12} {}",
                c.pid,
                c.protocol,
                format!("{}:{}", c.local_addr, c.local_port),
                format!("{}:{}", c.remote_addr, c.remote_port),
                c.state,
                c.process_name.as_deref().unwrap_or("-")
            ))
            .style(Style::default().fg(proto_color))
        })
        .collect();

    // Build title with filter and sort info
    let total = state.connections.len();
    let showing = filtered.len();
    let sort_info = format!("{} {}", state.sort_key.as_str(), state.sort_order.as_str());
    let title = format!(" Network (Nexus) [{}/{} | {}] ", showing, total, sort_info);

    // Create inner area inside the border for the header
    let inner_area = area.inner(Margin::new(1, 1));

    // Split inner area into header (1 line) and list (remaining space)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner_area);

    // Render header as non-selectable text in the first line of inner area
    let header_text = format!(
        "{:6} {:5} {:22} {:22} {:12} {}",
        "PID", "Proto", "Local", "Remote", "State", "Process"
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
