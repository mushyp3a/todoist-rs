use std::sync::mpsc::Sender;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use reqwest::Client;

use crate::{
    api_calls::{
        close_task, create_task, delete_task, reopen_task, spawn_call, spawn_task_call, update_task,
    },
    new_task,
    task_edit::{self, CurrentlyEditing},
    tasks::{Filter, SortCriterion, Task},
    App, CurrentFocus, TaskResult, UndoAction,
};

/// Handles a key press on whichever screen is showing. Returns `true` to quit.
pub fn handle_key(app: &mut App, key: KeyEvent, client: &Client, tx: &Sender<TaskResult>) -> bool {
    if app.show_task_editor {
        handle_task_editor(app, key, client, tx);
    } else if app.show_new_task {
        handle_new_task(app, key, client, tx);
    } else if app.show_error {
        if key.code == KeyCode::Esc {
            app.clear_error();
        }
    } else {
        return handle_main_screen(app, key, client, tx);
    }
    false
}

fn handle_main_screen(
    app: &mut App,
    key: KeyEvent,
    client: &Client,
    tx: &Sender<TaskResult>,
) -> bool {
    match key.code {
        KeyCode::Char('h') => app.show_help = !app.show_help,
        KeyCode::Char('q') => return true,
        KeyCode::Char('t') => show_filter(app, Filter::Today),
        KeyCode::Char('o') => show_filter(app, Filter::Overdue),
        KeyCode::Char('a') => show_filter(app, Filter::All),
        KeyCode::Char('p') => app.tasks.sort_tasks(SortCriterion::Priority),
        KeyCode::Char('s') => app.tasks.sort_tasks(SortCriterion::Date),
        _ => {}
    }

    if app.show_help {
        return false;
    }

    if key.code == KeyCode::Tab {
        app.current_focus = match app.current_focus {
            CurrentFocus::Projects => CurrentFocus::Tasks,
            CurrentFocus::Tasks => CurrentFocus::Projects,
            _ => return false,
        };
        return false;
    }

    match app.current_focus {
        CurrentFocus::Projects => handle_projects(app, key),
        CurrentFocus::Tasks => handle_tasks(app, key, client, tx),
        _ => {}
    }
    false
}

fn show_filter(app: &mut App, filter: Filter) {
    app.tasks.filter = filter;
    app.tasks.filter_task_list();
    app.projects.unselect();
}

fn is_newline_chord(key: &KeyEvent) -> bool {
    (key.code == KeyCode::Enter && key.modifiers.contains(KeyModifiers::ALT))
        || (key.code == KeyCode::Char('n') && key.modifiers.contains(KeyModifiers::CONTROL))
}

fn is_back_tab(key: &KeyEvent) -> bool {
    key.code == KeyCode::BackTab
        || (key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::SHIFT))
}

/// Index into `app.tasks.tasks` of the highlighted task.
fn selected_task_index(app: &App) -> Option<usize> {
    let selected = app.tasks.state.selected()?;
    app.tasks.display_tasks.get(selected).copied()
}

fn open_task_editor(app: &mut App, index: usize) {
    let task = &app.tasks.tasks[index];
    let children = app
        .tasks
        .tasks
        .iter()
        .enumerate()
        .filter(|(_, child)| child.parent_id.as_ref() == Some(&task.id))
        .map(|(i, _)| i)
        .collect();

    app.task_edit = task_edit::TaskEdit::new(
        task.content.clone(),
        task.description.clone(),
        task.due
            .as_ref()
            .map_or(String::new(), |d| d.string.clone()),
        children,
        index,
        CurrentlyEditing::Content,
    );
    app.show_task_editor = true;
}

fn open_new_task(app: &mut App, project_id: Option<String>, parent_id: Option<String>) {
    app.new_task = new_task::NewTask::new(project_id, parent_id);
    app.show_new_task = true;
}

