use ratatui::{
    style::{Modifier, Style},
    widgets::ListState,
};
use tui_textarea::TextArea;

#[derive(Debug, Default, Clone)]
pub struct TaskEdit<'a> {
    pub content: TextArea<'a>,
    pub description: TextArea<'a>,
    pub due_string: TextArea<'a>,
    pub currently_editing: CurrentlyEditing,
    pub children: Vec<usize>,
    pub children_list_state: ListState,
    pub current_task_index: usize,
}

impl<'a> TaskEdit<'a> {
    pub fn new(
        content: String,
        description: String,
        due_string: String,
        children: Vec<usize>,
        current_task_index: usize,
        currently_editing: CurrentlyEditing,
    ) -> Self {
        let description_lines = description.split('\n').map(|s| s.to_string()).collect();

        let mut task_edit = TaskEdit {
            content: TextArea::from(vec![content]),
            description: TextArea::new(description_lines),
            due_string: TextArea::from(vec![due_string]),
            currently_editing,
            children,
            children_list_state: ListState::default(),
            current_task_index,
        };

        set_due_placeholder(&mut task_edit.due_string);
        task_edit.update_cursor_styles();

        task_edit
    }

    pub fn next(&mut self) {
        if self.children.is_empty() {
            self.children_list_state.select(None);
            return;
        }
        let i = match self.children_list_state.selected() {
            Some(i) => {
                if i >= self.children.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.children_list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.children.is_empty() {
            self.children_list_state.select(None);
            return;
        }
        let i = match self.children_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.children.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.children_list_state.select(Some(i));
    }

    pub fn focus(&mut self, field: CurrentlyEditing) {
        self.currently_editing = field;
        self.update_cursor_styles();
    }

    pub fn update_cursor_styles(&mut self) {
        let active = match self.currently_editing {
            CurrentlyEditing::Content => Some(0),
            CurrentlyEditing::Description => Some(1),
            CurrentlyEditing::DueString => Some(2),
            CurrentlyEditing::ChildTasks => None,
        };
        apply_cursor_styles(
            [
                &mut self.content,
                &mut self.description,
                &mut self.due_string,
            ],
            active,
        );
    }
}

/// Shows the cursor only in the active field (`active` indexes `fields`).
pub fn apply_cursor_styles(fields: [&mut TextArea; 3], active: Option<usize>) {
    for (i, field) in fields.into_iter().enumerate() {
        let style = if Some(i) == active {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        field.set_cursor_style(style);
    }
}

/// Hints at what the due field accepts; shown while it is empty.
pub fn set_due_placeholder(due: &mut TextArea) {
    due.set_placeholder_text("e.g. tomorrow 5pm, every monday, 25 dec");
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum CurrentlyEditing {
    #[default]
    Content,
    Description,
    DueString,
    ChildTasks,
}

impl CurrentlyEditing {
    pub fn next(self) -> Self {
        match self {
            Self::Content => Self::Description,
            Self::Description => Self::DueString,
            Self::DueString => Self::ChildTasks,
            Self::ChildTasks => Self::Content,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Content => Self::ChildTasks,
            Self::Description => Self::Content,
            Self::DueString => Self::Description,
            Self::ChildTasks => Self::DueString,
        }
    }
}
