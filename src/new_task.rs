use tui_textarea::TextArea;

use crate::task_edit::{apply_cursor_styles, set_due_placeholder};

#[derive(Debug, Default)]
pub struct NewTask<'a> {
    pub content: TextArea<'a>,
    pub description: TextArea<'a>,
    pub due_string: TextArea<'a>,
    /// `None` creates the task in the user's Inbox.
    pub project_id: Option<String>,
    pub parent_id: Option<String>,
    pub currently_editing: CurrentlyEditing,
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum CurrentlyEditing {
    #[default]
    Content,
    Description,
    DueString,
}

impl CurrentlyEditing {
    pub fn next(self) -> Self {
        match self {
            Self::Content => Self::Description,
            Self::Description => Self::DueString,
            Self::DueString => Self::Content,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Content => Self::DueString,
            Self::Description => Self::Content,
            Self::DueString => Self::Description,
        }
    }
}

impl<'a> NewTask<'a> {
    pub fn new(project_id: Option<String>, parent_id: Option<String>) -> Self {
        let mut new_task = NewTask {
            project_id,
            parent_id,
            ..Default::default()
        };
        set_due_placeholder(&mut new_task.due_string);
        new_task.update_cursor_styles();
        new_task
    }

    pub fn focus(&mut self, field: CurrentlyEditing) {
        self.currently_editing = field;
        self.update_cursor_styles();
    }

    pub fn update_cursor_styles(&mut self) {
        let active = match self.currently_editing {
            CurrentlyEditing::Content => 0,
            CurrentlyEditing::Description => 1,
            CurrentlyEditing::DueString => 2,
        };
        apply_cursor_styles(
            [
                &mut self.content,
                &mut self.description,
                &mut self.due_string,
            ],
            Some(active),
        );
    }

    /// The request body for creating the task, or `None` if it has no content yet.
    pub fn to_json(&self) -> Option<serde_json::Value> {
        let content = self.content.lines().join("\n");
        if content.trim().is_empty() {
            return None;
        }

        let mut json = serde_json::json!({
            "content": content,
            "description": self.description.lines().join("\n"),
        });
        if let Some(project_id) = &self.project_id {
            json["project_id"] = project_id.clone().into();
        }
        if let Some(parent_id) = &self.parent_id {
            json["parent_id"] = parent_id.clone().into();
        }
        let due_string = self.due_string.lines().join("\n");
        if !due_string.trim().is_empty() {
            json["due_string"] = due_string.trim().into();
        }
        Some(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled(content: &str, due: &str) -> NewTask<'static> {
        let mut task = NewTask::new(Some("p1".into()), None);
        task.content = TextArea::from(vec![content.to_string()]);
        task.due_string = TextArea::from(vec![due.to_string()]);
        task
    }

    #[test]
    fn blank_content_is_not_submitted() {
        assert!(filled("   ", "tomorrow").to_json().is_none());
    }

    #[test]
    fn only_filled_fields_are_sent() {
        let json = filled("Buy milk", "").to_json().unwrap();
        assert_eq!(json["content"], "Buy milk");
        assert_eq!(json["project_id"], "p1");
        assert!(json.get("due_string").is_none());
        assert!(json.get("parent_id").is_none());
    }

    #[test]
    fn due_string_is_trimmed_and_sent() {
        let json = filled("Buy milk", " tomorrow 5pm ").to_json().unwrap();
        assert_eq!(json["due_string"], "tomorrow 5pm");
    }

    #[test]
    fn no_project_means_inbox() {
        let mut task = filled("x", "");
        task.project_id = None;
        assert!(task.to_json().unwrap().get("project_id").is_none());
    }
}
