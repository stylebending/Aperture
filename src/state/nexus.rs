use std::time::{Duration, Instant};

use ratatui::widgets::ListState;

use crate::sys::network::ConnectionInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    State,
    Pid,
    Protocol,
    ProcessName,
}

impl SortKey {
    pub fn next(&self) -> Self {
        match self {
            SortKey::State => SortKey::Pid,
            SortKey::Pid => SortKey::Protocol,
            SortKey::Protocol => SortKey::ProcessName,
            SortKey::ProcessName => SortKey::State,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SortKey::State => "State",
            SortKey::Pid => "PID",
            SortKey::Protocol => "Proto",
            SortKey::ProcessName => "Process",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    pub fn toggle(&self) -> Self {
        match self {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SortOrder::Ascending => "▲",
            SortOrder::Descending => "▼",
        }
    }
}

fn state_priority(state: &str) -> u8 {
    match state {
        "ESTABLISHED" => 0,
        "LISTENING" => 1,
        "SYN_SENT" => 2,
        "SYN_RCVD" => 3,
        "FIN_WAIT1" => 4,
        "FIN_WAIT2" => 5,
        "CLOSE_WAIT" => 6,
        "CLOSING" => 7,
        "LAST_ACK" => 8,
        "TIME_WAIT" => 9,
        "CLOSED" => 10,
        "DELETE_TCB" => 11,
        "N/A" => 12,
        _ => 13,
    }
}

pub struct NexusState {
    pub connections: Vec<ConnectionInfo>,
    pub list_state: ListState,
    pub active_filter: Option<String>,
    pub selected_connection_key: Option<(u32, String, u16, String, u16)>,
    pub last_navigation: Instant,
    pub sort_key: SortKey,
    pub sort_order: SortOrder,
    last_data_hash: u64,
    is_initial_load: bool,
}

impl NexusState {
    const NAVIGATION_DEBOUNCE: Duration = Duration::from_millis(50);

    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            list_state: ListState::default(),
            active_filter: None,
            selected_connection_key: None,
            last_navigation: Instant::now(),
            sort_key: SortKey::State,
            sort_order: SortOrder::Ascending,
            last_data_hash: 0,
            is_initial_load: true,
        }
    }

