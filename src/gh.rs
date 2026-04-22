use anyhow::{Context, Result, bail};
use std::process::Command;

use crate::model::{PrDetail, PrSummary, parse_pr_detail, parse_pr_list};

const LIST_FIELDS: &str = "number,title,author,headRefName,isDraft,mergeable,updatedAt,url,reviewDecision,statusCheckRollup,reviewRequests,latestReviews";

const VIEW_FIELDS: &str = "number,title,body,author,headRefName,baseRefName,isDraft,mergeable,updatedAt,url,reviewDecision,statusCheckRollup,reviewRequests,latestReviews,additions,deletions,changedFiles";

pub fn check_gh_available() -> Result<()> {
    let out = Command::new("gh")
        .arg("--version")
        .output()
        .context("failed to invoke `gh` — install the GitHub CLI from https://cli.github.com")?;
    if !out.status.success() {
        bail!("`gh --version` failed; is the GitHub CLI installed and on PATH?");
    }
    Ok(())
}

pub fn check_repo_context() -> Result<String> {
    let out = Command::new("gh")
        .args(["repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"])
        .output()
        .context("failed to run `gh repo view`")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!(
            "could not resolve repo from current directory — run `prq` inside a repo linked to GitHub (gh auth login / git remote add origin ...).\n{stderr}"
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn viewer_login() -> Result<String> {
    let out = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()
        .context("failed to run `gh api user`")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("`gh api user` failed: {stderr}");
    }
    let login = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if login.is_empty() {
        bail!("`gh api user` returned an empty login");
    }
    Ok(login)
}

pub fn list_prs(limit: u32, viewer: Option<&str>) -> Result<Vec<PrSummary>> {
    let out = Command::new("gh")
        .args([
            "pr",
            "list",
            "--state",
            "open",
            "--limit",
            &limit.to_string(),
            "--json",
            LIST_FIELDS,
        ])
        .output()
        .context("failed to run `gh pr list`")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("`gh pr list` failed: {stderr}");
    }
    let stdout = std::str::from_utf8(&out.stdout).context("gh pr list: non-utf8 output")?;
    parse_pr_list(stdout, viewer).context("failed to parse `gh pr list` JSON")
}

pub fn view_pr(number: u32, viewer: Option<&str>) -> Result<PrDetail> {
    let num = number.to_string();
    let out = Command::new("gh")
        .args(["pr", "view", &num, "--json", VIEW_FIELDS])
        .output()
        .context("failed to run `gh pr view`")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("`gh pr view {number}` failed: {stderr}");
    }
    let stdout = std::str::from_utf8(&out.stdout).context("gh pr view: non-utf8 output")?;
    parse_pr_detail(stdout, viewer).context("failed to parse `gh pr view` JSON")
}
