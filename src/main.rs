use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    time::Duration,
};

use color_eyre::{eyre::Context, Result};
use crossterm::event::{self, KeyEventKind};
use new_task::NewTask;
use projects::Projects;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use sections::Sections;
use tasks::{Task, Tasks};
use tokio::sync::Mutex;

mod api_calls;
mod banner;
mod config;
mod error;
mod key_handler;
mod new_task;
mod projects;
mod sections;
mod task_edit;
mod tasks;
mod tui;

#[derive(Debug, Default, PartialEq)]
pub enum CurrentFocus {
    #[default]
    Projects,
    Tasks,
    Help,
    Error,
}

/// A removed task that `u` can bring back.
#[derive(Debug)]
pub enum UndoAction {
    Completed(Task),
    Deleted(Task),
}

#[derive(Debug, Default)]
pub struct App<'a> {
    pub projects: Projects,
    pub current_focus: CurrentFocus,
    pub tasks: Tasks,
    pub show_help: bool,
    pub sections: Sections,
    pub show_task_editor: bool,
    pub task_edit: task_edit::TaskEdit<'a>,
    pub show_new_task: bool,
    pub new_task: NewTask<'a>,
    pub error_message: Option<String>,
    pub show_error: bool,
    pub undo_stack: Vec<UndoAction>,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        App::default()
    }

    pub fn set_error_message(&mut self, message: String) {
        self.error_message = Some(message);
        self.show_error = true;
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
        self.show_error = false;
    }

    /// Applies the outcome of a background API call.
    fn apply(&mut self, result: TaskResult) {
        match result {
            TaskResult::Task(task) => {
                match self.tasks.tasks.iter_mut().find(|t| t.id == task.id) {
                    Some(existing) => *existing = *task,
                    None => self.tasks.tasks.push(*task),
                }
                self.tasks.find_tasks_with_children();
                self.tasks.filter_task_list();
            }
            TaskResult::Error(message) => self.set_error_message(message),
        }
    }
}

/// Sent from background API calls back to the UI loop.
#[derive(Debug, Clone)]
pub enum TaskResult {
    Task(Box<Task>),
    Error(String),
}

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
    let (tx, rx): (Sender<TaskResult>, Receiver<TaskResult>) = mpsc::channel();
    let client = build_client(&config::load_token()?)?;

    error::install_hooks()?;
    let mut terminal = tui::init()?;
    let app = Arc::new(Mutex::new(App::new()));

    // Load everything in the background; the UI is usable (but empty) meanwhile.
    let (app_clone, client_clone, tx_clone) = (Arc::clone(&app), client.clone(), tx.clone());
    tokio::spawn(async move {
        let loaded = tokio::try_join!(
            api_calls::fetch_projects(&client_clone),
            api_calls::fetch_tasks(&client_clone),
            api_calls::fetch_sections(&client_clone),
        );
        match loaded {
            Ok((projects, tasks, sections)) => {
                let mut app = app_clone.lock().await;
                app.projects = Projects::new(projects);
                app.tasks = Tasks::new(tasks);
                app.sections = Sections::new(sections);
                app.tasks.filter_task_list();
                app.tasks.find_tasks_with_children();
            }
            Err(error) => {
                let _ = tx_clone.send(TaskResult::Error(format!("{error:#}")));
            }
        }
    });

    loop {
        let mut app = app.lock().await;
        terminal.draw(|frame| tui::ui(frame, &mut app))?;

        if event::poll(Duration::from_millis(150))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && key_handler::handle_key(&mut app, key, &client, &tx)
                {
                    break;
                }
            }
        }

        while let Ok(result) = rx.try_recv() {
            app.apply(result);
        }
    }

    tui::restore()?;
    Ok(())
}
