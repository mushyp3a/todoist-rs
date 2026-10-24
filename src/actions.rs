//! What each key binding does to the app.

use serde_json::{json, Value};

use crate::{
    api_calls::{
        close_task, create_task, delete_task, fetch_task, move_task, quick_add, reopen_task,
        spawn_op, update_task, MoveTarget,
    },
    app::{App, Focus, Mode, Overlay, Undo},
    keymap::Action,
    picker::{PickItem, PickTarget, Picker, PickerKind},
    prompt::{Prompt, PromptKind},
    task_edit::{parse_labels, TaskEdit},
    tasks::{api_priority, Due, Task},
    view::{ordered_projects, Row, SidebarEntry, Sort, View},
};

impl App {
    pub fn run(&mut self, action: Action) {
        use Action::*;
        match action {
            Down => self.vertical(1),
            Up => self.vertical(-1),
            Top => match self.focus {
                Focus::List => self.cursor_to(true),
                Focus::Sidebar => self.move_sidebar(-1000),
            },
            Bottom => match self.focus {
                Focus::List => self.cursor_to(false),
                Focus::Sidebar => self.move_sidebar(1000),
            },
            HalfPageDown => self.half_page(true),
            HalfPageUp => self.half_page(false),
            NextHeader => self.jump_header(true),
            PrevHeader => self.jump_header(false),
            Left => self.left(),
            Right => self.right(),
            Open => match self.focus {
                Focus::Sidebar => self.focus = Focus::List,
                Focus::List => self.edit(),
            },
            FocusSidebar => {
                self.show_sidebar = true;
                self.focus = Focus::Sidebar;
            }
            FocusList => self.focus = Focus::List,
            ToggleSidebar => {
                self.show_sidebar = !self.show_sidebar;
                if !self.show_sidebar {
                    self.focus = Focus::List;
                }
            }
            ToggleFold => self.toggle_fold(),
            Fold => self.set_fold(true),
            Unfold => self.set_fold(false),
            FoldAll => {
                self.folded = self
                    .rows
                    .iter()
                    .filter_map(|row| match row {
                        Row::Task(task) if task.has_children => Some(task.id.clone()),
                        _ => None,
                    })
                    .collect();
                self.rebuild();
            }
            UnfoldAll => {
                self.folded.clear();
                self.rebuild();
            }
            Edit => self.edit(),
            Complete => self.complete(),
            Delete => self.delete(),
            Undo => self.undo(),
            Priority(level) => self.set_priority(level),
            Postpone(days) => self.postpone(days),
            SetDue => self.prompt_for_task(|task| {
                (
                    PromptKind::Due(task.id.clone()),
                    task.due
                        .as_ref()
                        .map_or(String::new(), |due| due.string.clone()),
                )
            }),
            Rename => self.prompt_for_task(|task| {
                (PromptKind::Rename(task.id.clone()), task.content.clone())
            }),
            EditLabels => self.prompt_for_task(|task| {
                (PromptKind::Labels(task.id.clone()), task.labels.join(" "))
            }),
            Move => self.open_move_picker(),
            Add => {
                let project = match &self.view {
                    View::Project(id) => Some(id.clone()),
                    _ => None,
                };
                let kind = PromptKind::Add {
                    project,
                    parent: None,
                };
                self.mode = Mode::Prompt(Prompt::new(kind, ""));
            }
            AddSubtask => match self.selected_task() {
                Some(task) => {
                    let kind = PromptKind::Add {
                        project: Some(task.project_id.clone()),
                        parent: Some(task.id.clone()),
                    };
                    self.mode = Mode::Prompt(Prompt::new(kind, ""));
                }
                None => self.error("No task selected"),
            },
            Search => {
                let current = self.search.clone();
                self.mode = Mode::Prompt(Prompt::new(PromptKind::Search, &current));
            }
            Cancel => {
                if !self.search.is_empty() {
                    self.search.clear();
                    self.rebuild();
                }
            }
            GotoToday => self.set_view(View::Today),
            GotoUpcoming => self.set_view(View::Upcoming),
            GotoInbox => self.set_view(View::Inbox),
            GotoAll => self.set_view(View::All),
            GotoOverdue => self.set_view(View::Overdue),
            Jump => self.open_jump_picker(),
            Refresh => {
                if self.syncing {
                    self.say("Already refreshing");
                } else {
                    self.say("Refreshing…");
                    self.start_sync();
                }
            }
            SortNatural => self.set_sort(Sort::Natural),
            SortDate => self.set_sort(Sort::Date),
            SortPriority => self.set_sort(Sort::Priority),
            CycleSort => self.set_sort(self.sort.next()),
            Help => self.open_overlay(Overlay::Help),
            Messages => self.open_overlay(Overlay::Messages),
            Quit => self.quit = true,
        }
    }

