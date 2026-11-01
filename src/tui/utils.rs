use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub const ACCENT: Color = Color::Cyan;
/// Border colour of the focused panel (as in lazygit).
pub const FOCUS: Color = Color::Green;
/// Background of the selected line.
pub const SELECTED_BG: Color = Color::Blue;

/// A rounded panel; the focused one gets a green border and a bold title.
pub fn panel(title: impl Into<String>, focused: bool) -> Block<'static> {
    let (border, title_style) = if focused {
        (
            Style::default().fg(FOCUS),
            Style::default().fg(FOCUS).add_modifier(Modifier::BOLD),
        )
    } else {
        (Style::default(), Style::default().add_modifier(Modifier::BOLD))
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(Span::styled(format!(" {} ", title.into()), title_style))
}

pub fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

pub fn centered_rect(width: Constraint, height: Constraint, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Fill(1), height, Constraint::Fill(1)]).split(area);
    Layout::horizontal([Constraint::Fill(1), width, Constraint::Fill(1)]).split(vertical[1])[1]
}

/// A bordered input box, highlighted when it is the focused field.
pub fn field_block(title: &str, active: bool) -> Block<'static> {
    let color = if active { FOCUS } else { Color::Reset };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .title(format!(" {title} "))
}

pub fn text_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

/// Shortens `text` to at most `max` columns, ending in "…" if cut.
pub fn truncate(text: &str, max: usize) -> String {
    if text_width(text) <= max {
        return text.to_string();
    }
    let mut out = String::new();
    let mut width = 0;
    for c in text.chars() {
        let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
        if width + char_width + 1 > max {
            break;
        }
        out.push(c);
        width += char_width;
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_untouched() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn long_text_is_cut_with_an_ellipsis_within_the_limit() {
        let cut = truncate("hello world", 6);
        assert_eq!(cut, "hello…");
        assert!(text_width(&cut) <= 6);
    }

    #[test]
    fn wide_characters_are_counted_by_columns() {
        assert!(text_width(&truncate("日本語日本語", 7)) <= 7);
    }
}
