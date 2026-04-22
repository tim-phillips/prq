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
    #[serde(default, alias = "detailsUrl", alias = "targetUrl")]
    pub url: Option<String>,
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
    pub url: Option<String>,
}

impl From<&RawCheck> for CheckRun {
    fn from(raw: &RawCheck) -> Self {
        CheckRun {
            name: raw.label().to_string(),
            state: raw.classify(),
            url: raw.url.clone(),
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
    fn resolve(r: &RawPr, viewer: Option<&str>) -> Self {
        let Some(viewer) = viewer else {
            return MyReviewState::NotInvolved;
        };
        if r.review_requests.iter().any(|req| req.login == viewer) {
            return MyReviewState::ReviewRequested;
        }
        let mine = r
            .latest_reviews
            .iter()
            .find(|rv| rv.author.login == viewer);
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
    #[serde(default, rename = "createdAt")]
    created_at: Option<DateTime<Utc>>,
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
    pub created_at: Option<DateTime<Utc>>,
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
            created_at: r.created_at,
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

pub fn parse_pr_list(json: &str, viewer: Option<&str>) -> anyhow::Result<Vec<PrSummary>> {
    let raws: Vec<RawPr> = serde_json::from_str(json)?;
    Ok(raws
        .iter()
        .map(|r| PrSummary::from_raw(r, viewer))
        .collect())
}

pub fn parse_pr_detail(json: &str, viewer: Option<&str>) -> anyhow::Result<PrDetail> {
    let raw: RawPr = serde_json::from_str(json)?;
    Ok(PrDetail::from_raw(&raw, viewer))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST_FIXTURE: &str = include_str!("../tests/fixtures/pr_list.json");
    const VIEW_FIXTURE: &str = include_str!("../tests/fixtures/pr_view.json");

    #[test]
    fn parses_list_fixture() {
        let prs = parse_pr_list(LIST_FIXTURE, Some("tim")).unwrap();
        assert_eq!(prs.len(), 4);

        let approved = &prs[0];
        assert_eq!(approved.number, 101);
        assert_eq!(approved.review, ReviewState::Approved);
        assert_eq!(approved.checks.overall, Some(CheckState::Pass));
        assert_eq!(approved.checks.passing, 2);
        assert_eq!(approved.author, "alice");
        assert!(!approved.is_draft);
        assert_eq!(approved.mergeable, Mergeable::Mergeable);
        assert_eq!(approved.my_review, MyReviewState::Approved);

        let changes = &prs[1];
        assert_eq!(changes.review, ReviewState::ChangesRequested);
        assert_eq!(changes.checks.overall, Some(CheckState::Fail));
        assert_eq!(changes.checks.failing, 1);
        assert_eq!(changes.checks.passing, 1);
        assert_eq!(changes.my_review, MyReviewState::WaitingOnAuthor);

        let pending = &prs[2];
        assert_eq!(pending.review, ReviewState::ReviewRequired);
        assert_eq!(pending.checks.overall, Some(CheckState::Pending));
        assert!(pending.checks.pending >= 1);
        assert_eq!(pending.my_review, MyReviewState::ReviewRequested);

        let draft = &prs[3];
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
                url: None,
            },
            RawCheck {
                name: Some("b".into()),
                context: None,
                status: Some("IN_PROGRESS".into()),
                conclusion: None,
                state: None,
                url: None,
            },
        ];
        assert_eq!(
            CheckRollup::from_raw(&raws).overall,
            Some(CheckState::Fail)
        );
    }

    #[test]
    fn status_context_shape_is_parsed() {
        let raw = RawCheck {
            name: None,
            context: Some("legacy/ci".into()),
            status: None,
            conclusion: None,
            state: Some("SUCCESS".into()),
            url: Some("https://example.com".into()),
        };
        assert_eq!(raw.classify(), CheckState::Pass);
        assert_eq!(raw.label(), "legacy/ci");
    }
}