    fn open_overlay(&mut self, overlay: Overlay) {
        self.overlay_scroll = 0;
        self.mode = Mode::Overlay(overlay);
    }

    fn set_sort(&mut self, sort: Sort) {
        self.sort = sort;
        self.say(format!("Sorted by {}", sort.label()));
        self.rebuild();
    }

    // ---- movement and folding ----------------------------------------

    fn vertical(&mut self, delta: isize) {
        match self.focus {
            Focus::List => self.move_cursor(delta),
            Focus::Sidebar => self.move_sidebar(delta),
        }
    }

    /// The project under the sidebar cursor, if it has sub-projects.
    fn sidebar_foldable(&self) -> Option<(String, bool)> {
        let entry = self.sidebar.get(self.sidebar_state.selected()?)?;
        match entry {
            SidebarEntry::Item {
                view: View::Project(id),
                foldable: true,
                folded,
                ..
            } => Some((id.clone(), *folded)),
            _ => None,
        }
    }

    fn set_project_fold(&mut self, id: String, fold: bool) {
        if fold {
            self.folded_projects.insert(id);
        } else {
            self.folded_projects.remove(&id);
        }
        self.rebuild();
    }

    /// `(has children, is folded)` for the task under the cursor.
    fn cursor_fold_state(&self) -> Option<(bool, bool)> {
        match self.rows.get(self.list_state.selected()?)? {
            Row::Task(row) => Some((row.has_children, row.collapsed)),
            Row::Header { .. } => None,
        }
    }

    fn left(&mut self) {
        match self.focus {
            Focus::List => match self.cursor_fold_state() {
                Some((true, false)) => self.set_fold(true),
                _ if self.show_sidebar => self.focus = Focus::Sidebar,
                _ => {}
            },
            Focus::Sidebar => {
                if let Some((id, false)) = self.sidebar_foldable() {
                    self.set_project_fold(id, true);
                }
            }
        }
    }

    fn right(&mut self) {
        match self.focus {
            Focus::List => self.set_fold(false),
            Focus::Sidebar => match self.sidebar_foldable() {
                Some((id, true)) => self.set_project_fold(id, false),
                _ => self.focus = Focus::List,
            },
        }
    }

    fn toggle_fold(&mut self) {
        match self.focus {
            Focus::List => {
                if let Some((true, folded)) = self.cursor_fold_state() {
                    self.set_fold(!folded);
                }
            }
            Focus::Sidebar => {
                if let Some((id, folded)) = self.sidebar_foldable() {
                    self.set_project_fold(id, !folded);
                }
            }
        }
    }

    fn set_fold(&mut self, fold: bool) {
        if self.focus != Focus::List {
            return;
        }
        let Some(id) = self.selected.clone() else {
            return;
        };
        if !matches!(self.cursor_fold_state(), Some((true, _))) {
            return;
        }
        if fold {
            self.folded.insert(id);
        } else {
            self.folded.remove(&id);
        }
        self.rebuild();
    }

    // ---- editing tasks -----------------------------------------------

    fn edit(&mut self) {
        match self.selected_task() {
            Some(task) => self.mode = Mode::Edit(Box::new(TaskEdit::new(task))),
            None => self.error("No task selected"),
        }
    }

    fn prompt_for_task(&mut self, make: impl FnOnce(&Task) -> (PromptKind, String)) {
        match self.selected_task().map(make) {
            Some((kind, initial)) => self.mode = Mode::Prompt(Prompt::new(kind, &initial)),
            None => self.error("No task selected"),
        }
    }

    /// Applies `change` to a task right away and sends `body` (and an optional
    /// move) to Todoist. If the server rejects it, the old task is restored.
    fn mutate(
        &mut self,
        id: &str,
        change: impl FnOnce(&mut Task),
        body: Value,
        move_to: Option<String>,
    ) {
        let Some(task) = self.store.task_mut(id) else {
            return;
        };
        let before = task.clone();
        change(task);
        self.undo.push(Undo::Changed(before.clone()));
        self.epoch += 1;
        self.rebuild();
        self.send_update(id.to_string(), body, move_to, before, None);
    }

