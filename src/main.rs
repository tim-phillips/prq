mod app;
mod cli;
mod events;
mod gh;
mod model;
mod refresh;
mod ui;

use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;

use crate::app::{App, ViewMode};
use crate::cli::Cli;
use crate::events::AppEvent;
use crate::refresh::{Response, Worker};

fn main() -> Result<()> {
    let cli = Cli::parse();

    gh::check_gh_available()?;
    let repo = gh::check_repo_context()?;

    let mut terminal = init_terminal()?;
    install_panic_hook();

    let result = run_app(&mut terminal, repo, &cli);

    restore_terminal(&mut terminal)?;
    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    repo: String,
    cli: &Cli,
) -> Result<()> {
    let mut app = App::new(repo, cli.refresh_interval, !cli.no_auto_refresh, cli.limit);
    let worker = Worker::spawn();
    worker.request_list(app.limit);

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        match events::poll(Duration::from_millis(100))? {
            AppEvent::Key(k) => handle_key(&mut app, &worker, k),
            AppEvent::Tick => {}
        }

        drain_worker(&mut app, &worker);

        if app.needs_auto_refresh() {
            app.loading_list = true;
            worker.request_list(app.limit);
        }

        if app.should_quit {
            worker.shutdown();
            break;
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, worker: &Worker, key: KeyEvent) {
    if app.show_help {
        if matches!(
            key.code,
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q')
        ) {
            app.show_help = false;
        }
        return;
    }

    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => app.should_quit = true,
        (_, KeyCode::Char('q')) => {
            if matches!(app.mode, ViewMode::Detail(_)) {
                app.back_to_list();
            } else {
                app.should_quit = true;
            }
        }
        (_, KeyCode::Char('?')) => app.show_help = true,
        (_, KeyCode::Char('r')) => match app.mode {
            ViewMode::List => {
                if !app.loading_list {
                    app.loading_list = true;
                    worker.request_list(app.limit);
                }
            }
            ViewMode::Detail(n) => {
                if !app.loading_detail {
                    app.loading_detail = true;
                    worker.request_detail(n);
                }
            }
        },
        (_, KeyCode::Char('o')) => open_selected(app),
        (_, KeyCode::Esc) => {
            if matches!(app.mode, ViewMode::Detail(_)) {
                app.back_to_list();
            }
        }
        _ => match app.mode {
            ViewMode::List => handle_list_key(app, worker, key),
            ViewMode::Detail(_) => {}
        },
    }
}

fn handle_list_key(app: &mut App, worker: &Worker, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
        KeyCode::Char('g') | KeyCode::Home => app.select_first(),
        KeyCode::Char('G') | KeyCode::End => app.select_last(),
        KeyCode::Enter => {
            if app.toggle_selected_stack() {
                return;
            }
            if let Some(pr) = app.selected_pr() {
                let n = pr.number;
                app.enter_detail();
                worker.request_detail(n);
            }
        }
        _ => {}
    }
}

fn open_selected(app: &mut App) {
    let url = match app.mode {
        ViewMode::List => app.selected_url(),
        ViewMode::Detail(_) => app.detail.as_ref().map(|d| d.summary.url.clone()),
    };
    if let Some(url) = url {
        if let Err(e) = open::that(&url) {
            app.last_error = Some(format!("failed to open {url}: {e}"));
        }
    }
}

fn drain_worker(app: &mut App, worker: &Worker) {
    while let Ok(msg) = worker.rx.try_recv() {
        match msg {
            Response::List(Ok(prs)) => app.apply_prs(prs),
            Response::List(Err(e)) => app.apply_list_error(e),
            Response::Detail { number, result } => match result {
                Ok(d) => {
                    if matches!(app.mode, ViewMode::Detail(n) if n == number) {
                        app.apply_detail(d);
                    }
                }
                Err(e) => app.apply_detail_error(e),
            },
        }
    }
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).context("failed to construct terminal")?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();
    Ok(())
}

fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default(info);
    }));
}
