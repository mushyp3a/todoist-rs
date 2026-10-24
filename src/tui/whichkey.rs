use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::utils::{dim, text_width, ACCENT};
use crate::{
    app::{App, Mode, WHICH_KEY_DELAY},
    keymap,
};

/// The popup listing what can follow the keys typed so far (like Emacs which-key).
pub fn render(f: &mut Frame, main: Rect, app: &App) {
    let waited = app
        .pending_since
        .is_some_and(|since| since.elapsed() >= WHICH_KEY_DELAY);
    if !matches!(app.mode, Mode::Normal) || app.pending_keys.is_empty() || !waited {
        return;
    }
    let mut hints = keymap::hints(&app.pending_keys);
    if hints.is_empty() {
        return;
    }
    hints.sort_by(|a, b| a.key.cmp(&b.key));

    let cell_width = hints
        .iter()
        .map(|hint| text_width(&hint.key) + text_width(&hint.text) + 5)
        .max()
        .unwrap_or(20)
        .max(22);
    let inner_width = main.width.saturating_sub(2) as usize;
    let columns = (inner_width / cell_width).max(1);
    let rows = hints.len().div_ceil(columns);
    let height = (rows as u16 + 2).min(main.height);

    let area = Rect {
        x: main.x,
        y: main.y + main.height - height,
        width: main.width,
        height,
    };

    // Fill column by column so related keys stay together.
    let mut lines: Vec<Line> = Vec::new();
    for row in 0..rows {
        let mut spans = Vec::new();
        for column in 0..columns {
            let Some(hint) = hints.get(column * rows + row) else {
                continue;
            };
            let text_style = if hint.text.starts_with('+') {
                Style::default().fg(Color::Magenta)
            } else {
                Style::default()
            };
            let used = text_width(&hint.key) + 3 + text_width(&hint.text);
            spans.push(Span::styled(
                hint.key.clone(),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(" → ", dim()));
            spans.push(Span::styled(hint.text.clone(), text_style));
            spans.push(Span::raw(" ".repeat(cell_width.saturating_sub(used))));
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(dim())
        .title(format!(" {} … ", app.pending_keys.join(" ")));
    f.render_widget(Paragraph::new(lines).block(block), area);
}