    fn send_update(
        &self,
        id: String,
        body: Value,
        move_to: Option<String>,
        restore: Task,
        note: Option<String>,
    ) {
        let client = self.client.clone();
        spawn_op(self.tx.clone(), Some(restore), note, async move {
            let mut latest = None;
            if body.as_object().is_some_and(|fields| !fields.is_empty()) {
                latest = Some(update_task(&client, &id, body).await?);
            }
            if let Some(project) = move_to {
                latest = Some(move_task(&client, &id, MoveTarget::Project(project)).await?);
            }
            Ok(latest)
        });
    }

    fn set_priority(&mut self, level: u8) {
        let Some(task) = self.selected_task() else {
            return;
        };
        if task.ui_priority() == level {
            self.say(format!("Already priority {level}"));
            return;
        }
        let (id, api) = (task.id.clone(), api_priority(level));
        self.mutate(
            &id,
            |task| task.priority = api,
            json!({ "priority": api }),
            None,
        );
    }

    fn postpone(&mut self, days: i64) {
        let today = self.today();
        let Some(task) = self.selected_task() else {
            return;
        };
        let Some(due) = &task.due else {
            self.error("No due date — press D to set one");
            return;
        };
        if due.is_recurring {
            self.error("Recurring task — press D to edit its schedule");
            return;
        }
        let date = if days > 0 {
            due.date.max(today) + chrono::Duration::days(days)
        } else if due.date > today {
            due.date + chrono::Duration::days(days)
        } else {
            self.say("Already due today or earlier");
            return;
        };
        let new_due = Due::local(date, due.time);
        let id = task.id.clone();
        let body = json!({ "due_string": new_due.api_string() });
        self.mutate(&id, |task| task.due = Some(new_due), body, None);
    }

    fn open_move_picker(&mut self) {
        let Some(task) = self.selected_task() else {
            self.error("No task selected");
            return;
        };
        let (id, current) = (task.id.clone(), task.project_id.clone());
        let items = self
            .project_items()
            .into_iter()
            .filter(|(project_id, _)| *project_id != current)
            .map(|(project_id, label)| PickItem {
                label,
                target: PickTarget::Project(project_id),
            })
            .collect();
        self.mode = Mode::Picker(Picker::new(PickerKind::MoveTask(id), "Move to", items));
    }

    /// `(project id, "Parent / Child")` for every project.
    fn project_items(&self) -> Vec<(String, String)> {
        ordered_projects(&self.store)
            .into_iter()
            .map(|(project, _)| {
                let mut path = vec![project.name.as_str()];
                let mut parent = project.parent_id.as_deref();
                while let Some(project) = parent.and_then(|id| self.store.project(id)) {
                    path.push(project.name.as_str());
                    parent = project.parent_id.as_deref();
                }
                path.reverse();
                (project.id.clone(), path.join(" / "))
            })
            .collect()
    }

    fn open_jump_picker(&mut self) {
        let view_item = |label: &str, view: View| PickItem {
            label: label.to_string(),
            target: PickTarget::View(view),
        };
        let mut items = vec![
            view_item("Inbox", View::Inbox),
            view_item("Today", View::Today),
            view_item("Upcoming", View::Upcoming),
            view_item("All tasks", View::All),
            view_item("Overdue", View::Overdue),
        ];
        for (id, label) in self.project_items() {
            items.push(view_item(&label, View::Project(id)));
        }
        for label in self.store.labels() {
            items.push(view_item(&format!("@{label}"), View::Label(label)));
        }
        self.mode = Mode::Picker(Picker::new(PickerKind::Jump, "Jump to", items));
    }

    // ---- completing, deleting, undoing --------------------------------

    /// Removes a task and its sub-tasks from the list.
    fn remove_locally(&mut self, id: &str) {
        for descendant in self.store.descendants(id) {
            self.store.remove(&descendant);
        }
        self.store.remove(id);
        self.epoch += 1;
        self.rebuild();
    }

    fn complete(&mut self) {
        let Some(task) = self.selected_task().cloned() else {
            return;
        };
        let (id, recurring) = (task.id.clone(), task.is_recurring());
        self.remove_locally(&id);
        self.undo.push(Undo::Completed(task.clone()));

        let client = self.client.clone();
        let content = task.content.clone();
        spawn_op(self.tx.clone(), Some(task), None, async move {
            close_task(&client, &id).await?;
            // A recurring task isn't finished; Todoist moves it to its next date.
            if recurring {
                Ok(Some(fetch_task(&client, &id).await?))
            } else {
                Ok(None)
            }
        });
        self.say(format!("Completed “{content}” — u to undo"));
    }

