use std::{
    collections::HashSet,
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

use chrono::{Local, NaiveDate};
use ratatui::widgets::ListState;
use reqwest::Client;

use crate::{
    api_calls,
    picker::Picker,
    projects::Project,
    prompt::Prompt,
    sections::Section,
    store::Store,
    task_edit::TaskEdit,
    tasks::Task,
    view::{build_rows, sidebar_entries, Row, RowQuery, SidebarEntry, Sort, View},
};

const SYNC_INTERVAL: Duration = Duration::from_secs(30);
const STATUS_LIFETIME: Duration = Duration::from_secs(6);
/// How long a key prefix waits before which-key appears.
pub const WHICH_KEY_DELAY: Duration = Duration::from_millis(350);

/// Results of background work, delivered to the UI loop.
pub enum Msg {
    Synced {
        projects: Vec<Project>,
        tasks: Vec<Task>,
        sections: Vec<Section>,
        /// The mutation count when the sync started; stale results are dropped.
        epoch: u64,
    },
    SyncFailed(String),
    Done {
        task: Option<Box<Task>>,
        note: Option<String>,
    },
    Failed {
        error: String,
        restore: Option<Box<Task>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Sidebar,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Overlay {
    Help,
    Messages,
}

pub enum Mode {
    Normal,
    Prompt(Prompt),
    Picker(Picker),
    Edit(Box<TaskEdit>),
    Overlay(Overlay),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Level {
    Info,
    Error,
}

pub struct Status {
    pub text: String,
    pub level: Level,
    pub at: Instant,
}

/// Something `u` can take back.
#[derive(Debug)]
pub enum Undo {
    Completed(Task),
    Deleted(Task),
    /// The task as it was before an edit.
    Changed(Task),
}

pub struct App {
    pub store: Store,
    pub view: View,
    pub sort: Sort,
    /// Active search; overrides the view while non-empty.
    pub search: String,
    pub rows: Vec<Row>,
    pub list_state: ListState,
    /// Id of the task under the cursor; the cursor follows it across rebuilds.
    pub selected: Option<String>,
    pub list_height: usize,
    pub focus: Focus,
    pub show_sidebar: bool,
    pub sidebar: Vec<SidebarEntry>,
    pub sidebar_state: ListState,
    pub folded: HashSet<String>,
    pub folded_projects: HashSet<String>,
    pub mode: Mode,
    pub overlay_scroll: u16,
    /// Keys typed so far of a multi-key binding.
    pub pending_keys: Vec<String>,
    pub pending_since: Option<Instant>,
    pub status: Option<Status>,
    pub log: Vec<(Level, String)>,
    pub undo: Vec<Undo>,
    pub client: Client,
    pub tx: Sender<Msg>,
    pub loaded: bool,
    pub syncing: bool,
    pub last_sync: Instant,
    /// Counts local changes, so a sync that started earlier can be discarded.
    pub epoch: u64,
    pub dirty: bool,
    pub quit: bool,
    started: Instant,
}

impl App {
    pub fn new(client: Client, tx: Sender<Msg>) -> App {
        App {
            store: Store::default(),
            view: View::Today,
            sort: Sort::Natural,
            search: String::new(),
            rows: Vec::new(),
            list_state: ListState::default(),
            selected: None,
            list_height: 20,
            focus: Focus::List,
            show_sidebar: true,
            sidebar: Vec::new(),
            sidebar_state: ListState::default(),
            folded: HashSet::new(),
            folded_projects: HashSet::new(),
            mode: Mode::Normal,
            overlay_scroll: 0,
            pending_keys: Vec::new(),
            pending_since: None,
            status: None,
            log: Vec::new(),
            undo: Vec::new(),
            client,
            tx,
            loaded: false,
            syncing: false,
            last_sync: Instant::now(),
            epoch: 0,
            dirty: true,
            quit: false,
            started: Instant::now(),
        }
    }

    /// Milliseconds since startup; drives the spinner.
    pub fn last_tick_ms(&self) -> u128 {
        self.started.elapsed().as_millis()
    }

    pub fn today(&self) -> NaiveDate {
        Local::now().date_naive()
    }

    // ---- status line -------------------------------------------------

    pub fn say(&mut self, text: impl Into<String>) {
        self.set_status(Level::Info, text.into());
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.set_status(Level::Error, text.into());
    }

    fn set_status(&mut self, level: Level, text: String) {
        self.log.push((level, text.clone()));
        self.status = Some(Status {
            text,
            level,
            at: Instant::now(),
        });
        self.dirty = true;
    }

    // ---- rows and selection ------------------------------------------

    /// Recomputes what is shown, keeping the cursor on the same task.
    pub fn rebuild(&mut self) {
        let old_index = self.list_state.selected().unwrap_or(0);
        let today = self.today();
        self.rows = build_rows(
            &self.store,
            &RowQuery {
                view: &self.view,
                search: &self.search,
                sort: self.sort,
                collapsed: &self.folded,
                today,
            },
        );
        self.sidebar = sidebar_entries(&self.store, &self.folded_projects, today);

        let same_task = self
            .selected
            .as_ref()
            .and_then(|id| self.rows.iter().position(|row| row.task_id() == Some(id)));
        let index = same_task.or_else(|| self.nearest_task_row(old_index));
        self.select_row(index);
        self.sync_sidebar_selection();
        self.dirty = true;
    }

    fn nearest_task_row(&self, from: usize) -> Option<usize> {
        let is_task = |i: &usize| self.rows[*i].task_id().is_some();
        let from = from.min(self.rows.len().saturating_sub(1));
        (from..self.rows.len())
            .find(is_task)
            .or_else(|| (0..from).rev().find(is_task))
    }

    fn select_row(&mut self, index: Option<usize>) {
        self.list_state.select(index);
        self.selected = index.and_then(|i| self.rows[i].task_id().map(str::to_string));
    }

    fn sync_sidebar_selection(&mut self) {
        let index = self.sidebar.iter().position(
            |entry| matches!(entry, SidebarEntry::Item { view, .. } if *view == self.view),
        );
        self.sidebar_state.select(index);
    }

    pub fn selected_task(&self) -> Option<&Task> {
        self.selected.as_ref().and_then(|id| self.store.task(id))
    }

    pub fn task_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.task_id().is_some())
            .count()
    }

    fn task_row_indices(&self) -> Vec<usize> {
        (0..self.rows.len())
            .filter(|&i| self.rows[i].task_id().is_some())
            .collect()
    }

    /// Moves the cursor `delta` tasks (headers are skipped), stopping at the ends.
    pub fn move_cursor(&mut self, delta: isize) {
        let indices = self.task_row_indices();
        if indices.is_empty() {
            return;
        }
        let current = self
            .list_state
            .selected()
            .and_then(|row| indices.iter().position(|&i| i == row))
            .unwrap_or(0) as isize;
        let target = (current + delta).clamp(0, indices.len() as isize - 1) as usize;
        self.select_row(Some(indices[target]));
    }

    pub fn cursor_to(&mut self, top: bool) {
        let indices = self.task_row_indices();
        let target = if top { indices.first() } else { indices.last() };
        if let Some(&row) = target {
            self.select_row(Some(row));
            if top {
                // Show the first header too.
                *self.list_state.offset_mut() = 0;
            }
        }
    }

    pub fn half_page(&mut self, down: bool) {
        let step = (self.list_height / 2).max(1) as isize;
        self.move_cursor(if down { step } else { -step });
    }

    /// Jumps to the first task of the next (or previous) section.
    pub fn jump_header(&mut self, forward: bool) {
        let current = self.list_state.selected().unwrap_or(0);
        let headers: Vec<usize> = (0..self.rows.len())
            .filter(|&i| matches!(self.rows[i], Row::Header { .. }))
            .collect();
        let target = if forward {
            headers.into_iter().find(|&i| i > current)
        } else {
            // The header of the current section is above the cursor; skip it.
            let mut above: Vec<usize> = headers.into_iter().filter(|&i| i < current).collect();
            above.pop();
            above.pop().or(Some(0))
        };
        if let Some(header) = target {
            if let Some(row) = (header..self.rows.len()).find(|&i| self.rows[i].task_id().is_some())
            {
                self.select_row(Some(row));
            }
        }
    }

    // ---- views -------------------------------------------------------

    pub fn set_view(&mut self, view: View) {
        self.view = view;
        self.search.clear();
        self.selected = None;
        self.list_state = ListState::default();
        self.rebuild();
    }

    pub fn move_sidebar(&mut self, delta: isize) {
        let items: Vec<usize> = (0..self.sidebar.len())
            .filter(|&i| matches!(self.sidebar[i], SidebarEntry::Item { .. }))
            .collect();
        if items.is_empty() {
            return;
        }
        let current = self
            .sidebar_state
            .selected()
            .and_then(|row| items.iter().position(|&i| i == row))
            .unwrap_or(0) as isize;
        let target = (current + delta).clamp(0, items.len() as isize - 1) as usize;
        if let SidebarEntry::Item { view, .. } = self.sidebar[items[target]].clone() {
            self.set_view(view);
        }
    }

    // ---- background messages -----------------------------------------

    pub fn handle_msg(&mut self, msg: Msg) {
        self.dirty = true;
        match msg {
            Msg::Synced {
                projects,
                tasks,
                sections,
                epoch,
            } => {
                self.syncing = false;
                self.last_sync = Instant::now();
                if self.loaded && epoch != self.epoch {
                    // Local changes happened since this sync started; its data is stale.
                    self.last_sync = Instant::now() - SYNC_INTERVAL + Duration::from_secs(2);
                    return;
                }
                if !self.loaded {
                    self.folded = tasks
                        .iter()
                        .filter(|task| task.is_collapsed)
                        .map(|task| task.id.clone())
                        .collect();
                    self.loaded = true;
                }
                self.store = Store {
                    tasks,
                    projects,
                    sections,
                };
                self.rebuild();
            }
            Msg::SyncFailed(error) => {
                self.syncing = false;
                self.last_sync = Instant::now();
                self.error(format!("Sync failed: {error}"));
            }
            Msg::Done { task, note } => {
                if let Some(task) = task {
                    // A task we haven't seen (just added) gets the cursor.
                    if self.store.task(&task.id).is_none() {
                        self.selected = Some(task.id.clone());
                    }
                    self.store.upsert(*task);
                    self.rebuild();
                }
                if let Some(note) = note {
                    self.say(note);
                }
            }
            Msg::Failed { error, restore } => {
                if let Some(task) = restore {
                    self.store.upsert(*task);
                    self.rebuild();
                }
                self.error(error);
                // Local state may now differ from the server's; fetch the truth.
                self.start_sync();
            }
        }
    }

    pub fn start_sync(&mut self) {
        if self.syncing {
            return;
        }
        self.syncing = true;
        let (client, tx, epoch) = (self.client.clone(), self.tx.clone(), self.epoch);
        tokio::spawn(async move {
            let loaded = tokio::try_join!(
                api_calls::fetch_projects(&client),
                api_calls::fetch_tasks(&client),
                api_calls::fetch_sections(&client),
            );
            let _ = tx.send(match loaded {
                Ok((projects, tasks, sections)) => Msg::Synced {
                    projects,
                    tasks,
                    sections,
                    epoch,
                },
                Err(error) => Msg::SyncFailed(format!("{error:#}")),
            });
        });
    }

    /// Periodic work: background refresh and expiring the status message.
    pub fn tick(&mut self) {
        let idle = matches!(self.mode, Mode::Normal);
        if self.loaded
            && idle
            && !self.syncing
            && api_calls::pending() == 0
            && self.last_sync.elapsed() >= SYNC_INTERVAL
        {
            self.start_sync();
        }
        if let Some(status) = &self.status {
            if status.at.elapsed() >= STATUS_LIFETIME {
                self.status = None;
                self.dirty = true;
            }
        }
    }

    /// True while something on screen changes without input (spinner, which-key).
    pub fn animating(&self) -> bool {
        self.syncing || api_calls::pending() > 0 || !self.pending_keys.is_empty()
    }
}
