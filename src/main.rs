use std::{sync::mpsc, time::Duration};

use app::App;
use color_eyre::{eyre::Context, Result};
use crossterm::event::{self, Event, KeyEventKind};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

mod actions;
mod api_calls;
mod app;
mod banner;
mod config;
mod error;
mod keymap;
mod keys;
mod picker;
mod projects;
mod prompt;
mod quick_add;
mod reschedule;
mod sections;
mod store;
mod task_edit;
mod tasks;
mod tui;
mod view;

fn build_client(token: &str) -> Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).wrap_err("The API token is invalid")?,
    );
    Ok(reqwest::Client::builder()
        .default_headers(headers)
        .build()?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel();
    let client = build_client(&config::load_token()?)?;

    error::install_hooks()?;
    let mut terminal = tui::init()?;
    let mut app = App::new(client, tx);
    app.start_sync();

    while !app.quit {
        // Redraw only when something changed (or something is animating).
        if app.dirty || app.animating() {
            terminal.draw(|frame| tui::ui(frame, &mut app))?;
            app.dirty = false;
        }

        if event::poll(Duration::from_millis(80))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    keys::handle_key(&mut app, key)
                }
                Event::Resize(..) => app.dirty = true,
                _ => {}
            }
        }

        while let Ok(msg) = rx.try_recv() {
            app.handle_msg(msg);
        }
        app.tick();
    }

    tui::restore()?;
    Ok(())
}