    fn delete(&mut self) {
        let Some(task) = self.selected_task().cloned() else {
            return;
        };
        let id = task.id.clone();
        self.remove_locally(&id);
        self.undo.push(Undo::Deleted(task.clone()));

        let client = self.client.clone();
        let content = task.content.clone();
        spawn_op(self.tx.clone(), Some(task), None, async move {
            delete_task(&client, &id).await?;
            Ok(None)
        });
        self.say(format!("Deleted “{content}” — u to undo"));
    }

    fn undo(&mut self) {
        let client = self.client.clone();
        match self.undo.pop() {
            None => self.say("Nothing to undo"),
            Some(Undo::Completed(task)) => {
                if task.is_recurring() {
                    self.error("Can't undo completing a recurring task");
                    return;
                }
                let id = task.id.clone();
                self.store.upsert(task);
                self.epoch += 1;
                self.selected = Some(id.clone());
                self.rebuild();
                spawn_op(self.tx.clone(), None, Some("Reopened".into()), async move {
                    reopen_task(&client, &id).await?;
                    Ok(None)
                });
            }
            Some(Undo::Deleted(task)) => {
                // Deleted tasks can't be restored, so recreate it.
                let body = recreate_body(&task);
                self.epoch += 1;
                spawn_op(
                    self.tx.clone(),
                    None,
                    Some("Recreated (sub-tasks aren't restored)".into()),
                    async move { Ok(Some(create_task(&client, body).await?)) },
                );
            }
            Some(Undo::Changed(before)) => {
                let Some(current) = self.store.task(&before.id).cloned() else {
                    self.error("That task no longer exists");
                    return;
                };
                let (body, move_to) = restore_body(&before, &current);
                let id = before.id.clone();
                // Sub-tasks travel with their parent between projects.
                if let Some(project) = &move_to {
                    for descendant in self.store.descendants(&id) {
                        if let Some(task) = self.store.task_mut(&descendant) {
                            task.project_id = project.clone();
                        }
                    }
                }
                self.store.upsert(before);
                self.epoch += 1;
                self.selected = Some(id.clone());
                self.rebuild();
                self.send_update(id, body, move_to, current, Some("Undone".into()));
            }
        }
    }

    // ---- adding ------------------------------------------------------

    fn add_task(&mut self, text: String, project: Option<String>, parent: Option<String>) {
        if text.trim().is_empty() {
            return;
        }
        let in_today = self.view == View::Today && self.search.is_empty();
        let text = match &self.view {
            View::Label(label) if !text.contains('@') => format!("{text} @{label}"),
            _ => text,
        };
        self.epoch += 1;
        self.say("Adding…");

        let client = self.client.clone();
        spawn_op(self.tx.clone(), None, Some("Added".into()), async move {
            let mut task = quick_add(&client, &text).await?;
            if let Some(parent) = parent {
                task = move_task(&client, &task.id, MoveTarget::Parent(parent)).await?;
            } else if let Some(project) = project {
                // `#project` in the text already chose a project.
                if !text.contains('#') && task.project_id != project {
                    task = move_task(&client, &task.id, MoveTarget::Project(project)).await?;
                }
            }
            if in_today && task.due.is_none() {
                let body = json!({ "due_string": "today" });
                task = update_task(&client, &task.id, body).await?;
            }
            Ok(Some(task))
        });
    }

    // ---- prompts, pickers and the edit panel ---------------------------

