use std::collections::HashMap;

use ratatui::widgets::ListState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Scope,
}

impl SortKey {
    pub fn next(&self) -> Self {
        match self {
            SortKey::Name => SortKey::Scope,
            SortKey::Scope => SortKey::Name,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SortKey::Name => "Name",
            SortKey::Scope => "Scope",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvScope {
    System,
    User,
    Process,
}

impl EnvScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnvScope::System => "System",
            EnvScope::User => "User",
            EnvScope::Process => "Process",
        }
    }

    fn priority(&self) -> u8 {
        match self {
            EnvScope::System => 0,
            EnvScope::User => 1,
            EnvScope::Process => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnvEntry {
    pub name: String,
    pub value: String,
    pub scope: EnvScope,
    pub overridden: bool,
}

pub struct EnvState {
    pub entries: Vec<EnvEntry>,
    pub list_state: ListState,
    pub active_filter: Option<String>,
    pub sort_key: SortKey,
    pub sort_order: SortOrder,
    last_data_hash: u64,
    is_initial_load: bool,
}

impl EnvState {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            list_state: ListState::default(),
            active_filter: None,
            sort_key: SortKey::Name,
            sort_order: SortOrder::Ascending,
            last_data_hash: 0,
            is_initial_load: true,
        }
    }

    fn compute_data_hash(&self, process: &[(String, String)], user: &[(String, String)], system: &[(String, String)]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        (process.len() + user.len() + system.len()).hash(&mut hasher);
        for (n, v) in process.iter().chain(user.iter()).chain(system.iter()) {
            n.hash(&mut hasher);
            v.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub fn set_filter(&mut self, query: String) {
        if query.is_empty() {
            self.active_filter = None;
        } else {
            self.active_filter = Some(query.to_lowercase());
        }
        self.update_selection();
    }

    pub fn clear_filter(&mut self) {
        self.active_filter = None;
        self.update_selection();
    }

    pub fn cycle_sort_key(&mut self) {
        self.sort_key = self.sort_key.next();
        self.sort_entries();
        self.update_selection();
    }

    pub fn toggle_sort_order(&mut self) {
        self.sort_order = self.sort_order.toggle();
        self.sort_entries();
        self.update_selection();
    }

    fn sort_entries(&mut self) {
        self.entries.sort_by(|a, b| {
            let cmp = match self.sort_key {
                SortKey::Name => a.name.cmp(&b.name),
                SortKey::Scope => a.scope.priority().cmp(&b.scope.priority()),
            };
            if self.sort_order == SortOrder::Descending {
                cmp.reverse()
            } else {
                cmp
            }
        });
    }

    fn update_selection(&mut self) {
        let filtered = self.get_filtered_indices();
        if !filtered.is_empty() {
            let idx = self.list_state.selected().map_or(0, |i| i.min(filtered.len() - 1));
            self.list_state.select(Some(idx));
        } else {
            self.list_state.select(None);
        }
    }

    fn get_filter(&self) -> Option<String> {
        self.active_filter.clone()
    }

    fn matches_filter(&self, entry: &EnvEntry, query: &str) -> bool {
        entry.name.to_lowercase().contains(query)
            || entry.value.to_lowercase().contains(query)
            || entry.scope.as_str().to_lowercase().contains(query)
    }

    pub fn get_filtered_indices(&self) -> Vec<usize> {
        match self.get_filter() {
            None => (0..self.entries.len()).collect(),
            Some(query) => self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| self.matches_filter(e, &query))
                .map(|(i, _)| i)
                .collect(),
        }
    }

    pub fn filtered_entries(&self) -> Vec<(usize, EnvEntry)> {
        match self.get_filter() {
            None => self
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| (i, e.clone()))
                .collect(),
            Some(query) => self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| self.matches_filter(e, &query))
                .map(|(i, e)| (i, e.clone()))
                .collect(),
        }
    }

    pub fn update_env_vars(
        &mut self,
        process_vars: Vec<(String, String)>,
        user_vars: Vec<(String, String)>,
        system_vars: Vec<(String, String)>,
    ) {
        let new_hash = self.compute_data_hash(&process_vars, &user_vars, &system_vars);
        if new_hash == self.last_data_hash {
            return;
        }
        self.last_data_hash = new_hash;

        let mut entries: Vec<EnvEntry> = Vec::new();

        let process_map: HashMap<String, String> = process_vars.into_iter().collect();
        let user_map: HashMap<String, String> = user_vars.into_iter().collect();
        let system_map: HashMap<String, String> = system_vars.into_iter().collect();

        let mut all_names: Vec<String> = process_map
            .keys()
            .chain(user_map.keys())
            .chain(system_map.keys())
            .map(|s| s.to_uppercase())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        all_names.sort();

        for name in all_names {
            let sys_val = system_map.get(&name);
            let usr_val = user_map.get(&name);
            let prc_val = process_map.get(&name);

            if let Some(value) = sys_val {
                let overridden = usr_val.is_some() || prc_val.is_some();
                entries.push(EnvEntry {
                    name: name.clone(),
                    value: value.clone(),
                    scope: EnvScope::System,
                    overridden,
                });
            }

            if let Some(value) = usr_val {
                let overridden = prc_val.is_some();
                entries.push(EnvEntry {
                    name: name.clone(),
                    value: value.clone(),
                    scope: EnvScope::User,
                    overridden,
                });
            }

            if let Some(value) = prc_val {
                entries.push(EnvEntry {
                    name: name.clone(),
                    value: value.clone(),
                    scope: EnvScope::Process,
                    overridden: false,
                });
            }
        }

        self.entries = entries;
        self.sort_entries();
        self.update_selection();

        if self.is_initial_load && !self.entries.is_empty() {
            self.list_state.select(Some(0));
        }
        self.is_initial_load = false;
    }

    pub fn select_next(&mut self) {
        let filtered = self.get_filtered_indices();
        if filtered.is_empty() {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some((i + 1) % filtered.len()));
    }

    pub fn select_prev(&mut self) {
        let filtered = self.get_filtered_indices();
        if filtered.is_empty() {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        self.list_state
            .select(Some((i + filtered.len() - 1) % filtered.len()));
    }

    pub fn select_page_up(&mut self) {
        let filtered = self.get_filtered_indices();
        if filtered.is_empty() {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some(i.saturating_sub(10)));
    }

    pub fn select_page_down(&mut self) {
        let filtered = self.get_filtered_indices();
        if filtered.is_empty() {
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        let new_idx = std::cmp::min(i + 10, filtered.len().saturating_sub(1));
        self.list_state.select(Some(new_idx));
    }

    pub fn select_first(&mut self) {
        let filtered = self.get_filtered_indices();
        if !filtered.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        let filtered = self.get_filtered_indices();
        if !filtered.is_empty() {
            self.list_state.select(Some(filtered.len() - 1));
        }
    }
}
