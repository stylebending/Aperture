use std::time::{Duration, Instant};

use ratatui::widgets::ListState;

use crate::sys::process::ProcessInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Pid,
    Cpu,
    Memory,
}

impl SortKey {
    pub fn next(&self) -> Self {
        match self {
            SortKey::Name => SortKey::Pid,
            SortKey::Pid => SortKey::Cpu,
            SortKey::Cpu => SortKey::Memory,
            SortKey::Memory => SortKey::Name,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SortKey::Name => "Name",
            SortKey::Pid => "PID",
            SortKey::Cpu => "CPU",
            SortKey::Memory => "Mem",
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

pub struct TreeNode {
    pub process: ProcessInfo,
    pub depth: usize,
    pub is_expanded: bool,
    pub has_children: bool,
}

pub struct LockerState {
    pub processes: Vec<ProcessInfo>,
    pub list_state: ListState,
    pub active_filter: Option<String>,
    pub selected_pid: Option<u32>,
    pub last_navigation: Instant,
    pub sort_key: SortKey,
    pub sort_order: SortOrder,
    pub tree_mode: bool,
    pub tree_nodes: Vec<TreeNode>,
    pub expanded_pids: std::collections::HashSet<u32>,
    last_data_hash: u64,
    is_initial_load: bool,
}

impl LockerState {
    // Short debounce for navigation only (50ms) - allows real-time feel while preventing jitter
    const NAVIGATION_DEBOUNCE: Duration = Duration::from_millis(50);

    pub fn new() -> Self {
        Self {
            processes: Vec::new(),
            list_state: ListState::default(),
            active_filter: None,
            selected_pid: None,
            last_navigation: Instant::now(),
            sort_key: SortKey::Cpu,
            sort_order: SortOrder::Descending,
            tree_mode: false,
            tree_nodes: Vec::new(),
            expanded_pids: std::collections::HashSet::new(),
            last_data_hash: 0,
            is_initial_load: true,
        }
    }

    pub fn toggle_tree_mode(&mut self) {
        self.tree_mode = !self.tree_mode;
        if self.tree_mode {
            self.build_tree("");
        }
        self.update_selection_from_pid();
    }

    pub fn toggle_expand(&mut self) {
        if !self.tree_mode {
            return;
        }

        if let Some(idx) = self.list_state.selected() {
            if let Some(node) = self.tree_nodes.get(idx) {
                let pid = node.process.pid;
                if node.has_children {
                    if self.expanded_pids.contains(&pid) {
                        self.expanded_pids.remove(&pid);
                    } else {
                        self.expanded_pids.insert(pid);
                    }
                    self.build_tree("");
                    // Try to restore selection
                    if let Some(new_idx) = self.tree_nodes.iter().position(|n| n.process.pid == pid)
                    {
                        self.list_state.select(Some(new_idx));
                    }
                }
            }
        }
    }

    pub fn build_tree(&mut self, search_query: &str) {
        self.tree_nodes.clear();

        // Determine which processes match the filter
        let matching_pids: std::collections::HashSet<u32> =
            if search_query.is_empty() && self.active_filter.is_none() {
                // No filter - include all processes
                self.processes.iter().map(|p| p.pid).collect()
            } else {
                // Get the effective filter query
                let query = if !search_query.is_empty() {
                    search_query.to_lowercase()
                } else {
                    self.active_filter.clone().unwrap_or_default()
                };

                // Find processes that match the filter
                self.processes
                    .iter()
                    .filter(|p| self.matches_filter(p, &query))
                    .map(|p| p.pid)
                    .collect()
            };

        // Build parent -> children mapping
        let mut children_map: std::collections::HashMap<u32, Vec<usize>> =
            std::collections::HashMap::new();
        for (idx, process) in self.processes.iter().enumerate() {
            children_map
                .entry(process.parent_pid)
                .or_default()
                .push(idx);
        }

        // Build set of PIDs to include (matching + their ancestors)
        let mut include_pids: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let pid_to_idx: std::collections::HashMap<u32, usize> = self
            .processes
            .iter()
            .enumerate()
            .map(|(idx, p)| (p.pid, idx))
            .collect();

        // For each matching process, add it and all its ancestors
        for &matching_pid in &matching_pids {
            let mut current_pid = matching_pid;
            loop {
                include_pids.insert(current_pid);

                // Find parent
                if let Some(&idx) = pid_to_idx.get(&current_pid) {
                    let parent_pid = self.processes[idx].parent_pid;
                    if parent_pid == 0 || !pid_to_idx.contains_key(&parent_pid) {
                        break;
                    }
                    current_pid = parent_pid;
                } else {
                    break;
                }
            }
        }

        // Find root processes (parent_pid == 0 or parent not in our list)
        let pids: std::collections::HashSet<u32> = self.processes.iter().map(|p| p.pid).collect();
        let mut roots: Vec<usize> = Vec::new();

        for (idx, process) in self.processes.iter().enumerate() {
            if (process.parent_pid == 0 || !pids.contains(&process.parent_pid))
                && include_pids.contains(&process.pid)
            {
                roots.push(idx);
            }
        }

        // Sort roots by current sort key
        roots.sort_by(|&a_idx, &b_idx| {
            let a = &self.processes[a_idx];
            let b = &self.processes[b_idx];
            self.compare_processes(a, b)
        });

        // Build tree recursively
        for &root_idx in &roots {
            self.add_tree_node(root_idx, 0, &children_map, &include_pids);
        }
    }

    fn add_tree_node(
        &mut self,
        process_idx: usize,
        depth: usize,
        children_map: &std::collections::HashMap<u32, Vec<usize>>,
        include_pids: &std::collections::HashSet<u32>,
    ) {
        let process = self.processes[process_idx].clone();
        let pid = process.pid;
        let all_children = children_map.get(&pid).cloned().unwrap_or_default();

        // Filter children to only include those in include_pids
        let children: Vec<usize> = all_children
            .into_iter()
            .filter(|&idx| include_pids.contains(&self.processes[idx].pid))
            .collect();

        self.tree_nodes.push(TreeNode {
            process,
            depth,
            is_expanded: self.expanded_pids.contains(&pid),
            has_children: !children.is_empty(),
        });

        if self.expanded_pids.contains(&pid) {
            // Sort children
            let mut sorted_children = children;
            sorted_children.sort_by(|&a_idx, &b_idx| {
                let a = &self.processes[a_idx];
                let b = &self.processes[b_idx];
                self.compare_processes(a, b)
            });

            for &child_idx in &sorted_children {
                self.add_tree_node(child_idx, depth + 1, children_map, include_pids);
            }
        }
    }

    fn compare_processes(&self, a: &ProcessInfo, b: &ProcessInfo) -> std::cmp::Ordering {
        let cmp = match self.sort_key {
            SortKey::Name => a.name.cmp(&b.name),
            SortKey::Pid => a.pid.cmp(&b.pid),
            SortKey::Cpu => {
                let a_val = if a.cpu_usage > 0.0 {
                    a.cpu_usage
                } else {
                    a.last_cpu_usage
                };
                let b_val = if b.cpu_usage > 0.0 {
                    b.cpu_usage
                } else {
                    b.last_cpu_usage
                };
                a_val
                    .partial_cmp(&b_val)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
            SortKey::Memory => {
                let a_val = if a.memory_mb > 0.0 {
                    a.memory_mb
                } else {
                    a.last_memory_mb
                };
                let b_val = if b.memory_mb > 0.0 {
                    b.memory_mb
                } else {
                    b.last_memory_mb
                };
                a_val
                    .partial_cmp(&b_val)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        };

        if self.sort_order == SortOrder::Descending {
            cmp.reverse()
        } else {
            cmp
        }
    }

    fn compute_data_hash(&self, processes: &[ProcessInfo]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        processes.len().hash(&mut hasher);
        for p in processes {
            p.pid.hash(&mut hasher);
            p.name.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub fn should_ignore_update(&self) -> bool {
        // Always allow initial load
        if self.is_initial_load {
            return false;
        }
        // Only debounce actual navigation (not filter operations)
        self.last_navigation.elapsed() < Self::NAVIGATION_DEBOUNCE
    }

    fn mark_navigation(&mut self) {
        self.last_navigation = Instant::now();
    }

    pub fn set_filter(&mut self, query: String) {
        // Don't mark navigation for filter changes - they should be instant
        if query.is_empty() {
            self.active_filter = None;
        } else {
            self.active_filter = Some(query.to_lowercase());
        }

        self.update_selection_from_pid();
    }

    pub fn clear_filter(&mut self) {
        // Don't mark navigation for filter changes - they should be instant
        self.active_filter = None;
        self.update_selection_from_pid();
    }

    pub fn cycle_sort_key(&mut self) {
        self.sort_key = self.sort_key.next();
        self.sort_processes();
        self.update_selection_from_pid();
    }

    pub fn toggle_sort_order(&mut self) {
        self.sort_order = self.sort_order.toggle();
        self.sort_processes();
        self.update_selection_from_pid();
    }

    pub fn sort_processes(&mut self) {
        match self.sort_key {
            SortKey::Name => {
                self.processes.sort_by(|a, b| {
                    let cmp = a.name.cmp(&b.name);
                    if self.sort_order == SortOrder::Descending {
                        cmp.reverse()
                    } else {
                        cmp
                    }
                });
            }
            SortKey::Pid => {
                self.processes.sort_by(|a, b| {
                    let cmp = a.pid.cmp(&b.pid);
                    if self.sort_order == SortOrder::Descending {
                        cmp.reverse()
                    } else {
                        cmp
                    }
                });
            }
            SortKey::Cpu => {
                self.processes.sort_by(|a, b| {
                    let a_val = if a.cpu_usage > 0.0 {
                        a.cpu_usage
                    } else {
                        a.last_cpu_usage
                    };
                    let b_val = if b.cpu_usage > 0.0 {
                        b.cpu_usage
                    } else {
                        b.last_cpu_usage
                    };
                    let cmp = a_val
                        .partial_cmp(&b_val)
                        .unwrap_or(std::cmp::Ordering::Equal);
                    if self.sort_order == SortOrder::Descending {
                        cmp.reverse()
                    } else {
                        cmp
                    }
                });
            }
            SortKey::Memory => {
                self.processes.sort_by(|a, b| {
                    let a_val = if a.memory_mb > 0.0 {
                        a.memory_mb
                    } else {
                        a.last_memory_mb
                    };
                    let b_val = if b.memory_mb > 0.0 {
                        b.memory_mb
                    } else {
                        b.last_memory_mb
                    };
                    let cmp = a_val
                        .partial_cmp(&b_val)
                        .unwrap_or(std::cmp::Ordering::Equal);
                    if self.sort_order == SortOrder::Descending {
                        cmp.reverse()
                    } else {
                        cmp
                    }
                });
            }
        }

        // Rebuild tree if in tree mode
        if self.tree_mode {
            self.build_tree("");
        }
    }

    fn update_selection_from_pid(&mut self) {
        if let Some(pid) = self.selected_pid {
            let filtered = self.get_filtered_indices("");
            if let Some(new_idx) = filtered
                .iter()
                .position(|&i| self.processes.get(i).map(|p| p.pid == pid).unwrap_or(false))
            {
                self.list_state.select(Some(new_idx));
            } else if !filtered.is_empty() {
                self.list_state.select(Some(0));
                self.selected_pid = filtered
                    .first()
                    .and_then(|&i| self.processes.get(i))
                    .map(|p| p.pid);
            } else {
                self.list_state.select(None);
                self.selected_pid = None;
            }
        } else if !self.processes.is_empty() {
            self.list_state.select(Some(0));
            self.selected_pid = self.processes.first().map(|p| p.pid);
        }
    }

    fn get_filter(&self, search_query: &str) -> Option<String> {
        if !search_query.is_empty() {
            Some(search_query.to_lowercase())
        } else {
            self.active_filter.clone()
        }
    }

    fn matches_filter(&self, process: &ProcessInfo, query: &str) -> bool {
        process.name.to_lowercase().contains(query)
            || process
                .path
                .as_ref()
                .map(|path| path.to_lowercase().contains(query))
                .unwrap_or(false)
            || process.pid.to_string().contains(query)
    }

    pub fn get_filtered_indices(&self, search_query: &str) -> Vec<usize> {
        match self.get_filter(search_query) {
            None => (0..self.processes.len()).collect(),
            Some(query) => self
                .processes
                .iter()
                .enumerate()
                .filter(|(_, p)| self.matches_filter(p, &query))
                .map(|(i, _)| i)
                .collect(),
        }
    }

    pub fn filtered_processes(&self, search_query: &str) -> Vec<(usize, ProcessInfo)> {
        match self.get_filter(search_query) {
            None => self
                .processes
                .iter()
                .enumerate()
                .map(|(i, p)| (i, p.clone()))
                .collect(),
            Some(query) => self
                .processes
                .iter()
                .enumerate()
                .filter(|(_, p)| self.matches_filter(p, &query))
                .map(|(i, p)| (i, p.clone()))
                .collect(),
        }
    }

    pub fn update_processes(&mut self, processes: Vec<ProcessInfo>) {
        // Check if data actually changed
        let new_hash = self.compute_data_hash(&processes);

        if new_hash == self.last_data_hash {
            // Data hasn't changed, skip update entirely
            return;
        }
        self.last_data_hash = new_hash;

        // Don't update during active navigation (but always allow initial load)
        if self.should_ignore_update() {
            return;
        }

        // Preserve cached metric values from existing processes to prevent "-" display
        // during the brief window before metrics are updated
        let cached_values: std::collections::HashMap<u32, (f32, f32, f64)> = self
            .processes
            .iter()
            .map(|p| (p.pid, (p.cpu_usage, p.last_cpu_usage, p.last_memory_mb)))
            .collect();

        // Copy cached values to new processes that still exist
        let mut processes = processes;
        for process in &mut processes {
            if let Some((cpu, last_cpu, mem)) = cached_values.get(&process.pid) {
                process.cpu_usage = *cpu;
                process.last_cpu_usage = *last_cpu;
                process.last_memory_mb = *mem;
            }
        }

        self.processes = processes;
        self.sort_processes();

        // Rebuild tree if in tree mode
        if self.tree_mode {
            self.build_tree("");
        }

        // Note: Don't update selection during background updates to prevent cursor jumps
        // Selection is only updated on user-initiated actions (sort change, navigation, etc.)

        // Initialize selection on first load (when is_initial_load is still true)
        if self.is_initial_load && !self.processes.is_empty() {
            self.update_selection_from_pid();
        }

        // Mark initial load as complete after first successful update
        self.is_initial_load = false;
    }

    pub fn select_next(&mut self, search_query: &str) {
        self.mark_navigation();

        if self.tree_mode {
            if self.tree_nodes.is_empty() {
                return;
            }
            let i = self.list_state.selected().unwrap_or(0);
            let new_idx = (i + 1) % self.tree_nodes.len();
            self.list_state.select(Some(new_idx));
            self.selected_pid = self.tree_nodes.get(new_idx).map(|n| n.process.pid);
        } else {
            let filtered = self.get_filtered_indices(search_query);
            if filtered.is_empty() {
                return;
            }
            let i = self.list_state.selected().unwrap_or(0);
            let new_idx = (i + 1) % filtered.len();
            self.list_state.select(Some(new_idx));
            self.selected_pid = filtered
                .get(new_idx)
                .and_then(|&idx| self.processes.get(idx))
                .map(|p| p.pid);
        }
    }

    pub fn select_prev(&mut self, search_query: &str) {
        self.mark_navigation();

        if self.tree_mode {
            if self.tree_nodes.is_empty() {
                return;
            }
            let i = self.list_state.selected().unwrap_or(0);
            let new_idx = (i + self.tree_nodes.len() - 1) % self.tree_nodes.len();
            self.list_state.select(Some(new_idx));
            self.selected_pid = self.tree_nodes.get(new_idx).map(|n| n.process.pid);
        } else {
            let filtered = self.get_filtered_indices(search_query);
            if filtered.is_empty() {
                return;
            }
            let i = self.list_state.selected().unwrap_or(0);
            let new_idx = (i + filtered.len() - 1) % filtered.len();
            self.list_state.select(Some(new_idx));
            self.selected_pid = filtered
                .get(new_idx)
                .and_then(|&idx| self.processes.get(idx))
                .map(|p| p.pid);
        }
    }

    pub fn select_page_up(&mut self, search_query: &str) {
        self.mark_navigation();

        if self.tree_mode {
            if self.tree_nodes.is_empty() {
                return;
            }
            let i = self.list_state.selected().unwrap_or(0);
            let page_size = 10;
            let new_idx = i.saturating_sub(page_size);
            self.list_state.select(Some(new_idx));
            self.selected_pid = self.tree_nodes.get(new_idx).map(|n| n.process.pid);
        } else {
            let filtered = self.get_filtered_indices(search_query);
            if filtered.is_empty() {
                return;
            }
            let i = self.list_state.selected().unwrap_or(0);
            let page_size = 10;
            let new_idx = i.saturating_sub(page_size);
            self.list_state.select(Some(new_idx));
            self.selected_pid = filtered
                .get(new_idx)
                .and_then(|&idx| self.processes.get(idx))
                .map(|p| p.pid);
        }
    }

    pub fn select_page_down(&mut self, search_query: &str) {
        self.mark_navigation();

        if self.tree_mode {
            if self.tree_nodes.is_empty() {
                return;
            }
            let i = self.list_state.selected().unwrap_or(0);
            let page_size = 10;
            let new_idx = std::cmp::min(i + page_size, self.tree_nodes.len().saturating_sub(1));
            self.list_state.select(Some(new_idx));
            self.selected_pid = self.tree_nodes.get(new_idx).map(|n| n.process.pid);
        } else {
            let filtered = self.get_filtered_indices(search_query);
            if filtered.is_empty() {
                return;
            }
            let i = self.list_state.selected().unwrap_or(0);
            let page_size = 10;
            let new_idx = std::cmp::min(i + page_size, filtered.len().saturating_sub(1));
            self.list_state.select(Some(new_idx));
            self.selected_pid = filtered
                .get(new_idx)
                .and_then(|&idx| self.processes.get(idx))
                .map(|p| p.pid);
        }
    }

    pub fn select_first(&mut self, search_query: &str) {
        self.mark_navigation();

        if self.tree_mode {
            if !self.tree_nodes.is_empty() {
                self.list_state.select(Some(0));
                self.selected_pid = self.tree_nodes.first().map(|n| n.process.pid);
            }
        } else {
            let filtered = self.get_filtered_indices(search_query);
            if !filtered.is_empty() {
                self.list_state.select(Some(0));
                self.selected_pid = filtered
                    .first()
                    .and_then(|&idx| self.processes.get(idx))
                    .map(|p| p.pid);
            }
        }
    }

    pub fn select_last(&mut self, search_query: &str) {
        self.mark_navigation();

        if self.tree_mode {
            if !self.tree_nodes.is_empty() {
                let last_idx = self.tree_nodes.len() - 1;
                self.list_state.select(Some(last_idx));
                self.selected_pid = self.tree_nodes.get(last_idx).map(|n| n.process.pid);
            }
        } else {
            let filtered = self.get_filtered_indices(search_query);
            if !filtered.is_empty() {
                let last_idx = filtered.len() - 1;
                self.list_state.select(Some(last_idx));
                self.selected_pid = filtered
                    .get(last_idx)
                    .and_then(|&idx| self.processes.get(idx))
                    .map(|p| p.pid);
            }
        }
    }

    pub fn get_selected_process(&self, search_query: &str) -> Option<&ProcessInfo> {
        if self.tree_mode {
            self.list_state
                .selected()
                .and_then(|idx| self.tree_nodes.get(idx))
                .map(|n| &n.process)
        } else {
            let filtered = self.get_filtered_indices(search_query);
            self.list_state
                .selected()
                .and_then(|idx| filtered.get(idx))
                .and_then(|&original_idx| self.processes.get(original_idx))
        }
    }
}
