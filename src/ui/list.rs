use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::app::{App, ListRow};
use crate::model::{PrStack, PrSummary};
use crate::ui::{checks_cell, format_age, my_review_cell, review_cell};

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    if app.loading_list && app.rows.is_empty() {
        let para = Paragraph::new("Loading PRs…").block(Block::default().borders(Borders::ALL));
        frame.render_widget(para, area);
        return;
    }

    if app.rows.is_empty() {
        let para = Paragraph::new("No open PRs.").block(Block::default().borders(Borders::ALL));
        frame.render_widget(para, area);
        return;
    }

    let header = Row::new(vec![
        "#", "Me", "R", "C", "Title", "Author", "Branch", "Age",
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app
        .rows
        .iter()
        .map(|row| match *row {
            ListRow::StackHeader(g) => stack_header_row(&app.groups[g], app.is_expanded(g)),
            ListRow::Pr { group, member } => {
                let stack = &app.groups[group];
                pr_row(
                    &stack.prs[member],
                    stack.is_stack(),
                    member + 1 == stack.prs.len(),
                )
            }
        })
        .collect();

    let widths = [
        Constraint::Length(6),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(20),
        Constraint::Length(14),
        Constraint::Length(24),
        Constraint::Length(6),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .block(Block::default().borders(Borders::ALL));

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn stack_header_row(stack: &PrStack, expanded: bool) -> Row<'static> {
    let marker = if expanded { "▾" } else { "▸" };
    let dim = if stack.all_drafts() {
        Style::default().add_modifier(Modifier::DIM)
    } else {
        Style::default()
    };
    let bottom = stack.bottom();
    let title = Line::from(vec![
        Span::styled(bottom.title.clone(), dim.add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("  (stack of {})", stack.prs.len()),
            Style::default().fg(Color::Cyan),
        ),
    ]);
    Row::new(vec![
        Cell::from(Span::styled(
            marker.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Cell::from(Line::from(my_review_cell(stack.my_review_rollup()))),
        Cell::from(Line::from(review_cell(stack.review_rollup()))),
        Cell::from(Line::from(checks_cell(&stack.checks_rollup()))),
        Cell::from(title),
        Cell::from(Span::styled(bottom.author.clone(), dim)),
        Cell::from(Span::styled(bottom.head_ref.clone(), dim)),
        Cell::from(Span::styled(format_age(stack.newest_update()), dim)),
    ])
}

fn pr_row(pr: &PrSummary, in_stack: bool, is_last_member: bool) -> Row<'static> {
    let num = if pr.is_draft {
        format!("{}*", pr.number)
    } else {
        pr.number.to_string()
    };
    let dim = if pr.is_draft {
        Style::default().add_modifier(Modifier::DIM)
    } else {
        Style::default()
    };
    let title = if in_stack {
        let glyph = if is_last_member { "└─ " } else { "├─ " };
        Line::from(vec![
            Span::styled(glyph, Style::default().fg(Color::Cyan)),
            Span::styled(pr.title.clone(), dim),
        ])
    } else {
        Line::from(Span::styled(pr.title.clone(), dim))
    };
    Row::new(vec![
        Cell::from(Span::styled(num, dim)),
        Cell::from(Line::from(my_review_cell(pr.my_review))),
        Cell::from(Line::from(review_cell(pr.review))),
        Cell::from(Line::from(checks_cell(&pr.checks))),
        Cell::from(title),
        Cell::from(Span::styled(pr.author.clone(), dim)),
        Cell::from(Span::styled(pr.head_ref.clone(), dim)),
        Cell::from(Span::styled(format_age(pr.updated_at), dim)),
    ])
}
