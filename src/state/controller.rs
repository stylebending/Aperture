use std::time::{Duration, Instant};

use ratatui::widgets::ListState;

use crate::sys::service::ServiceInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Status,
    Type,
}

impl SortKey {
    pub fn next(&self) -> Self {
        match self {
            SortKey::Name => SortKey::Status,
            SortKey::Status => SortKey::Type,
            SortKey::Type => SortKey::Name,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SortKey::Name => "Name",
            SortKey::Status => "Status",
            SortKey::Type => "Type",
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

fn status_priority(status: &str) -> u8 {
    match status {
        "Running" => 0,
        "Start Pending" => 1,
        "Stop Pending" => 2,
        "Paused" => 3,
        "Pause Pending" => 4,
        "Continue Pending" => 5,
        "Stopped" => 6,
        _ => 7,
    }
}

pub struct ControllerState {
    pub services: Vec<ServiceInfo>,
    pub list_state: ListState,
    pub active_filter: Option<String>,
    pub selected_service_name: Option<String>,
    pub last_navigation: Instant,
    pub sort_key: SortKey,
    pub sort_order: SortOrder,
    last_data_hash: u64,
    is_initial_load: bool,
}

impl ControllerState {
    const NAVIGATION_DEBOUNCE: Duration = Duration::from_millis(50);

    pub fn new() -> Self {
        Self {
            services: Vec::new(),
            list_state: ListState::default(),
            active_filter: None,
            selected_service_name: None,
            last_navigation: Instant::now(),
            sort_key: SortKey::Status,
            sort_order: SortOrder::Ascending,
            last_data_hash: 0,
            is_initial_load: true,
        }
    }

    fn compute_data_hash(&self, services: &[ServiceInfo]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        services.len().hash(&mut hasher);
        for s in services {
            s.service_name.hash(&mut hasher);
            s.status.hash(&mut hasher);
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

        self.update_selection_from_name();
    }

    pub fn clear_filter(&mut self) {
        // Filter changes are instant - no debounce
        self.active_filter = None;
        self.update_selection_from_name();
    }

    pub fn cycle_sort_key(&mut self) {
        self.sort_key = self.sort_key.next();
        self.sort_services();
        self.update_selection_from_name();
    }

    pub fn toggle_sort_order(&mut self) {
        self.sort_order = self.sort_order.toggle();
        self.sort_services();
        self.update_selection_from_name();
    }

    fn sort_services(&mut self) {
        match self.sort_key {
            SortKey::Name => {
                self.services.sort_by(|a, b| {
                    let cmp = a.display_name.cmp(&b.display_name);
                    if self.sort_order == SortOrder::Descending {
                        cmp.reverse()
                    } else {
                        cmp
                    }
                });
            }
            SortKey::Status => {
                self.services.sort_by(|a, b| {
                    let a_priority = status_priority(&a.status);
                    let b_priority = status_priority(&b.status);
                    let cmp = a_priority.cmp(&b_priority);
                    if self.sort_order == SortOrder::Descending {
                        cmp.reverse()
                    } else {
                        cmp
                    }
                });
            }
            SortKey::Type => {
                self.services.sort_by(|a, b| {
                    let cmp = a.service_type.cmp(&b.service_type);
                    if self.sort_order == SortOrder::Descending {
                        cmp.reverse()
                    } else {
                        cmp
                    }
                });
            }
        }
    }

    fn update_selection_from_name(&mut self) {
        if let Some(ref name) = self.selected_service_name {
            let filtered = self.get_filtered_indices("");
            if let Some(new_idx) = filtered.iter().position(|&i| {
                self.services
                    .get(i)
                    .map(|s| &s.service_name == name)
                    .unwrap_or(false)
            }) {
                self.list_state.select(Some(new_idx));
            } else if !filtered.is_empty() {
                self.list_state.select(Some(0));
                self.selected_service_name = filtered
                    .get(0)
                    .and_then(|&i| self.services.get(i))
                    .map(|s| s.service_name.clone());
            } else {
                self.list_state.select(None);
                self.selected_service_name = None;
            }
        } else if !self.services.is_empty() {
            self.list_state.select(Some(0));
            self.selected_service_name = self.services.get(0).map(|s| s.service_name.clone());
        }
    }

    fn get_filter(&self, search_query: &str) -> Option<String> {
        if !search_query.is_empty() {
            Some(search_query.to_lowercase())
        } else {
            self.active_filter.clone()
        }
    }

    fn matches_filter(&self, service: &ServiceInfo, query: &str) -> bool {
        service.display_name.to_lowercase().contains(query)
            || service.service_name.to_lowercase().contains(query)
    }

    pub fn get_filtered_indices(&self, search_query: &str) -> Vec<usize> {
        match self.get_filter(search_query) {
            None => (0..self.services.len()).collect(),
            Some(query) => self
                .services
                .iter()
                .enumerate()
                .filter(|(_, s)| self.matches_filter(s, &query))
                .map(|(i, _)| i)
                .collect(),
        }
    }

    pub fn filtered_services(&self, search_query: &str) -> Vec<(usize, ServiceInfo)> {
        match self.get_filter(search_query) {
            None => self
                .services
                .iter()
                .enumerate()
                .map(|(i, s)| (i, s.clone()))
                .collect(),
            Some(query) => self
                .services
                .iter()
                .enumerate()
                .filter(|(_, s)| self.matches_filter(s, &query))
                .map(|(i, s)| (i, s.clone()))
                .collect(),
        }
    }

    pub fn update_services(&mut self, services: Vec<ServiceInfo>) {
        // Check if data actually changed
        let new_hash = self.compute_data_hash(&services);

        if new_hash == self.last_data_hash {
            // Data hasn't changed, skip update
            return;
        }
        self.last_data_hash = new_hash;

        // Don't update during active navigation (but always allow initial load)
        if self.should_ignore_update() {
            return;
        }

        self.services = services;
        self.sort_services();
        self.update_selection_from_name();

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
        self.selected_service_name = filtered
            .get(new_idx)
            .and_then(|&idx| self.services.get(idx))
            .map(|s| s.service_name.clone());
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
        self.selected_service_name = filtered
            .get(new_idx)
            .and_then(|&idx| self.services.get(idx))
            .map(|s| s.service_name.clone());
    }

    pub fn select_page_up(&mut self, search_query: &str) {
        self.mark_navigation();
        let filtered = self.get_filtered_indices(search_query);
        if filtered.is_empty() {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        let page_size = 10;
        let new_idx = if i >= page_size { i - page_size } else { 0 };
        self.list_state.select(Some(new_idx));
        self.selected_service_name = filtered
            .get(new_idx)
            .and_then(|&idx| self.services.get(idx))
            .map(|s| s.service_name.clone());
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
        self.selected_service_name = filtered
            .get(new_idx)
            .and_then(|&idx| self.services.get(idx))
            .map(|s| s.service_name.clone());
    }

    pub fn toggle_selected_service(&mut self, search_query: &str) {
        let filtered = self.get_filtered_indices(search_query);
        if let Some(idx) = self.list_state.selected() {
            if let Some(&original_idx) = filtered.get(idx) {
                if let Some(service) = self.services.get(original_idx) {
                    let _ =
                        crate::sys::service::toggle_service(&service.service_name, &service.status);
                }
            }
        }
    }
}
