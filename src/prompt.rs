use tui_textarea::{CursorMove, TextArea};

use crate::quick_add::Suggestions;

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
    /// Completions for the `#`/`@` word under the cursor (quick add only).
    pub suggest: Option<Suggestions>,
}

impl Prompt {
    pub fn new(kind: PromptKind, initial: &str) -> Self {
        let mut input = TextArea::new(vec![initial.to_string()]);
        input.move_cursor(CursorMove::End);
        // The cursor is drawn by the terminal, not as a highlighted cell.
        input.set_cursor_line_style(ratatui::style::Style::default());
        Prompt {
            kind,
            input,
            suggest: None,
        }
    }

    /// The cursor's position in chars.
    pub fn cursor(&self) -> usize {
        self.input.cursor().1
    }

    /// Replaces the text and puts the cursor at `cursor` chars.
    pub fn set_text(&mut self, text: &str, cursor: usize) {
        let mut input = TextArea::new(vec![text.to_string()]);
        input.move_cursor(CursorMove::Jump(0, cursor as u16));
        input.set_cursor_line_style(ratatui::style::Style::default());
        self.input = input;
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