    pub fn submit_prompt(&mut self, prompt: Prompt) {
        let text = prompt.text();
        let text = text.trim();
        match prompt.kind {
            PromptKind::Add { project, parent } => self.add_task(text.to_string(), project, parent),
            PromptKind::Search => {
                self.search = text.to_string();
                self.rebuild();
            }
            PromptKind::Due(id) => {
                let Some(task) = self.store.task(&id) else {
                    return;
                };
                let old = task.due.as_ref().map_or("", |due| due.string.as_str());
                if text == old {
                    return;
                }
                let body = json!({ "due_string": if text.is_empty() { "no date" } else { text } });
                let clear = text.is_empty();
                self.mutate(
                    &id,
                    |task| {
                        if clear {
                            task.due = None
                        }
                    },
                    body,
                    None,
                );
            }
            PromptKind::Rename(id) => {
                if text.is_empty() {
                    self.error("A task needs some content");
                    return;
                }
                if self
                    .store
                    .task(&id)
                    .is_some_and(|task| task.content == text)
                {
                    return;
                }
                let new = text.to_string();
                self.mutate(
                    &id,
                    |task| task.content = new,
                    json!({ "content": text }),
                    None,
                );
            }
            PromptKind::Labels(id) => {
                let labels = parse_labels(text);
                if self
                    .store
                    .task(&id)
                    .is_some_and(|task| task.labels == labels)
                {
                    return;
                }
                let body = json!({ "labels": labels });
                self.mutate(&id, |task| task.labels = labels, body, None);
            }
        }
    }

    pub fn choose(&mut self, picker: Picker) {
        let Some(item) = picker.chosen().cloned() else {
            return;
        };
        match (picker.kind, item.target) {
            (PickerKind::Jump, PickTarget::View(view)) => {
                self.set_view(view);
                self.focus = Focus::List;
            }
            (PickerKind::MoveTask(id), PickTarget::Project(project)) => {
                let descendants = self.store.descendants(&id);
                let target = project.clone();
                self.mutate(
                    &id,
                    |task| {
                        task.project_id = target;
                        task.section_id = None;
                        task.parent_id = None;
                    },
                    json!({}),
                    Some(project.clone()),
                );
                for descendant in descendants {
                    if let Some(task) = self.store.task_mut(&descendant) {
                        task.project_id = project.clone();
                        task.section_id = None;
                    }
                }
                self.rebuild();
                self.say(format!("Moved to {}", item.label));
            }
            _ => {}
        }
    }

    /// Saves the edit panel; returns `false` if it should stay open.
    pub fn save_edit(&mut self, edit: &TaskEdit) -> bool {
        match edit.changes() {
            Err(message) => {
                self.error(message);
                false
            }
            Ok(None) => true,
            Ok(Some(changes)) => {
                let clear_due = changes.due.as_deref() == Some("");
                self.mutate(
                    &edit.task_id,
                    |task| {
                        task.content = changes.content;
                        task.description = changes.description;
                        task.labels = changes.labels;
                        task.priority = changes.priority;
                        if clear_due {
                            task.due = None;
                        }
                    },
                    changes.body,
                    None,
                );
                true
            }
        }
    }
}

/// Request body that recreates a deleted task.
fn recreate_body(task: &Task) -> Value {
    let mut body = json!({
        "content": task.content,
        "description": task.description,
        "project_id": task.project_id,
        "labels": task.labels,
        "priority": task.priority,
    });
    if let Some(section_id) = &task.section_id {
        body["section_id"] = section_id.clone().into();
    }
    if let Some(parent_id) = &task.parent_id {
        body["parent_id"] = parent_id.clone().into();
    }
    if let Some(due) = &task.due {
        body["due_string"] = due.api_string().into();
    }
    if let Some(duration) = &task.duration {
        body["duration"] = duration.amount.into();
        body["duration_unit"] = duration.unit.clone().into();
    }
    body
}

