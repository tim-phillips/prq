use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::{App, ViewMode};
use crate::model::{Mergeable, PrDetail, ReviewerState};
use crate::ui::{check_state_span, format_age};

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    let number = match app.mode {
        ViewMode::Detail(n) => n,
        _ => return,
    };

    if app.loading_detail && app.detail.is_none() {
        let para = Paragraph::new(format!("Loading PR #{number}…"))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(para, area);
        return;
    }

    let Some(detail) = &app.detail else {
        let para =
            Paragraph::new("No detail available.").block(Block::default().borders(Borders::ALL));
        frame.render_widget(para, area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(3),
            Constraint::Length(10),
        ])
        .split(area);

    draw_meta(frame, chunks[0], detail);
    draw_body(frame, chunks[1], detail);
    draw_bottom(frame, chunks[2], detail);
}

fn draw_meta(frame: &mut Frame<'_>, area: Rect, d: &PrDetail) {
    let s = &d.summary;
    let mergeable = match s.mergeable {
        Mergeable::Mergeable => Span::styled("mergeable", Style::default().fg(Color::Green)),
        Mergeable::Conflicting => Span::styled("conflicting", Style::default().fg(Color::Red)),
        Mergeable::Unknown => Span::styled("unknown", Style::default().add_modifier(Modifier::DIM)),
    };
    let draft = if s.is_draft {
        Span::styled(" draft", Style::default().add_modifier(Modifier::DIM))
    } else {
        Span::raw("")
    };
    let title_line = Line::from(vec![
        Span::styled(
            format!("#{} ", s.number),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            s.title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        draft,
    ]);
    let refs = Line::from(format!("{} → {}", s.head_ref, s.base_ref));
    let meta = Line::from(vec![
        Span::raw(format!(
            "by {} · updated {} · ",
            s.author,
            format_age(s.updated_at)
        )),
        mergeable,
        Span::raw(format!(
            " · +{} −{} in {} files",
            d.additions, d.deletions, d.changed_files
        )),
    ]);
    let text = vec![title_line, refs, meta, Line::from(s.url.clone())];
    let para = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" overview "))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn draw_body(frame: &mut Frame<'_>, area: Rect, d: &PrDetail) {
    let inner_width = area.width.saturating_sub(2).max(20) as usize;
    let body = if d.body.trim().is_empty() {
        "(no description)".to_string()
    } else {
        textwrap::fill(&d.body, inner_width)
    };
    let para = Paragraph::new(body)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" description "),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn draw_bottom(frame: &mut Frame<'_>, area: Rect, d: &PrDetail) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let check_items: Vec<ListItem> = if d.checks.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "(no checks)",
            Style::default().add_modifier(Modifier::DIM),
        )))]
    } else {
        d.checks
            .iter()
            .map(|c| {
                ListItem::new(Line::from(vec![
                    check_state_span(c.state),
                    Span::raw(" "),
                    Span::raw(c.name.clone()),
                ]))
            })
            .collect()
    };
    let checks =
        List::new(check_items).block(Block::default().borders(Borders::ALL).title(" checks "));
    frame.render_widget(checks, cols[0]);

    let reviewer_items: Vec<ListItem> = if d.reviewers.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "(no reviewers)",
            Style::default().add_modifier(Modifier::DIM),
        )))]
    } else {
        d.reviewers
            .iter()
            .map(|r| {
                let icon = match r.state {
                    ReviewerState::Approved => Span::styled("✓", Style::default().fg(Color::Green)),
                    ReviewerState::ChangesRequested => {
                        Span::styled("✗", Style::default().fg(Color::Red))
                    }
                    ReviewerState::Commented => Span::styled("·", Style::default().fg(Color::Blue)),
                    ReviewerState::Pending => Span::styled("●", Style::default().fg(Color::Yellow)),
                };
                ListItem::new(Line::from(vec![
                    icon,
                    Span::raw(" "),
                    Span::raw(r.login.clone()),
                ]))
            })
            .collect()
    };
    let reviewers = List::new(reviewer_items)
        .block(Block::default().borders(Borders::ALL).title(" reviewers "));
    frame.render_widget(reviewers, cols[1]);
}
