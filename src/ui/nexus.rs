use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
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

    let header = ListItem::new(format!(
        "{:6} {:5} {:22} {:22} {:12} {}",
        "PID", "Proto", "Local", "Remote", "State", "Process"
    ))
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    let mut all_items: Vec<ListItem> = vec![header];
    all_items.extend(items);

    // Build title with filter and sort info
    let total = state.connections.len();
    let showing = filtered.len();
    let sort_info = format!("{} {}", state.sort_key.as_str(), state.sort_order.as_str());
    let title = format!(" Network (Nexus) [{}/{} | {}] ", showing, total, sort_info);

    let list = List::new(all_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    // Pass mutable reference directly (not cloned) so selection is preserved
    f.render_stateful_widget(list, area, &mut state.list_state);
}
