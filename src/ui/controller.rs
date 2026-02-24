use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
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
    let title = if showing != total {
        format!(
            " Services (Controller) [{}/{} | {}] ",
            showing, total, sort_info
        )
    } else {
        format!(" Services (Controller) [{}] ", sort_info)
    };

    let list = List::new(items)
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