/// What to send so `current` goes back to `before`: an update body with only
/// the fields that differ, and the project to move back to if it changed.
fn restore_body(before: &Task, current: &Task) -> (Value, Option<String>) {
    let mut body = serde_json::Map::new();
    if before.content != current.content {
        body.insert("content".into(), before.content.clone().into());
    }
    if before.description != current.description {
        body.insert("description".into(), before.description.clone().into());
    }
    if before.labels != current.labels {
        body.insert("labels".into(), before.labels.clone().into());
    }
    if before.priority != current.priority {
        body.insert("priority".into(), before.priority.into());
    }
    let due_changed = match (&before.due, &current.due) {
        (None, None) => false,
        (Some(a), Some(b)) => !a.same_as(b),
        _ => true,
    };
    if due_changed {
        let due = before
            .due
            .as_ref()
            .map_or("no date".to_string(), Due::api_string);
        body.insert("due_string".into(), due.into());
    }
    let move_to = (before.project_id != current.project_id).then(|| before.project_id.clone());
    (body.into(), move_to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{projects::Project, store::Store};

    fn test_app(tasks: Vec<Value>) -> App {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut app = App::new(reqwest::Client::new(), tx);
        app.store = Store {
            tasks: tasks.into_iter().map(task).collect(),
            projects: vec![Project {
                id: "p".into(),
                name: "Proj".into(),
                parent_id: None,
                child_order: 0,
                inbox_project: false,
            }],
            sections: vec![],
        };
        app.view = View::Project("p".into());
        app.rebuild();
        app
    }

    fn item(id: &str, parent: Option<&str>, order: i32) -> Value {
        json!({"id": id, "content": id, "project_id": "p", "parent_id": parent, "child_order": order})
    }

    #[test]
    fn cursor_stays_on_the_same_task_when_the_list_changes() {
        let mut app = test_app(vec![
            item("a", None, 1),
            item("b", None, 2),
            item("c", None, 3),
        ]);
        app.move_cursor(2);
        assert_eq!(app.selected.as_deref(), Some("c"));
        // A task above the cursor disappears; the cursor must not jump.
        app.store.remove("a");
        app.rebuild();
        assert_eq!(app.selected.as_deref(), Some("c"));
    }

    #[test]
    fn removing_the_selected_task_selects_its_neighbour() {
        let mut app = test_app(vec![
            item("a", None, 1),
            item("b", None, 2),
            item("c", None, 3),
        ]);
        app.move_cursor(1);
        app.remove_locally("b");
        assert_eq!(app.selected.as_deref(), Some("c"));
        app.remove_locally("c");
        assert_eq!(app.selected.as_deref(), Some("a"));
    }

    #[test]
    fn removing_a_parent_removes_its_subtasks() {
        let mut app = test_app(vec![
            item("a", None, 1),
            item("b", Some("a"), 2),
            item("c", None, 3),
        ]);
        app.remove_locally("a");
        assert_eq!(app.task_count(), 1);
        assert!(app.store.task("b").is_none());
    }

    #[test]
    fn folding_hides_subtasks_and_unfolding_shows_them() {
        let mut app = test_app(vec![item("a", None, 1), item("b", Some("a"), 2)]);
        assert_eq!(app.task_count(), 2);
        app.run(Action::ToggleFold);
        assert_eq!(app.task_count(), 1);
        app.run(Action::ToggleFold);
        assert_eq!(app.task_count(), 2);
    }

    #[test]
    fn folding_a_task_without_subtasks_does_nothing() {
        let mut app = test_app(vec![item("a", None, 1), item("b", None, 2)]);
        app.run(Action::ToggleFold);
        assert_eq!(app.task_count(), 2);
    }

    #[test]
    fn cursor_movement_stops_at_the_ends() {
        let mut app = test_app(vec![item("a", None, 1), item("b", None, 2)]);
        app.run(Action::Down);
        app.run(Action::Down);
        assert_eq!(app.selected.as_deref(), Some("b"));
        app.run(Action::Top);
        assert_eq!(app.selected.as_deref(), Some("a"));
    }

    #[test]
    fn search_filters_across_the_whole_list_and_cancel_restores_it() {
        let mut app = test_app(vec![item("alpha", None, 1), item("beta", None, 2)]);
        app.search = "alp".into();
        app.rebuild();
        assert_eq!(app.task_count(), 1);
        app.run(Action::Cancel);
        assert_eq!(app.task_count(), 2);
    }

    fn task(json: Value) -> Task {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn restore_body_only_contains_what_changed() {
        let before = task(json!({"id":"1","content":"a","priority":1,"project_id":"p1"}));
        let mut current = before.clone();
        current.priority = 4;
        current.project_id = "p2".into();
        let (body, move_to) = restore_body(&before, &current);
        assert_eq!(body, json!({"priority": 1}));
        assert_eq!(move_to.as_deref(), Some("p1"));
    }

    #[test]
    fn restore_body_brings_back_a_removed_due_date() {
        let before = task(json!({"id":"1","content":"a",
            "due":{"date":"2026-10-25T17:00:00","string":"x","is_recurring":false}}));
        let mut current = before.clone();
        current.due = None;
        let (body, _) = restore_body(&before, &current);
        assert_eq!(body, json!({"due_string": "2026-10-25 17:00"}));
    }

    #[test]
    fn recreate_body_keeps_structure() {
        let deleted =
            task(json!({"id":"1","content":"a","project_id":"p","parent_id":"q","labels":["x"]}));
        let body = recreate_body(&deleted);
        assert_eq!(body["parent_id"], "q");
        assert_eq!(body["labels"], json!(["x"]));
    }
}
