use ratatui::style::{Modifier, Style};
use tui_textarea::{CursorMove, TextArea};

use crate::tasks::{api_priority, Task};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Field {
    Content,
    Description,
    Due,
    Labels,
    Priority,
}

impl Field {
    fn next(self) -> Self {
        match self {
            Field::Content => Field::Description,
            Field::Description => Field::Due,
            Field::Due => Field::Labels,
            Field::Labels => Field::Priority,
            Field::Priority => Field::Content,
        }
    }

    fn previous(self) -> Self {
        match self {
            Field::Content => Field::Priority,
            Field::Description => Field::Content,
            Field::Due => Field::Description,
            Field::Labels => Field::Due,
            Field::Priority => Field::Labels,
        }
    }
}

/// The edit panel for one task.
pub struct TaskEdit {
    pub task_id: String,
    pub content: TextArea<'static>,
    pub description: TextArea<'static>,
    pub due: TextArea<'static>,
    pub labels: TextArea<'static>,
    /// 1 (highest) to 4.
    pub priority: u8,
    pub focus: Field,
    /// `C-c` was pressed; waiting for `C-c` (save) or `C-k` (abort).
    pub chord: bool,
    /// Esc was pressed with unsaved changes; waiting for y/n.
    pub confirm_discard: bool,
    original: Original,
}

#[derive(Clone, PartialEq)]
struct Original {
    content: String,
    description: String,
    due: String,
    labels: String,
    priority: u8,
}

/// What the panel changes, ready to apply and send.
pub struct Changes {
    pub content: String,
    pub description: String,
    pub labels: Vec<String>,
    pub priority: u8,
    /// New due text if it was edited; empty clears the date.
    pub due: Option<String>,
    pub body: serde_json::Value,
}

fn field(text: &str) -> TextArea<'static> {
    let lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    let mut area = TextArea::new(lines);
    area.move_cursor(CursorMove::Bottom);
    area.move_cursor(CursorMove::End);
    area.set_cursor_line_style(Style::default());
    area
}

pub fn parse_labels(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || c == ',')
        .map(|label| label.trim_start_matches('@'))
        .filter(|label| !label.is_empty())
        .map(str::to_string)
        .collect()
}

impl TaskEdit {
    pub fn new(task: &Task) -> Self {
        let due = task
            .due
            .as_ref()
            .map_or(String::new(), |due| due.string.clone());
        let labels = task.labels.join(" ");
        let mut edit = TaskEdit {
            task_id: task.id.clone(),
            content: field(&task.content),
            description: field(&task.description),
            due: field(&due),
            labels: field(&labels),
            priority: task.ui_priority(),
            focus: Field::Content,
            chord: false,
            confirm_discard: false,
            original: Original {
                content: task.content.clone(),
                description: task.description.clone(),
                due,
                labels,
                priority: task.ui_priority(),
            },
        };
        edit.due
            .set_placeholder_text("e.g. tomorrow 5pm, every monday, 25 dec");
        edit.update_cursors();
        edit
    }

    fn current(&self) -> Original {
        Original {
            content: self.content.lines().join("\n"),
            description: self.description.lines().join("\n"),
            due: self.due.lines().join(" ").trim().to_string(),
            labels: self.labels.lines().join(" "),
            priority: self.priority,
        }
    }

    pub fn is_dirty(&self) -> bool {
        let current = self.current();
        // Labels are compared as lists so spacing doesn't count as an edit.
        current.content != self.original.content
            || current.description != self.original.description
            || current.due != self.original.due
            || parse_labels(&current.labels) != parse_labels(&self.original.labels)
            || current.priority != self.original.priority
    }

    pub fn focus_next(&mut self) {
        self.focus = self.focus.next();
        self.update_cursors();
    }

    pub fn focus_previous(&mut self) {
        self.focus = self.focus.previous();
        self.update_cursors();
    }

    fn update_cursors(&mut self) {
        let active = Style::default().add_modifier(Modifier::REVERSED);
        let inactive = Style::default();
        for (area, which) in [
            (&mut self.content, Field::Content),
            (&mut self.description, Field::Description),
            (&mut self.due, Field::Due),
            (&mut self.labels, Field::Labels),
        ] {
            area.set_cursor_style(if self.focus == which {
                active
            } else {
                inactive
            });
        }
    }

    /// The text area that has focus, if the focused field is a text field.
    pub fn active_area(&mut self) -> Option<&mut TextArea<'static>> {
        match self.focus {
            Field::Content => Some(&mut self.content),
            Field::Description => Some(&mut self.description),
            Field::Due => Some(&mut self.due),
            Field::Labels => Some(&mut self.labels),
            Field::Priority => None,
        }
    }

    /// The edits compared to the task; `None` if nothing changed. The error
    /// is for content left blank.
    pub fn changes(&self) -> Result<Option<Changes>, &'static str> {
        if !self.is_dirty() {
            return Ok(None);
        }
        let current = self.current();
        if current.content.trim().is_empty() {
            return Err("A task needs some content");
        }

        let labels = parse_labels(&current.labels);
        let mut body = serde_json::Map::new();
        if current.content != self.original.content {
            body.insert("content".into(), current.content.clone().into());
        }
        if current.description != self.original.description {
            body.insert("description".into(), current.description.clone().into());
        }
        if labels != parse_labels(&self.original.labels) {
            body.insert("labels".into(), labels.clone().into());
        }
        if current.priority != self.original.priority {
            body.insert("priority".into(), api_priority(current.priority).into());
        }
        let due = (current.due != self.original.due).then(|| current.due.clone());
        if let Some(due) = &due {
            body.insert(
                "due_string".into(),
                if due.is_empty() {
                    "no date"
                } else {
                    due.as_str()
                }
                .into(),
            );
        }

        Ok(Some(Changes {
            content: current.content,
            description: current.description,
            labels,
            priority: api_priority(current.priority),
            due,
            body: body.into(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task() -> Task {
        serde_json::from_value(serde_json::json!({
            "id": "1", "content": "Buy milk", "priority": 1, "labels": ["home"],
            "due": {"date": "2026-10-25", "string": "25 Oct", "is_recurring": false},
        }))
        .unwrap()
    }

    #[test]
    fn untouched_edit_has_no_changes() {
        assert!(TaskEdit::new(&task()).changes().unwrap().is_none());
    }

    #[test]
    fn only_edited_fields_are_sent() {
        let mut edit = TaskEdit::new(&task());
        edit.priority = 1;
        edit.labels = field("home @errands");
        let changes = edit.changes().unwrap().unwrap();
        assert_eq!(changes.body["priority"], 4);
        assert_eq!(
            changes.body["labels"],
            serde_json::json!(["home", "errands"])
        );
        assert!(changes.body.get("content").is_none());
        assert!(changes.body.get("due_string").is_none());
    }

    #[test]
    fn clearing_due_sends_no_date() {
        let mut edit = TaskEdit::new(&task());
        edit.due = field("");
        let changes = edit.changes().unwrap().unwrap();
        assert_eq!(changes.body["due_string"], "no date");
    }

    #[test]
    fn blank_content_is_rejected() {
        let mut edit = TaskEdit::new(&task());
        edit.content = field("  ");
        assert!(edit.changes().is_err());
    }
}