/// Saves the edited task locally and on the server. Returns `false` (and
/// changes nothing) if the content was left blank.
fn save_task_edit(app: &mut App, client: &Client, tx: &Sender<TaskResult>) -> bool {
    let content = app.task_edit.content.lines().join("\n");
    if content.trim().is_empty() {
        return false;
    }
    let description = app.task_edit.description.lines().join("\n");
    let due_text = app.task_edit.due_string.lines().join("\n");
    let due_text = due_text.trim();

    let task = &mut app.tasks.tasks[app.task_edit.current_task_index];
    task.content = content.clone();
    task.description = description.clone();

    let mut body = serde_json::json!({ "content": content, "description": description });
    // Only send the due date when it was edited, so it isn't re-parsed (and
    // possibly changed) on every save.
    let old_due = task.due.as_ref().map_or("", |due| due.string.as_str());
    if due_text != old_due {
        body["due_string"] = if due_text.is_empty() {
            "no date"
        } else {
            due_text
        }
        .into();
    }

    let client = client.clone();
    let task_id = task.id.clone();
    spawn_task_call(tx.clone(), async move {
        update_task(&client, &task_id, body).await
    });
    true
}

fn handle_task_editor(app: &mut App, key: KeyEvent, client: &Client, tx: &Sender<TaskResult>) {
    let focus = app.task_edit.currently_editing;

    if is_newline_chord(&key) {
        if focus == CurrentlyEditing::Description {
            app.task_edit.description.insert_newline();
        }
        return;
    }
    if is_back_tab(&key) {
        app.task_edit.focus(focus.previous());
        return;
    }

    match key.code {
        KeyCode::Esc => app.show_task_editor = false,
        KeyCode::Tab => app.task_edit.focus(focus.next()),
        KeyCode::Enter => {
            if !save_task_edit(app, client, tx) {
                return;
            }
            app.show_task_editor = false;
            // With a sub-task highlighted, Enter saves this task and opens that one.
            if focus == CurrentlyEditing::ChildTasks {
                if let Some(selected) = app.task_edit.children_list_state.selected() {
                    let child = app.task_edit.children[selected];
                    open_task_editor(app, child);
                }
            }
        }
        _ => match focus {
            CurrentlyEditing::Content => {
                app.task_edit.content.input(key);
            }
            CurrentlyEditing::Description => {
                app.task_edit.description.input(key);
            }
            CurrentlyEditing::DueString => {
                app.task_edit.due_string.input(key);
            }
            CurrentlyEditing::ChildTasks => match key.code {
                KeyCode::Char('j') | KeyCode::Down => app.task_edit.next(),
                KeyCode::Char('k') | KeyCode::Up => app.task_edit.previous(),
                KeyCode::Char('n') => {
                    let task = &app.tasks.tasks[app.task_edit.current_task_index];
                    let (project_id, parent_id) = (task.project_id.clone(), task.id.clone());
                    app.show_task_editor = false;
                    open_new_task(app, Some(project_id), Some(parent_id));
                }
                _ => {}
            },
        },
    }
}

fn show_selected_project(app: &mut App) {
    if let Some(selected) = app.projects.state.selected() {
        let project_id = app.projects.projects[selected].id.clone();
        app.tasks.filter = Filter::ProjectId(project_id.clone());
        app.tasks.filter_task_list();
        app.projects.selected_project = Some(project_id);
    }
}

fn handle_projects(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.projects.next();
            show_selected_project(app);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.projects.previous();
            show_selected_project(app);
        }
        KeyCode::Char('+') | KeyCode::Char('n') => {
            let project_id = app
                .projects
                .state
                .selected()
                .map(|selected| app.projects.projects[selected].id.clone());
            open_new_task(app, project_id, None);
        }
        _ => {}
    }
}

/// Removes the highlighted task from the list, returning it.
fn take_selected_task(app: &mut App) -> Option<Task> {
    let index = selected_task_index(app)?;
    let task = app.tasks.tasks.remove(index);
    app.tasks.filter_task_list();
    Some(task)
}

