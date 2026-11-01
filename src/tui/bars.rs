use ratatui::{prelude::*, widgets::Paragraph};

use super::utils::{dim, text_width};
use crate::{
    api_calls,
    app::{App, Level, Mode},
    prompt::{Prompt, PromptKind},
    quick_add::{tokenize, TokenKind},
};

const SPINNER: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

/// Pending keys and sync state, shown at the right of the bottom bar.
fn status_right(app: &App) -> Vec<Span<'static>> {
    let mut right = Vec::new();
    if !app.pending_keys.is_empty() {
        right.push(Span::styled(
            format!("{} ", app.pending_keys.join(" ")),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }
    let busy = app.syncing || api_calls::pending() > 0;
    if busy {
        let frame = (app.last_tick_ms() / 100) as usize % SPINNER.len();
        right.push(Span::styled(
            format!("{} syncing ", SPINNER[frame]),
            Style::default().fg(Color::Yellow),
        ));
    } else if app.loaded {
        right.push(Span::styled("✓ synced ", Style::default().fg(Color::Green)));
    }
    right
}

/// lazygit-style `key action` hints.
fn hints() -> Line<'static> {
    let items = [
        ("space space", "complete"),
        ("a", "add"),
        ("e", "edit"),
        ("d d", "delete"),
        ("/", "filter"),
        ("tab", "panel"),
        ("?", "keybindings"),
        ("q", "quit"),
    ];
    let mut spans = vec![Span::raw(" ")];
    for (key, action) in items {
        spans.push(Span::styled(
            key,
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(": {action}  "), dim()));
    }
    Line::from(spans)
}

/// The line under the status bar: prompts, messages, or a hint.
pub fn echo_area(f: &mut Frame, area: Rect, app: &mut App) {
    if let Mode::Prompt(prompt) = &mut app.mode {
        let label = prompt.label();
        let label_width = text_width(label) as u16;
        let [label_area, input_area] =
            Layout::horizontal([Constraint::Length(label_width), Constraint::Min(1)]).areas(area);
        let accent = match prompt.kind {
            PromptKind::Search => Color::Yellow,
            _ => Color::Cyan,
        };
        f.render_widget(
            Paragraph::new(Span::styled(
                label,
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            )),
            label_area,
        );
        if matches!(prompt.kind, PromptKind::Add { .. }) {
            highlighted_input(f, input_area, prompt);
        } else {
            f.render_widget(&prompt.input, input_area);
        }
        return;
    }

    let line = match &app.status {
        Some(status) => {
            let style = match status.level {
                Level::Error => Style::default().fg(Color::Red),
                Level::Info => Style::default(),
            };
            Line::from(Span::styled(
                format!(" {}", status.text.replace('\n', " ")),
                style,
            ))
        }
        None if matches!(app.mode, Mode::Normal) => hints(),
        None => Line::default(),
    };
    f.render_widget(Paragraph::new(line), area);

    let right = status_right(app);
    let right_width: usize = right.iter().map(|span| text_width(&span.content)).sum();
    if right_width < area.width as usize {
        let rect = Rect {
            x: area.x + area.width - right_width as u16,
            width: right_width as u16,
            ..area
        };
        f.render_widget(Paragraph::new(Line::from(right)), rect);
    }
}

/// Quick-add input with the parts Todoist will recognise coloured.
fn highlighted_input(f: &mut Frame, area: Rect, prompt: &Prompt) {
    let line = prompt.text();
    let mut spans = Vec::new();
    let mut at = 0;
    for (range, kind) in tokenize(&line) {
        if range.start > at {
            spans.push(Span::raw(line[at..range.start].to_string()));
        }
        spans.push(Span::styled(line[range.clone()].to_string(), token_style(kind)));
        at = range.end;
    }
    spans.push(Span::raw(line[at..].to_string()));

    // Scroll sideways so the cursor stays in view.
    let before: String = line.chars().take(prompt.cursor()).collect();
    let cursor_x = text_width(&before);
    let scroll = (cursor_x + 1).saturating_sub(area.width as usize);
    f.render_widget(
        Paragraph::new(Line::from(spans)).scroll((0, scroll as u16)),
        area,
    );
    f.set_cursor_position((area.x + (cursor_x - scroll) as u16, area.y));
}

fn token_style(kind: TokenKind) -> Style {
    let color = match kind {
        TokenKind::Due => Color::Green,
        TokenKind::Priority(1) => Color::Red,
        TokenKind::Priority(2) => Color::Yellow,
        TokenKind::Priority(3) => Color::Blue,
        TokenKind::Priority(_) => Color::Gray,
        TokenKind::Project => Color::Magenta,
        TokenKind::Label => Color::Cyan,
    };
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}
