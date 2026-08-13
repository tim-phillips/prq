use std::collections::HashSet;
use std::time::Instant;

use ratatui::widgets::TableState;

use crate::model::{Attention, PrDetail, PrStack, PrSummary, group_stacks};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Detail(u32),
}

/// One visible row in the list: a collapsible stack header, or a PR
/// (standalone, or a member of an expanded stack).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListRow {
    StackHeader(usize),
    Pr { group: usize, member: usize },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AttentionCounts {
    pub to_review: usize,
    pub changes_requested: usize,
    pub conflicts: usize,
    pub ready: usize,
}

pub struct App {
    pub repo: String,
    pub groups: Vec<PrStack>,
    pub rows: Vec<ListRow>,
    expanded: HashSet<u32>,
    pub table_state: TableState,
    pub mode: ViewMode,
    pub loading_list: bool,
    pub loading_detail: bool,
    pub detail: Option<PrDetail>,
    pub last_refresh: Option<Instant>,
    pub last_error: Option<String>,
    pub show_help: bool,
    pub should_quit: bool,
    pub refresh_interval_secs: u64,
    pub auto_refresh: bool,
    pub limit: u32,
}

impl App {
    pub fn new(repo: String, refresh_interval_secs: u64, auto_refresh: bool, limit: u32) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        App {
            repo,
            groups: Vec::new(),
            rows: Vec::new(),
            expanded: HashSet::new(),
            table_state,
            mode: ViewMode::List,
            loading_list: true,
            loading_detail: false,
            detail: None,
            last_refresh: None,
            last_error: None,
            show_help: false,
            should_quit: false,
            refresh_interval_secs,
            auto_refresh,
            limit,
        }
    }

    pub fn pr_count(&self) -> usize {
        self.groups.iter().map(|g| g.prs.len()).sum()
    }

    /// How many PRs need something from the viewer, by kind. Shown in the
    /// header so the queue is readable without scanning the Me column.
    pub fn attention_counts(&self) -> AttentionCounts {
        let mut counts = AttentionCounts::default();
        for pr in self.groups.iter().flat_map(|g| &g.prs) {
            match pr.attention {
                Attention::ReviewRequested => counts.to_review += 1,
                Attention::MyPrChangesRequested => counts.changes_requested += 1,
                Attention::MyPrConflicts => counts.conflicts += 1,
                Attention::MyPrReadyToMerge => counts.ready += 1,
                _ => {}
            }
        }
        counts
    }

    pub fn is_expanded(&self, group: usize) -> bool {
        self.groups
            .get(group)
            .is_some_and(|g| self.expanded.contains(&g.bottom().number))
    }

    fn rebuild_rows(&mut self) {
        self.rows.clear();
        for (g, stack) in self.groups.iter().enumerate() {
            if stack.is_stack() {
                self.rows.push(ListRow::StackHeader(g));
                if self.expanded.contains(&stack.bottom().number) {
                    for m in 0..stack.prs.len() {
                        self.rows.push(ListRow::Pr {
                            group: g,
                            member: m,
                        });
                    }
                }
            } else {
                self.rows.push(ListRow::Pr {
                    group: g,
                    member: 0,
                });
            }
        }
    }

    pub fn selected_row(&self) -> Option<ListRow> {
        self.rows.get(self.table_state.selected()?).copied()
    }

    pub fn selected_pr(&self) -> Option<&PrSummary> {
        match self.selected_row()? {
            ListRow::Pr { group, member } => self.groups.get(group)?.prs.get(member),
            ListRow::StackHeader(_) => None,
        }
    }

    /// URL to open for the selection; a stack header opens its bottom PR.
    pub fn selected_url(&self) -> Option<String> {
        match self.selected_row()? {
            ListRow::Pr { group, member } => {
                Some(self.groups.get(group)?.prs.get(member)?.url.clone())
            }
            ListRow::StackHeader(g) => Some(self.groups.get(g)?.bottom().url.clone()),
        }
    }

    /// Expand or collapse the selected stack header. Returns false when the
    /// selection is not a stack header.
    pub fn toggle_selected_stack(&mut self) -> bool {
        let Some(ListRow::StackHeader(g)) = self.selected_row() else {
            return false;
        };
        let bottom = self.groups[g].bottom().number;
        if !self.expanded.remove(&bottom) {
            self.expanded.insert(bottom);
        }
        self.rebuild_rows();
        if let Some(idx) = self.rows.iter().position(|r| *r == ListRow::StackHeader(g)) {
            self.table_state.select(Some(idx));
        }
        true
    }

    pub fn select_next(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let new = match self.table_state.selected() {
            Some(i) if i + 1 < self.rows.len() => i + 1,
            Some(i) => i,
            None => 0,
        };
        self.table_state.select(Some(new));
    }

    pub fn select_prev(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let new = match self.table_state.selected() {
            Some(i) if i > 0 => i - 1,
            _ => 0,
        };
        self.table_state.select(Some(new));
    }

    pub fn select_first(&mut self) {
        if !self.rows.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        if !self.rows.is_empty() {
            self.table_state.select(Some(self.rows.len() - 1));
        }
    }

    pub fn apply_prs(&mut self, prs: Vec<PrSummary>) {
        let keep = self.selected_number();
        self.groups = group_stacks(prs);
        let live: HashSet<u32> = self
            .groups
            .iter()
            .filter(|g| g.is_stack())
            .map(|g| g.bottom().number)
            .collect();
        self.expanded.retain(|n| live.contains(n));
        self.rebuild_rows();
        if self.rows.is_empty() {
            self.table_state.select(None);
        } else {
            let idx = keep.and_then(|n| self.row_index_for(n)).unwrap_or(0);
            self.table_state.select(Some(idx));
        }
        self.loading_list = false;
        self.last_refresh = Some(Instant::now());
        self.last_error = None;
    }

    /// PR number identifying the current selection (a stack header is
    /// identified by its bottom PR). Used to re-anchor the selection after
    /// a refresh and as the target for actions like opening in workmux.
    pub fn selected_number(&self) -> Option<u32> {
        match self.selected_row()? {
            ListRow::Pr { group, member } => Some(self.groups[group].prs[member].number),
            ListRow::StackHeader(g) => Some(self.groups[g].bottom().number),
        }
    }

    fn row_index_for(&self, number: u32) -> Option<usize> {
        let exact = self.rows.iter().position(|r| {
            matches!(*r, ListRow::Pr { group, member }
                if self.groups[group].prs[member].number == number)
        });
        exact.or_else(|| {
            self.rows.iter().position(|r| {
                matches!(*r, ListRow::StackHeader(g)
                    if self.groups[g].prs.iter().any(|p| p.number == number))
            })
        })
    }

    pub fn apply_list_error(&mut self, err: String) {
        self.loading_list = false;
        self.last_error = Some(err);
    }

    pub fn apply_detail(&mut self, detail: PrDetail) {
        self.loading_detail = false;
        self.detail = Some(detail);
    }

    pub fn apply_detail_error(&mut self, err: String) {
        self.loading_detail = false;
        self.last_error = Some(err);
    }

    pub fn enter_detail(&mut self) {
        if let Some(pr) = self.selected_pr() {
            self.mode = ViewMode::Detail(pr.number);
            self.detail = None;
            self.loading_detail = true;
        }
    }

    pub fn back_to_list(&mut self) {
        self.mode = ViewMode::List;
        self.detail = None;
        self.loading_detail = false;
    }

    pub fn needs_auto_refresh(&self) -> bool {
        if !self.auto_refresh || self.loading_list {
            return false;
        }
        match self.last_refresh {
            None => false,
            Some(t) => t.elapsed().as_secs() >= self.refresh_interval_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::parse_pr_list;

    const STACKED_FIXTURE: &str = include_str!("../tests/fixtures/pr_list_stacked.json");

    fn app_with_fixture() -> App {
        let mut app = App::new("example/repo".into(), 60, true, 100);
        let prs = parse_pr_list(STACKED_FIXTURE, Some("tim")).unwrap();
        app.apply_prs(prs);
        app
    }

    #[test]
    fn stacks_collapse_to_one_row() {
        let app = app_with_fixture();
        assert_eq!(app.pr_count(), 5);
        // 3-PR stack collapses to a header, plus two standalone PRs.
        assert_eq!(app.rows.len(), 3);
        assert!(
            app.rows
                .iter()
                .any(|r| matches!(r, ListRow::StackHeader(_)))
        );
    }

    #[test]
    fn toggle_expands_and_collapses_stack() {
        let mut app = app_with_fixture();
        let header_idx = app
            .rows
            .iter()
            .position(|r| matches!(r, ListRow::StackHeader(_)))
            .unwrap();
        app.table_state.select(Some(header_idx));

        assert!(app.toggle_selected_stack());
        assert_eq!(app.rows.len(), 6);
        // Selection stays on the header.
        assert!(matches!(app.selected_row(), Some(ListRow::StackHeader(_))));
        // Members follow the header, bottom to top.
        let member_numbers: Vec<u32> = app.rows[header_idx + 1..header_idx + 4]
            .iter()
            .map(|r| match *r {
                ListRow::Pr { group, member } => app.groups[group].prs[member].number,
                _ => panic!("expected PR row"),
            })
            .collect();
        assert_eq!(member_numbers, vec![201, 202, 203]);

        assert!(app.toggle_selected_stack());
        assert_eq!(app.rows.len(), 3);
    }

    #[test]
    fn toggle_is_noop_on_pr_row() {
        let mut app = app_with_fixture();
        let pr_idx = app
            .rows
            .iter()
            .position(|r| matches!(r, ListRow::Pr { .. }))
            .unwrap();
        app.table_state.select(Some(pr_idx));
        assert!(!app.toggle_selected_stack());
    }

    #[test]
    fn selection_and_expansion_survive_refresh() {
        let mut app = app_with_fixture();
        let header_idx = app
            .rows
            .iter()
            .position(|r| matches!(r, ListRow::StackHeader(_)))
            .unwrap();
        app.table_state.select(Some(header_idx));
        app.toggle_selected_stack();

        // Select a member inside the expanded stack, then refresh.
        app.table_state.select(Some(header_idx + 2));
        let selected = app.selected_pr().unwrap().number;
        let prs = parse_pr_list(STACKED_FIXTURE, Some("tim")).unwrap();
        app.apply_prs(prs);

        assert_eq!(app.rows.len(), 6); // still expanded
        assert_eq!(app.selected_pr().unwrap().number, selected);
    }

    #[test]
    fn attention_counts_tally_by_kind() {
        let json = r#"[
            {"number": 1, "title": "review me", "author": {"login": "alice"}, "headRefName": "a", "baseRefName": "main",
             "reviewRequests": [{"login": "tim"}]},
            {"number": 2, "title": "reworked", "author": {"login": "tim"}, "headRefName": "b", "baseRefName": "main",
             "mergeable": "MERGEABLE", "reviewDecision": "CHANGES_REQUESTED"},
            {"number": 3, "title": "conflicted", "author": {"login": "tim"}, "headRefName": "c", "baseRefName": "main",
             "mergeable": "CONFLICTING"},
            {"number": 4, "title": "ready", "author": {"login": "tim"}, "headRefName": "d", "baseRefName": "main",
             "mergeable": "MERGEABLE", "reviewDecision": "APPROVED"},
            {"number": 5, "title": "not mine", "author": {"login": "bob"}, "headRefName": "e", "baseRefName": "main"}
        ]"#;
        let mut app = App::new("example/repo".into(), 60, true, 100);
        app.apply_prs(parse_pr_list(json, Some("tim")).unwrap());
        let counts = app.attention_counts();
        assert_eq!(counts.to_review, 1);
        assert_eq!(counts.changes_requested, 1);
        assert_eq!(counts.conflicts, 1);
        assert_eq!(counts.ready, 1);
    }

    #[test]
    fn header_selection_falls_back_after_member_collapses() {
        let mut app = app_with_fixture();
        let header_idx = app
            .rows
            .iter()
            .position(|r| matches!(r, ListRow::StackHeader(_)))
            .unwrap();
        app.table_state.select(Some(header_idx));
        assert!(app.selected_pr().is_none());
        assert!(app.selected_url().is_some());
    }
}