    fn compute_data_hash(&self, connections: &[ConnectionInfo]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        connections.len().hash(&mut hasher);
        for c in connections {
            c.pid.hash(&mut hasher);
            c.local_addr.hash(&mut hasher);
            c.local_port.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub fn should_ignore_update(&self) -> bool {
        if self.is_initial_load {
            return false;
        }
        self.last_navigation.elapsed() < Self::NAVIGATION_DEBOUNCE
    }

    fn mark_navigation(&mut self) {
        self.last_navigation = Instant::now();
    }

    pub fn set_filter(&mut self, query: String) {
        // Filter changes are instant - no debounce
        if query.is_empty() {
            self.active_filter = None;
        } else {
            self.active_filter = Some(query.to_lowercase());
        }

        self.update_selection_from_key();
    }

    pub fn clear_filter(&mut self) {
        // Filter changes are instant - no debounce
        self.active_filter = None;
        self.update_selection_from_key();
    }

    pub fn cycle_sort_key(&mut self) {
        self.sort_key = self.sort_key.next();
        self.sort_connections();
        self.update_selection_from_key();
    }

    pub fn toggle_sort_order(&mut self) {
        self.sort_order = self.sort_order.toggle();
        self.sort_connections();
        self.update_selection_from_key();
    }

    fn sort_connections(&mut self) {
        match self.sort_key {
            SortKey::State => {
                self.connections.sort_by(|a, b| {
                    let a_priority = state_priority(&a.state);
                    let b_priority = state_priority(&b.state);
                    let cmp = a_priority.cmp(&b_priority);
                    if self.sort_order == SortOrder::Descending {
                        cmp.reverse()
                    } else {
                        cmp
                    }
                });
            }
            SortKey::Pid => {
                self.connections.sort_by(|a, b| {
                    let cmp = a.pid.cmp(&b.pid);
                    if self.sort_order == SortOrder::Descending {
                        cmp.reverse()
                    } else {
                        cmp
                    }
                });
            }
            SortKey::Protocol => {
                self.connections.sort_by(|a, b| {
                    let cmp = a.protocol.cmp(&b.protocol);
                    if self.sort_order == SortOrder::Descending {
                        cmp.reverse()
                    } else {
                        cmp
                    }
                });
            }
            SortKey::ProcessName => {
                self.connections.sort_by(|a, b| {
                    let a_name = a.process_name.as_deref().unwrap_or("");
                    let b_name = b.process_name.as_deref().unwrap_or("");
                    let cmp = a_name.cmp(b_name);
                    if self.sort_order == SortOrder::Descending {
                        cmp.reverse()
                    } else {
                        cmp
                    }
                });
            }
        }
    }

    fn update_selection_from_key(&mut self) {
        if let Some((pid, ref local_addr, local_port, ref remote_addr, remote_port)) =
            self.selected_connection_key
        {
            let filtered = self.get_filtered_indices("");
            if let Some(new_idx) = filtered.iter().position(|&i| {
                self.connections
                    .get(i)
                    .map(|c| {
                        c.pid == pid
                            && c.local_addr == *local_addr
                            && c.local_port == local_port
                            && c.remote_addr == *remote_addr
                            && c.remote_port == remote_port
                    })
                    .unwrap_or(false)
            }) {
                self.list_state.select(Some(new_idx));
            } else if !filtered.is_empty() {
                self.list_state.select(Some(0));
                self.selected_connection_key = filtered.first().and_then(|&i| {
                    self.connections.get(i).map(|c| {
                        (
                            c.pid,
                            c.local_addr.clone(),
                            c.local_port,
                            c.remote_addr.clone(),
                            c.remote_port,
                        )
                    })
                });
            } else {
                self.list_state.select(None);
                self.selected_connection_key = None;
            }
        } else if !self.connections.is_empty() {
            self.list_state.select(Some(0));
            self.selected_connection_key = self.connections.first().map(|c| {
                (
                    c.pid,
                    c.local_addr.clone(),
                    c.local_port,
                    c.remote_addr.clone(),
                    c.remote_port,
                )
            });
        }
    }

    fn get_filter(&self, search_query: &str) -> Option<String> {
        if !search_query.is_empty() {
            Some(search_query.to_lowercase())
        } else {
            self.active_filter.clone()
        }
    }

    fn matches_filter(&self, conn: &ConnectionInfo, query: &str) -> bool {
        conn.process_name
            .as_ref()
            .map(|n| n.to_lowercase().contains(query))
            .unwrap_or(false)
            || conn.local_addr.to_lowercase().contains(query)
            || conn.remote_addr.to_lowercase().contains(query)
            || conn.pid.to_string().contains(query)
            || conn.local_port.to_string().contains(query)
    }

    pub fn get_filtered_indices(&self, search_query: &str) -> Vec<usize> {
        match self.get_filter(search_query) {
            None => (0..self.connections.len()).collect(),
            Some(query) => self
                .connections
                .iter()
                .enumerate()
                .filter(|(_, c)| self.matches_filter(c, &query))
                .map(|(i, _)| i)
                .collect(),
        }
    }

    pub fn filtered_connections(&self, search_query: &str) -> Vec<(usize, ConnectionInfo)> {
        match self.get_filter(search_query) {
            None => self
                .connections
                .iter()
                .enumerate()
                .map(|(i, c)| (i, c.clone()))
                .collect(),
            Some(query) => self
                .connections
                .iter()
                .enumerate()
                .filter(|(_, c)| self.matches_filter(c, &query))
                .map(|(i, c)| (i, c.clone()))
                .collect(),
        }
    }

    pub fn update_connections(&mut self, connections: Vec<ConnectionInfo>) {
        // Check if data actually changed
        let new_hash = self.compute_data_hash(&connections);

        if new_hash == self.last_data_hash {
            // Data hasn't changed, skip update
            return;
        }
        self.last_data_hash = new_hash;

        // Don't update during active navigation (but always allow initial load)
        if self.should_ignore_update() {
            return;
        }

        self.connections = connections;
        self.sort_connections();
        self.update_selection_from_key();

        // Mark initial load as complete
        self.is_initial_load = false;
    }

    pub fn select_next(&mut self, search_query: &str) {
        self.mark_navigation();
        let filtered = self.get_filtered_indices(search_query);
        if filtered.is_empty() {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        let new_idx = (i + 1) % filtered.len();
        self.list_state.select(Some(new_idx));
        self.selected_connection_key = filtered.get(new_idx).and_then(|&idx| {
            self.connections.get(idx).map(|c| {
                (
                    c.pid,
                    c.local_addr.clone(),
                    c.local_port,
                    c.remote_addr.clone(),
                    c.remote_port,
                )
            })
        });
    }

    pub fn select_prev(&mut self, search_query: &str) {
        self.mark_navigation();
        let filtered = self.get_filtered_indices(search_query);
        if filtered.is_empty() {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        let new_idx = (i + filtered.len() - 1) % filtered.len();
        self.list_state.select(Some(new_idx));
        self.selected_connection_key = filtered.get(new_idx).and_then(|&idx| {
            self.connections.get(idx).map(|c| {
                (
                    c.pid,
                    c.local_addr.clone(),
                    c.local_port,
                    c.remote_addr.clone(),
                    c.remote_port,
                )
            })
        });
    }

    pub fn select_page_up(&mut self, search_query: &str) {
        self.mark_navigation();
        let filtered = self.get_filtered_indices(search_query);
        if filtered.is_empty() {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        let page_size = 10;
        let new_idx = i.saturating_sub(page_size);
        self.list_state.select(Some(new_idx));
        self.selected_connection_key = filtered.get(new_idx).and_then(|&idx| {
            self.connections.get(idx).map(|c| {
                (
                    c.pid,
                    c.local_addr.clone(),
                    c.local_port,
                    c.remote_addr.clone(),
                    c.remote_port,
                )
            })
        });
    }

    pub fn select_page_down(&mut self, search_query: &str) {
        self.mark_navigation();
        let filtered = self.get_filtered_indices(search_query);
        if filtered.is_empty() {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        let page_size = 10;
        let new_idx = std::cmp::min(i + page_size, filtered.len().saturating_sub(1));
        self.list_state.select(Some(new_idx));
        self.selected_connection_key = filtered.get(new_idx).and_then(|&idx| {
            self.connections.get(idx).map(|c| {
                (
                    c.pid,
                    c.local_addr.clone(),
                    c.local_port,
                    c.remote_addr.clone(),
                    c.remote_port,
                )
            })
        });
    }

    pub fn select_first(&mut self, search_query: &str) {
        self.mark_navigation();
        let filtered = self.get_filtered_indices(search_query);
        if !filtered.is_empty() {
            self.list_state.select(Some(0));
            self.selected_connection_key = filtered.first().and_then(|&idx| {
                self.connections.get(idx).map(|c| {
                    (
                        c.pid,
                        c.local_addr.clone(),
                        c.local_port,
                        c.remote_addr.clone(),
                        c.remote_port,
                    )
                })
            });
        }
    }

    pub fn select_last(&mut self, search_query: &str) {
        self.mark_navigation();
        let filtered = self.get_filtered_indices(search_query);
        if !filtered.is_empty() {
            let last_idx = filtered.len() - 1;
            self.list_state.select(Some(last_idx));
            self.selected_connection_key = filtered.get(last_idx).and_then(|&idx| {
                self.connections.get(idx).map(|c| {
                    (
                        c.pid,
                        c.local_addr.clone(),
                        c.local_port,
                        c.remote_addr.clone(),
                        c.remote_port,
                    )
                })
            });
        }
    }
}
