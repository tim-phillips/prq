use anyhow::{Context, Result, bail};
use std::process::Command;

/// Check out the PR's head branch in a workmux worktree and open its tmux
/// window, reusing the worktree if one already exists.
pub fn open_pr(number: u32) -> Result<()> {
    let out = Command::new("workmux")
        .args(["add", "--pr", &number.to_string(), "--open-if-exists"])
        .output()
        .context("failed to invoke `workmux` — is it installed and on PATH?")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("`workmux add --pr {number}` failed: {}", stderr.trim());
    }
    Ok(())
}
