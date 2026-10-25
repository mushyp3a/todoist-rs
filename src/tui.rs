use std::io::{self, stdout, Stdout};

use crossterm::{execute, terminal::*};
use ratatui::prelude::*;

mod bars;
mod help;
mod list;
mod popups;
mod sidebar;
mod utils;
mod whichkey;

use crate::app::{App, Mode, Overlay};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> io::Result<Tui> {
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

pub fn restore() -> io::Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

pub fn ui(f: &mut Frame, app: &mut App) {
    let [main, modeline, echo] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(f.area());

    // The sidebar gives way on narrow terminals.
    if app.show_sidebar && main.width >= 60 {
        let [sidebar_area, list_area] =
            Layout::horizontal([Constraint::Length(26), Constraint::Min(1)]).areas(main);
        sidebar::render(f, sidebar_area, app);
        list::render(f, list_area, app);
    } else {
        list::render(f, main, app);
    }

    bars::modeline(f, modeline, app);
    bars::echo_area(f, echo, app);

    match &mut app.mode {
        Mode::Edit(edit) => popups::edit(f, edit),
        Mode::Picker(picker) => popups::picker(f, picker),
        Mode::Overlay(Overlay::Help) => help::help(f, app.overlay_scroll),
        Mode::Overlay(Overlay::Messages) => help::messages(f, &app.log, app.overlay_scroll),
        Mode::Prompt(prompt) => {
            if let Some(suggest) = &prompt.suggest {
                popups::suggestions(f, echo, prompt.label(), suggest);
            }
        }
        Mode::Normal => {}
    }

    whichkey::render(f, main, app);
}
