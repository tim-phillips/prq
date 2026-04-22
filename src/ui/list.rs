use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::app::App;
use crate::ui::{checks_cell, format_age, my_review_cell, review_cell};

pub fn draw(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    if app.loading_list && app.prs.is_empty() {
        let para = Paragraph::new("Loading PRs…").block(Block::default().borders(Borders::ALL));
        frame.render_widget(para, area);
        return;
    }

    if app.prs.is_empty() {
        let para = Paragraph::new("No open PRs.").block(Block::default().borders(Borders::ALL));
        frame.render_widget(para, area);
        return;
    }

    let header = Row::new(vec!["#", "Me", "R", "C", "Title", "Author", "Branch", "Age"])
        .style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app
        .prs
        .iter()
        .map(|pr| {
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
            Row::new(vec![
                Cell::from(Span::styled(num, dim)),
                Cell::from(Line::from(my_review_cell(pr.my_review))),
                Cell::from(Line::from(review_cell(pr.review))),
                Cell::from(Line::from(checks_cell(&pr.checks))),
                Cell::from(Span::styled(pr.title.clone(), dim)),
                Cell::from(Span::styled(pr.author.clone(), dim)),
                Cell::from(Span::styled(pr.head_ref.clone(), dim)),
                Cell::from(Span::styled(format_age(pr.updated_at), dim)),
            ])
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
