use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    Approved,
    ChangesRequested,
    ReviewRequired,
    Reviewed,
    None,
}

impl ReviewState {
    pub fn from_decision(s: &str) -> Self {
        match s {
            "APPROVED" => ReviewState::Approved,
            "CHANGES_REQUESTED" => ReviewState::ChangesRequested,
            "REVIEW_REQUIRED" => ReviewState::ReviewRequired,
            "" => ReviewState::None,
            _ => ReviewState::Reviewed,
        }
    }

    fn urgency(self) -> u8 {
        match self {
            ReviewState::ChangesRequested => 4,
            ReviewState::ReviewRequired => 3,
            ReviewState::Reviewed => 2,
            ReviewState::None => 1,
            ReviewState::Approved => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    Pass,
    Fail,
    Pending,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mergeable {
    Mergeable,
    Conflicting,
    Unknown,
}

impl Mergeable {
    fn from_str(s: &str) -> Self {
        match s {
            "MERGEABLE" => Mergeable::Mergeable,
            "CONFLICTING" => Mergeable::Conflicting,
            _ => Mergeable::Unknown,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CheckRollup {
    pub passing: u16,
    pub failing: u16,
    pub pending: u16,
    pub skipped: u16,
    pub overall: Option<CheckState>,
}

impl CheckRollup {
    pub fn from_raw(entries: &[RawCheck]) -> Self {
        let mut rollup = CheckRollup::default();
        for e in entries {
            match e.classify() {
                CheckState::Pass => rollup.passing += 1,
                CheckState::Fail => rollup.failing += 1,
                CheckState::Pending => rollup.pending += 1,
                CheckState::None => rollup.skipped += 1,
            }
        }
        rollup.overall = if entries.is_empty() {
            None
        } else if rollup.failing > 0 {
            Some(CheckState::Fail)
        } else if rollup.pending > 0 {
            Some(CheckState::Pending)
        } else if rollup.passing > 0 {
            Some(CheckState::Pass)
        } else {
            Some(CheckState::None)
        };
        rollup
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawCheck {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub conclusion: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

impl RawCheck {
    pub fn label(&self) -> &str {
        self.name
            .as_deref()
            .or(self.context.as_deref())
            .unwrap_or("check")
    }

    pub fn classify(&self) -> CheckState {
        if let Some(state) = self.state.as_deref() {
            return match state {
                "SUCCESS" => CheckState::Pass,
                "FAILURE" | "ERROR" => CheckState::Fail,
                "PENDING" | "EXPECTED" => CheckState::Pending,
                _ => CheckState::None,
            };
        }
        let status = self.status.as_deref().unwrap_or("");
        if status != "COMPLETED" && !status.is_empty() {
            return CheckState::Pending;
        }
        match self.conclusion.as_deref().unwrap_or("") {
            "SUCCESS" => CheckState::Pass,
            "FAILURE" | "TIMED_OUT" | "ACTION_REQUIRED" | "STARTUP_FAILURE" | "CANCELLED" => {
                CheckState::Fail
            }
            "SKIPPED" | "NEUTRAL" | "STALE" => CheckState::None,
            "" => CheckState::Pending,
            _ => CheckState::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckRun {
    pub name: String,
    pub state: CheckState,
}

impl From<&RawCheck> for CheckRun {
    fn from(raw: &RawCheck) -> Self {
        CheckRun {
            name: raw.label().to_string(),
            state: raw.classify(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewerState {
    Approved,
    ChangesRequested,
    Commented,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MyReviewState {
    NotInvolved,
    ReviewRequested,
    WaitingOnAuthor,
    Approved,
    Commented,
}

impl MyReviewState {
    fn urgency(self) -> u8 {
        match self {
            MyReviewState::ReviewRequested => 4,
            MyReviewState::WaitingOnAuthor => 3,
            MyReviewState::Commented => 2,
            MyReviewState::Approved => 1,
            MyReviewState::NotInvolved => 0,
        }
    }

    fn resolve(r: &RawPr, viewer: Option<&str>) -> Self {
        let Some(viewer) = viewer else {
            return MyReviewState::NotInvolved;
        };
        if r.review_requests.iter().any(|req| req.login == viewer) {
            return MyReviewState::ReviewRequested;
        }
        let mine = r.latest_reviews.iter().find(|rv| rv.author.login == viewer);
        match mine.map(|rv| rv.state.as_str()) {
            Some("APPROVED") => MyReviewState::Approved,
            Some("CHANGES_REQUESTED") => MyReviewState::WaitingOnAuthor,
            Some("COMMENTED") => MyReviewState::Commented,
            _ => MyReviewState::NotInvolved,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Reviewer {
    pub login: String,
    pub state: ReviewerState,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawAuthor {
    #[serde(default)]
    login: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RawReviewer {
    #[serde(default)]
    login: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawReview {
    author: RawAuthor,
    #[serde(default)]
    state: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawPr {
    pub number: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    author: RawAuthor,
    #[serde(default, rename = "headRefName")]
    head_ref: String,
    #[serde(default, rename = "baseRefName")]
    base_ref: String,
    #[serde(default, rename = "isDraft")]
    is_draft: bool,
    #[serde(default)]
    mergeable: String,
    #[serde(default, rename = "updatedAt")]
    updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub url: String,
    #[serde(default, rename = "reviewDecision")]
    review_decision: String,
    #[serde(default, rename = "statusCheckRollup")]
    status_check_rollup: Vec<RawCheck>,
    #[serde(default, rename = "reviewRequests")]
    review_requests: Vec<RawReviewer>,
    #[serde(default, rename = "latestReviews")]
    latest_reviews: Vec<RawReview>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub additions: Option<u32>,
    #[serde(default)]
    pub deletions: Option<u32>,
    #[serde(default, rename = "changedFiles")]
    pub changed_files: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct PrSummary {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub head_ref: String,
    pub base_ref: String,
    pub is_draft: bool,
    pub mergeable: Mergeable,
    pub updated_at: Option<DateTime<Utc>>,
    pub url: String,
    pub review: ReviewState,
    pub my_review: MyReviewState,
    pub checks: CheckRollup,
}

impl PrSummary {
    pub fn from_raw(r: &RawPr, viewer: Option<&str>) -> Self {
        PrSummary {
            number: r.number,
            title: r.title.clone(),
            author: r.author.login.clone(),
            head_ref: r.head_ref.clone(),
            base_ref: r.base_ref.clone(),
            is_draft: r.is_draft,
            mergeable: Mergeable::from_str(&r.mergeable),
            updated_at: r.updated_at,
            url: r.url.clone(),
            review: ReviewState::from_decision(&r.review_decision),
            my_review: MyReviewState::resolve(r, viewer),
            checks: CheckRollup::from_raw(&r.status_check_rollup),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrDetail {
    pub summary: PrSummary,
    pub body: String,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
    pub checks: Vec<CheckRun>,
    pub reviewers: Vec<Reviewer>,
}

impl PrDetail {
    pub fn from_raw(r: &RawPr, viewer: Option<&str>) -> Self {
        let mut reviewers: Vec<Reviewer> = r
            .latest_reviews
            .iter()
            .filter_map(|rv| {
                let state = match rv.state.as_str() {
                    "APPROVED" => ReviewerState::Approved,
                    "CHANGES_REQUESTED" => ReviewerState::ChangesRequested,
                    "COMMENTED" => ReviewerState::Commented,
                    _ => return None,
                };
                Some(Reviewer {
                    login: rv.author.login.clone(),
                    state,
                })
            })
            .collect();

        for req in &r.review_requests {
            let login = if req.login.is_empty() {
                req.name.clone().unwrap_or_default()
            } else {
                req.login.clone()
            };
            if login.is_empty() {
                continue;
            }
            if !reviewers.iter().any(|r| r.login == login) {
                reviewers.push(Reviewer {
                    login,
                    state: ReviewerState::Pending,
                });
            }
        }

        PrDetail {
            summary: PrSummary::from_raw(r, viewer),
            body: r.body.clone().unwrap_or_default(),
            additions: r.additions.unwrap_or(0),
            deletions: r.deletions.unwrap_or(0),
            changed_files: r.changed_files.unwrap_or(0),
            checks: r.status_check_rollup.iter().map(CheckRun::from).collect(),
            reviewers,
        }
    }
}

/// One entry in the PR list: either a standalone PR (`prs.len() == 1`) or a
/// stack of dependent PRs ordered bottom (closest to trunk) to top.
#[derive(Debug, Clone)]
pub struct PrStack {
    pub prs: Vec<PrSummary>,
}

impl PrStack {
    pub fn is_stack(&self) -> bool {
        self.prs.len() > 1
    }

    pub fn bottom(&self) -> &PrSummary {
        &self.prs[0]
    }

    pub fn newest_update(&self) -> Option<DateTime<Utc>> {
        self.prs.iter().filter_map(|p| p.updated_at).max()
    }

    pub fn all_drafts(&self) -> bool {
        self.prs.iter().all(|p| p.is_draft)
    }

    pub fn review_rollup(&self) -> ReviewState {
        self.prs
            .iter()
            .map(|p| p.review)
            .max_by_key(|r| r.urgency())
            .unwrap_or(ReviewState::None)
    }

    pub fn my_review_rollup(&self) -> MyReviewState {
        self.prs
            .iter()
            .map(|p| p.my_review)
            .max_by_key(|r| r.urgency())
            .unwrap_or(MyReviewState::NotInvolved)
    }

    pub fn checks_rollup(&self) -> CheckRollup {
        let mut rollup = CheckRollup::default();
        let mut any = false;
        for pr in &self.prs {
            rollup.passing += pr.checks.passing;
            rollup.failing += pr.checks.failing;
            rollup.pending += pr.checks.pending;
            rollup.skipped += pr.checks.skipped;
            any |= pr.checks.overall.is_some();
        }
        rollup.overall = if !any {
            None
        } else if rollup.failing > 0 {
            Some(CheckState::Fail)
        } else if rollup.pending > 0 {
            Some(CheckState::Pending)
        } else if rollup.passing > 0 {
            Some(CheckState::Pass)
        } else {
            Some(CheckState::None)
        };
        rollup
    }
}

/// Group open PRs into stacks: a PR whose base branch is another open PR's
/// head branch is stacked on top of it (how gh-stack and similar tools model
/// stacks). Each group is ordered bottom to top; groups are sorted by the
/// most recently updated PR they contain.
pub fn group_stacks(prs: Vec<PrSummary>) -> Vec<PrStack> {
    use std::collections::HashMap;

    let mut head_to_idx: HashMap<&str, usize> = HashMap::new();
    for (i, pr) in prs.iter().enumerate() {
        head_to_idx.entry(pr.head_ref.as_str()).or_insert(i);
    }

    let mut children: Vec<Vec<usize>> = vec![Vec::new(); prs.len()];
    let mut has_parent = vec![false; prs.len()];
    for (i, pr) in prs.iter().enumerate() {
        if pr.base_ref.is_empty() {
            continue;
        }
        if let Some(&parent) = head_to_idx.get(pr.base_ref.as_str())
            && parent != i
        {
            children[parent].push(i);
            has_parent[i] = true;
        }
    }
    for c in &mut children {
        c.sort_by_key(|&i| prs[i].number);
    }

    let mut visited = vec![false; prs.len()];
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for root in 0..prs.len() {
        if has_parent[root] || visited[root] {
            continue;
        }
        let mut order = Vec::new();
        let mut pending = vec![root];
        while let Some(i) = pending.pop() {
            if visited[i] {
                continue;
            }
            visited[i] = true;
            order.push(i);
            for &c in children[i].iter().rev() {
                pending.push(c);
            }
        }
        groups.push(order);
    }
    // A base-branch cycle has no root; fall back to standalone entries.
    for (i, seen) in visited.iter().enumerate() {
        if !seen {
            groups.push(vec![i]);
        }
    }

    let mut slots: Vec<Option<PrSummary>> = prs.into_iter().map(Some).collect();
    let mut out: Vec<PrStack> = groups
        .into_iter()
        .map(|idxs| PrStack {
            prs: idxs.into_iter().map(|i| slots[i].take().unwrap()).collect(),
        })
        .collect();
    out.sort_by_key(|s| std::cmp::Reverse(s.newest_update()));
    out
}

pub fn parse_pr_list(json: &str, viewer: Option<&str>) -> anyhow::Result<Vec<PrSummary>> {
    let raws: Vec<RawPr> = serde_json::from_str(json)?;
    let mut prs: Vec<PrSummary> = raws
        .iter()
        .map(|r| PrSummary::from_raw(r, viewer))
        .collect();
    prs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(prs)
}

pub fn parse_pr_detail(json: &str, viewer: Option<&str>) -> anyhow::Result<PrDetail> {
    let raw: RawPr = serde_json::from_str(json)?;
    Ok(PrDetail::from_raw(&raw, viewer))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST_FIXTURE: &str = include_str!("../tests/fixtures/pr_list.json");
    const STACKED_FIXTURE: &str = include_str!("../tests/fixtures/pr_list_stacked.json");
    const VIEW_FIXTURE: &str = include_str!("../tests/fixtures/pr_view.json");

    #[test]
    fn parses_list_fixture() {
        let prs = parse_pr_list(LIST_FIXTURE, Some("tim")).unwrap();
        assert_eq!(prs.len(), 4);

        assert_eq!(
            prs.iter().map(|p| p.number).collect::<Vec<_>>(),
            vec![103, 101, 104, 102]
        );

        let approved = prs.iter().find(|p| p.number == 101).unwrap();
        assert_eq!(approved.review, ReviewState::Approved);
        assert_eq!(approved.checks.overall, Some(CheckState::Pass));
        assert_eq!(approved.checks.passing, 2);
        assert_eq!(approved.author, "alice");
        assert!(!approved.is_draft);
        assert_eq!(approved.mergeable, Mergeable::Mergeable);
        assert_eq!(approved.my_review, MyReviewState::Approved);

        let changes = prs.iter().find(|p| p.number == 102).unwrap();
        assert_eq!(changes.review, ReviewState::ChangesRequested);
        assert_eq!(changes.checks.overall, Some(CheckState::Fail));
        assert_eq!(changes.checks.failing, 1);
        assert_eq!(changes.checks.passing, 1);
        assert_eq!(changes.my_review, MyReviewState::WaitingOnAuthor);

        let pending = prs.iter().find(|p| p.number == 103).unwrap();
        assert_eq!(pending.review, ReviewState::ReviewRequired);
        assert_eq!(pending.checks.overall, Some(CheckState::Pending));
        assert!(pending.checks.pending >= 1);
        assert_eq!(pending.my_review, MyReviewState::ReviewRequested);

        let draft = prs.iter().find(|p| p.number == 104).unwrap();
        assert!(draft.is_draft);
        assert_eq!(draft.review, ReviewState::None);
        assert_eq!(draft.checks.overall, None);
        assert_eq!(draft.my_review, MyReviewState::NotInvolved);
    }

    #[test]
    fn my_review_waiting_on_author_is_distinct_from_review_requested() {
        let prs = parse_pr_list(LIST_FIXTURE, Some("tim")).unwrap();
        let waiting = prs.iter().find(|p| p.number == 102).unwrap();
        let requested = prs.iter().find(|p| p.number == 103).unwrap();
        assert_eq!(waiting.my_review, MyReviewState::WaitingOnAuthor);
        assert_eq!(requested.my_review, MyReviewState::ReviewRequested);
    }

    #[test]
    fn no_viewer_means_all_not_involved() {
        let prs = parse_pr_list(LIST_FIXTURE, None).unwrap();
        for pr in &prs {
            assert_eq!(pr.my_review, MyReviewState::NotInvolved);
        }
    }

    #[test]
    fn parses_view_fixture() {
        let detail = parse_pr_detail(VIEW_FIXTURE, Some("tim")).unwrap();
        assert_eq!(detail.summary.number, 101);
        assert_eq!(detail.additions, 120);
        assert_eq!(detail.deletions, 30);
        assert_eq!(detail.changed_files, 5);
        assert_eq!(detail.checks.len(), 2);
        let approved_reviewer = detail
            .reviewers
            .iter()
            .find(|r| r.state == ReviewerState::Approved);
        assert!(approved_reviewer.is_some());
        let pending_reviewer = detail
            .reviewers
            .iter()
            .find(|r| r.state == ReviewerState::Pending);
        assert!(pending_reviewer.is_some());
        assert_eq!(detail.summary.my_review, MyReviewState::Approved);
    }

    #[test]
    fn groups_stacked_prs_by_base_branch() {
        let prs = parse_pr_list(STACKED_FIXTURE, Some("tim")).unwrap();
        let groups = group_stacks(prs);

        assert_eq!(groups.len(), 3);

        let stack = groups.iter().find(|g| g.is_stack()).unwrap();
        assert_eq!(
            stack.prs.iter().map(|p| p.number).collect::<Vec<_>>(),
            vec![201, 202, 203]
        );
        assert_eq!(stack.bottom().number, 201);

        // Groups sort by their newest member: #204 (13:00), stack (12:00), #205 (09:00).
        assert_eq!(groups[0].bottom().number, 204);
        assert!(groups[1].is_stack());
        assert_eq!(groups[2].bottom().number, 205);
    }

    #[test]
    fn stack_rollups_surface_most_urgent_state() {
        let prs = parse_pr_list(STACKED_FIXTURE, Some("tim")).unwrap();
        let groups = group_stacks(prs);
        let stack = groups.iter().find(|g| g.is_stack()).unwrap();

        // 201 approved, 202 review required, 203 no decision.
        assert_eq!(stack.review_rollup(), ReviewState::ReviewRequired);
        // Review requested from tim on 202 beats tim's approval on 201.
        assert_eq!(stack.my_review_rollup(), MyReviewState::ReviewRequested);
        // 201 passing + 202 pending → pending overall.
        assert_eq!(stack.checks_rollup().overall, Some(CheckState::Pending));
        assert!(!stack.all_drafts());
    }

    #[test]
    fn tree_shaped_stack_groups_all_descendants() {
        // One parent PR with three siblings based on it, like a feature
        // branch with several follow-ups.
        let json = r#"[
            {"number": 10, "title": "base", "headRefName": "feat", "baseRefName": "main", "updatedAt": "2026-04-22T10:00:00Z"},
            {"number": 12, "title": "b", "headRefName": "feat-b", "baseRefName": "feat", "updatedAt": "2026-04-22T11:00:00Z"},
            {"number": 11, "title": "a", "headRefName": "feat-a", "baseRefName": "feat", "updatedAt": "2026-04-22T12:00:00Z"},
            {"number": 13, "title": "c", "headRefName": "feat-c", "baseRefName": "feat", "updatedAt": "2026-04-22T13:00:00Z"}
        ]"#;
        let prs = parse_pr_list(json, None).unwrap();
        let groups = group_stacks(prs);
        assert_eq!(groups.len(), 1);
        let stack = &groups[0];
        assert_eq!(stack.bottom().number, 10);
        // Siblings follow the parent in PR-number order.
        assert_eq!(
            stack.prs.iter().map(|p| p.number).collect::<Vec<_>>(),
            vec![10, 11, 12, 13]
        );
    }

    #[test]
    fn base_branch_cycle_falls_back_to_standalone() {
        let json = r#"[
            {"number": 1, "title": "a", "headRefName": "x", "baseRefName": "y", "updatedAt": "2026-04-22T10:00:00Z"},
            {"number": 2, "title": "b", "headRefName": "y", "baseRefName": "x", "updatedAt": "2026-04-22T11:00:00Z"}
        ]"#;
        let prs = parse_pr_list(json, None).unwrap();
        let groups = group_stacks(prs);
        assert_eq!(groups.len(), 2);
        assert!(groups.iter().all(|g| !g.is_stack()));
    }

    #[test]
    fn parses_base_ref_from_list() {
        let prs = parse_pr_list(LIST_FIXTURE, None).unwrap();
        assert!(prs.iter().all(|p| p.base_ref == "main"));
    }

    #[test]
    fn rollup_empty_is_none() {
        let r = CheckRollup::from_raw(&[]);
        assert_eq!(r.overall, None);
    }

    #[test]
    fn rollup_fail_beats_pending() {
        let raws = vec![
            RawCheck {
                name: Some("a".into()),
                context: None,
                status: Some("COMPLETED".into()),
                conclusion: Some("FAILURE".into()),
                state: None,
            },
            RawCheck {
                name: Some("b".into()),
                context: None,
                status: Some("IN_PROGRESS".into()),
                conclusion: None,
                state: None,
            },
        ];
        assert_eq!(CheckRollup::from_raw(&raws).overall, Some(CheckState::Fail));
    }

    #[test]
    fn status_context_shape_is_parsed() {
        let raw = RawCheck {
            name: None,
            context: Some("legacy/ci".into()),
            status: None,
            conclusion: None,
            state: Some("SUCCESS".into()),
        };
        assert_eq!(raw.classify(), CheckState::Pass);
        assert_eq!(raw.label(), "legacy/ci");
    }
}
