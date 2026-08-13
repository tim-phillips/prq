# prq

**prq** (_PR Queue_) is a terminal UI for browsing the open pull requests on a GitHub repo and seeing, at a glance, which ones need your attention. It shells out to the GitHub CLI (`gh`) so it uses the same auth and repo context you already have.

## Requirements

- [GitHub CLI](https://cli.github.com) installed and on `PATH`, signed in via `gh auth login`.
- The current directory must be inside a git repo that `gh` can resolve to a GitHub remote (i.e. `gh repo view` works there).
- A Rust toolchain to build from source.

## Install

```sh
cargo install --path .
```

Or build and run directly:

```sh
cargo run --release
```

## Usage

Run it from inside the repo you want to inspect:

```sh
prq
```

On startup it verifies `gh` is installed, resolves the repo with `gh repo view`, and begins loading open PRs in the background.

### Flags

| Flag                        | Default | What it does                                     |
| --------------------------- | ------- | ------------------------------------------------ |
| `--refresh-interval <SECS>` | `60`    | How often the list auto-refreshes.               |
| `--no-auto-refresh`         | off     | Disable periodic refresh; only `r` will refresh. |
| `--limit <N>`               | `100`   | Maximum number of PRs to fetch per listing.      |

Example:

```sh
prq --limit 30 --refresh-interval 30
```

## The interface

The screen is split into three zones:

```
prq · owner/repo · 12 open PR(s) · refreshed 14s ago               ← header
┌───────────────────────────────────────────────────────────────┐
│  #    Me  R  C  Title                       Author  Branch    │  ← body:
│  101      ✓  ✓  Refactor auth middleware    alice   auth-rfc  │    list or
│  102  …   ✗  ✗  Fix null deref in exporter  bob     fix-null  │    detail
│  103  !   ●  ●  Add support for foo         carol   feat-foo  │
│  104*     ∅  ∅  WIP: experimental cache     dave    cache-wip │
└───────────────────────────────────────────────────────────────┘
 j/k move · g/G top/bottom · Enter detail · o open · r refresh  ← footer
```

- **Header** — repo, PR count, attention counts (how many PRs are waiting on your review `!`, have changes requested on your PRs `✗`, have merge conflicts `↯`, or are ready to merge `»` — zero counts are hidden), and how long ago the data was refreshed.
- **Body** — the list of open PRs, or the detail view for a selected PR.
- **Footer** — context-sensitive keybindings, or the most recent error.

### List view

The list shows one row per open PR with these columns:

| Col      | Meaning                                                                                                    |
| -------- | ---------------------------------------------------------------------------------------------------------- |
| `#`      | PR number. A trailing `*` and dim style means the PR is a draft. `▸`/`▾` marks a collapsed/expanded stack. |
| `Me`     | What the PR needs from **you** — as reviewer or author (see icons below).                                  |
| `R`      | Overall review decision on the PR.                                                                         |
| `C`      | Overall CI/check status rollup.                                                                            |
| `Title`  | PR title.                                                                                                  |
| `Author` | PR author's GitHub login.                                                                                  |
| `Branch` | Head branch name.                                                                                          |
| `Age`    | Time since the PR was last updated.                                                                        |

#### Stacked PRs

Stacks of dependent PRs (as created by [gh-stack](https://github.com/github/gh-stack) or similar tools) are detected by base branch: a PR whose base branch is another open PR's head branch belongs to the same stack. Each stack collapses into a single `▸` row so a long stack doesn't flood the list.

The collapsed row shows the bottom PR's title with `(stack of N)`, and its `Me`/`R`/`C` columns roll up the most urgent state across the whole stack (e.g. one failing check anywhere shows `✗`; a review request for you on any member shows `!`). The age is the most recent update in the stack.

Press `Enter` on a stack row to expand it in place — members appear beneath it, ordered bottom (closest to trunk) to top, and behave like normal rows. `Enter` on the header again collapses it. `o` on a collapsed stack opens the bottom PR in the browser. Expansion state is preserved across refreshes.

#### `Me` — does this PR need your attention?

| Icon        | State                                                                    |
| ----------- | ------------------------------------------------------------------------ |
| `!`         | Review requested from you — you're blocking someone.                     |
| `✗`         | A reviewer requested changes on your PR — it needs rework.               |
| `↯`         | Your PR has merge conflicts.                                             |
| `»`         | Your PR is ready to merge: approved, checks green, merges cleanly.       |
| `○`         | Your PR, nothing to do yet — waiting on reviewers or checks, or a draft. |
| `…`         | You requested changes; now waiting on the author.                        |
| `✓`         | You already approved.                                                    |
| `·`         | You left a comment (no formal verdict).                                  |
| ` ` (blank) | You're not involved.                                                     |

#### `R` — review status

| Icon | State                         |
| ---- | ----------------------------- |
| `✓`  | Approved.                     |
| `✗`  | Changes requested.            |
| `●`  | Review required (pending).    |
| `·`  | Reviewed without a decision.  |
| `∅`  | No review state (e.g. draft). |

#### `C` — checks status

| Icon | State                                        |
| ---- | -------------------------------------------- |
| `✓`  | All completed checks passed.                 |
| `✗`  | At least one check failed.                   |
| `●`  | Checks still running.                        |
| `∅`  | No checks, or none reporting a useful state. |

### Detail view

Pressing `Enter` on a PR opens a detail view with:

- **overview** — number, title, draft flag, head → base refs, author, mergeability (mergeable / conflicting / unknown), diff size (`+additions −deletions in N files`), and the PR URL.
- **description** — the PR body, word-wrapped.
- **checks** — per-check status with name and state icon.
- **reviewers** — every requested or responded reviewer, with their state: `✓` approved, `✗` requested changes, `·` commented, `●` pending.

`Esc` (or `q`) returns to the list.

## Keybindings

### Global

| Key      | Action                                                                                                            |
| -------- | ----------------------------------------------------------------------------------------------------------------- |
| `?`      | Toggle help overlay.                                                                                              |
| `o`      | Open the selected PR (list) or current PR (detail) in the browser.                                                |
| `w`      | Check out the PR branch in [workmux](https://github.com/raine/workmux) (`workmux add --pr <n> --open-if-exists`). |
| `r`      | Refresh now — refetches the list or the open detail.                                                              |
| `q`      | Back to list (in detail), or quit (in list).                                                                      |
| `Ctrl-C` | Quit from anywhere.                                                                                               |

### List view

| Key          | Action                                                            |
| ------------ | ----------------------------------------------------------------- |
| `j` / `↓`    | Select next PR.                                                   |
| `k` / `↑`    | Select previous PR.                                               |
| `g` / `Home` | Jump to first PR.                                                 |
| `G` / `End`  | Jump to last PR.                                                  |
| `Enter`      | Open detail view for the selected PR, or expand/collapse a stack. |

### Detail view

| Key   | Action              |
| ----- | ------------------- |
| `Esc` | Return to the list. |

## Refresh behavior

- On startup the list is fetched once.
- Unless `--no-auto-refresh` is set, the list is refetched every `--refresh-interval` seconds. Your current selection is preserved across refreshes by PR number.
- Refreshes run on a background thread — the UI never blocks. While loading, the header shows `loading…` or stays on the previous data.
- `r` forces an immediate refresh of whichever view you're in.
- Detail data is fetched on demand when you open a PR and is never auto-refreshed; press `r` inside detail view to pull fresh data.

## Troubleshooting

- **``failed to invoke `gh` ``/`` `gh --version` failed ``** — the GitHub CLI isn't installed or isn't on `PATH`. Install it from <https://cli.github.com>.
- **`could not resolve repo from current directory`** — run `prq` inside a git repo that has a GitHub remote, and make sure `gh auth login` has been completed.
- **Footer shows a red `error: ...`** — the most recent `gh` invocation failed. The previous data stays on screen; press `r` to retry.
