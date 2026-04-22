use chrono::{DateTime, Utc};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, ViewMode};
use crate::model::{CheckRollup, CheckState, MyReviewState, ReviewState};

pub mod detail;
pub mod list;

pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let size = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(1),    // body
            Constraint::Length(1), // footer
        ])
        .split(size);

    draw_header(frame, chunks[0], app);
    match app.mode {
        ViewMode::List => list::draw(frame, chunks[1], app),
        ViewMode::Detail(_) => detail::draw(frame, chunks[1], app),
    }
    draw_footer(frame, chunks[2], app);

    if app.show_help {
        draw_help_overlay(frame, size);
    }
}

fn draw_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let count = app.prs.len();
    let refreshed = match app.last_refresh {
        Some(t) => format!("refreshed {} ago", format_elapsed(t.elapsed())),
        None if app.loading_list => "loading…".to_string(),
        None => "—".to_string(),
    };
    let title = format!("workstatus · {} · {count} open PR(s) · {refreshed}", app.repo);
    let para = Paragraph::new(title).style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(para, area);
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut spans: Vec<Span> = Vec::new();
    if let Some(err) = &app.last_error {
        spans.push(Span::styled(
            format!("error: {err}"),
            Style::default().fg(Color::Red).add_modifier(Modifier::DIM),
        ));
    } else {
        let keys = match app.mode {
            ViewMode::List => "j/k move · g/G top/bottom · Enter detail · o open · r refresh · ? help · q quit",
            ViewMode::Detail(_) => "Esc back · o open · r refresh · ? help · q quit",
        };
        spans.push(Span::styled(keys, Style::default().add_modifier(Modifier::DIM)));
    }
    let para = Paragraph::new(Line::from(spans));
    frame.render_widget(para, area);
}

fn draw_help_overlay(frame: &mut Frame<'_>, area: Rect) {
    let popup = centered_rect(60, 50, area);
    frame.render_widget(Clear, popup);
    let text = vec![
        Line::from(Span::styled(
            "Keybindings",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("j / ↓        select next PR"),
        Line::from("k / ↑        select previous PR"),
        Line::from("g            jump to top"),
        Line::from("G            jump to bottom"),
        Line::from("Enter        open PR detail view"),
        Line::from("Esc          back to list"),
        Line::from("o            open PR in browser"),
        Line::from("r            refresh now"),
        Line::from("?            toggle this help"),
        Line::from("q / Ctrl-C   quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Me column",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("!            review requested from you"),
        Line::from("…            you requested changes, waiting on author"),
        Line::from("✓            you approved"),
        Line::from("·            you commented"),
    ];
    let para = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" help "))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, popup);
}

fn centered_rect(pct_x: u16, pct_y: u16, area: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(vert[1])[1]
}

pub fn my_review_cell(state: MyReviewState) -> Span<'static> {
    match state {
        MyReviewState::ReviewRequested => {
            Span::styled("!", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        }
        MyReviewState::WaitingOnAuthor => Span::styled("…", Style::default().fg(Color::Magenta)),
        MyReviewState::Approved => Span::styled(
            "✓",
            Style::default().fg(Color::Green).add_modifier(Modifier::DIM),
        ),
        MyReviewState::Commented => Span::styled(
            "·",
            Style::default().fg(Color::Blue).add_modifier(Modifier::DIM),
        ),
        MyReviewState::NotInvolved => Span::raw(" "),
    }
}

pub fn review_cell(state: ReviewState) -> Span<'static> {
    match state {
        ReviewState::Approved => Span::styled("✓", Style::default().fg(Color::Green)),
        ReviewState::ChangesRequested => Span::styled("✗", Style::default().fg(Color::Red)),
        ReviewState::ReviewRequired => Span::styled("●", Style::default().fg(Color::Yellow)),
        ReviewState::Reviewed => Span::styled("·", Style::default().fg(Color::Blue)),
        ReviewState::None => Span::styled("∅", Style::default().add_modifier(Modifier::DIM)),
    }
}

pub fn checks_cell(rollup: &CheckRollup) -> Span<'static> {
    match rollup.overall {
        Some(CheckState::Pass) => Span::styled("✓", Style::default().fg(Color::Green)),
        Some(CheckState::Fail) => Span::styled("✗", Style::default().fg(Color::Red)),
        Some(CheckState::Pending) => Span::styled("●", Style::default().fg(Color::Yellow)),
        Some(CheckState::None) | None => {
            Span::styled("∅", Style::default().add_modifier(Modifier::DIM))
        }
    }
}

pub fn check_state_span(state: CheckState) -> Span<'static> {
    match state {
        CheckState::Pass => Span::styled("✓", Style::default().fg(Color::Green)),
        CheckState::Fail => Span::styled("✗", Style::default().fg(Color::Red)),
        CheckState::Pending => Span::styled("●", Style::default().fg(Color::Yellow)),
        CheckState::None => Span::styled("∅", Style::default().add_modifier(Modifier::DIM)),
    }
}

pub fn format_age(ts: Option<DateTime<Utc>>) -> String {
    match ts {
        None => "—".to_string(),
        Some(t) => {
            let now = Utc::now();
            let delta = now.signed_duration_since(t);
            if delta.num_seconds() < 0 {
                return "just now".to_string();
            }
            let dur = std::time::Duration::from_secs(delta.num_seconds() as u64);
            format_elapsed(dur)
        }
    }
}

pub fn format_elapsed(dur: std::time::Duration) -> String {
    let secs = dur.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86_400)
    }
}