fn handle_tasks(app: &mut App, key: KeyEvent, client: &Client, tx: &Sender<TaskResult>) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => app.tasks.next(),
        KeyCode::Char('k') | KeyCode::Up => app.tasks.previous(),
        KeyCode::Enter => {
            if let Some(index) = selected_task_index(app) {
                open_task_editor(app, index);
            }
        }
        KeyCode::Char('x') => {
            if let Some(task) = take_selected_task(app) {
                let (client, task_id) = (client.clone(), task.id.clone());
                app.undo_stack.push(UndoAction::Completed(task));
                spawn_call(
                    tx.clone(),
                    async move { close_task(&client, &task_id).await },
                );
            }
        }
        KeyCode::Char('d') => {
            if let Some(task) = take_selected_task(app) {
                let (client, task_id) = (client.clone(), task.id.clone());
                app.undo_stack.push(UndoAction::Deleted(task));
                spawn_call(
                    tx.clone(),
                    async move { delete_task(&client, &task_id).await },
                );
            }
        }
        KeyCode::Char('u') => undo(app, client, tx),
        KeyCode::Char('n') => {
            // Prefer the selected project, then the highlighted task's project;
            // with neither, the task goes to the Inbox.
            let project_id = app
                .projects
                .state
                .selected()
                .map(|selected| app.projects.projects[selected].id.clone())
                .or_else(|| {
                    selected_task_index(app).map(|i| app.tasks.tasks[i].project_id.clone())
                });
            open_new_task(app, project_id, None);
        }
        _ => {}
    }
}

fn undo(app: &mut App, client: &Client, tx: &Sender<TaskResult>) {
    let client = client.clone();
    match app.undo_stack.pop() {
        Some(UndoAction::Completed(task)) => {
            let task_id = task.id.clone();
            app.tasks.tasks.push(task);
            app.tasks.filter_task_list();
            spawn_call(
                tx.clone(),
                async move { reopen_task(&client, &task_id).await },
            );
        }
        Some(UndoAction::Deleted(task)) => {
            // Deleted tasks can't be restored, so recreate it; the new task
            // arrives through `tx` and is added to the list then.
            let mut body = serde_json::json!({
                "content": task.content,
                "description": task.description,
                "project_id": task.project_id,
                "labels": task.labels,
                "priority": task.priority,
            });
            if let Some(section_id) = task.section_id {
                body["section_id"] = section_id.into();
            }
            if let Some(parent_id) = task.parent_id {
                body["parent_id"] = parent_id.into();
            }
            if let Some(due) = task.due {
                body["due_string"] = due.string.into();
            }
            if let Some(duration) = task.duration {
                body["duration"] = duration.amount.into();
                body["duration_unit"] = duration.unit.into();
            }
            spawn_task_call(tx.clone(), async move { create_task(&client, body).await });
        }
        None => {}
    }
}

fn handle_new_task(app: &mut App, key: KeyEvent, client: &Client, tx: &Sender<TaskResult>) {
    let focus = app.new_task.currently_editing;

    if is_newline_chord(&key) {
        if focus == new_task::CurrentlyEditing::Description {
            app.new_task.description.insert_newline();
        }
        return;
    }
    if is_back_tab(&key) {
        app.new_task.focus(focus.previous());
        return;
    }

    match key.code {
        KeyCode::Esc => app.show_new_task = false,
        KeyCode::Tab => app.new_task.focus(focus.next()),
        KeyCode::Enter => {
            // A task without content can't be created, so keep the prompt open.
            if let Some(body) = app.new_task.to_json() {
                app.show_new_task = false;
                let client = client.clone();
                spawn_task_call(tx.clone(), async move { create_task(&client, body).await });
            }
        }
        _ => {
            match focus {
                new_task::CurrentlyEditing::Content => app.new_task.content.input(key),
                new_task::CurrentlyEditing::Description => app.new_task.description.input(key),
                new_task::CurrentlyEditing::DueString => app.new_task.due_string.input(key),
            };
        }
    }
}
