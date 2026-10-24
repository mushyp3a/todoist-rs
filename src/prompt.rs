use tui_textarea::{CursorMove, TextArea};

/// What a one-line prompt in the echo area will do when submitted.
#[derive(Debug, Clone)]
pub enum PromptKind {
    Add {
        project: Option<String>,
        parent: Option<String>,
    },
    Search,
    Due(String),
    Rename(String),
    Labels(String),
}

pub struct Prompt {
    pub kind: PromptKind,
    pub input: TextArea<'static>,
}

impl Prompt {
    pub fn new(kind: PromptKind, initial: &str) -> Self {
        let mut input = TextArea::new(vec![initial.to_string()]);
        input.move_cursor(CursorMove::End);
        // The cursor is drawn by the terminal, not as a highlighted cell.
        input.set_cursor_line_style(ratatui::style::Style::default());
        Prompt { kind, input }
    }

    pub fn label(&self) -> &'static str {
        match self.kind {
            PromptKind::Add {
                parent: Some(_), ..
            } => "Add sub-task: ",
            PromptKind::Add { .. } => "Add task: ",
            PromptKind::Search => "Search: ",
            PromptKind::Due(_) => "Due (empty clears): ",
            PromptKind::Rename(_) => "Rename: ",
            PromptKind::Labels(_) => "Labels (space separated): ",
        }
    }

    pub fn text(&self) -> String {
        self.input.lines().join(" ")
    }
}
