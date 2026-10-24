use ratatui::{prelude::*, widgets::Paragraph};

use super::utils::{dim, text_width};
use crate::{
    api_calls,
    app::{App, Level, Mode, Overlay},
    prompt::PromptKind,
};

const SPINNER: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

fn badge(app: &App) -> Span<'static> {
    let (text, color) = match &app.mode {
        Mode::Normal => (" NORMAL ", Color::Green),
        Mode::Prompt(_) | Mode::Edit(_) | Mode::Picker(_) => (" INSERT ", Color::Yellow),
        Mode::Overlay(Overlay::Help) => (" HELP ", Color::Magenta),
        Mode::Overlay(Overlay::Messages) => (" MESSAGES ", Color::Magenta),
    };
    Span::styled(
        text,
        Style::default()
            .fg(Color::Black)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )
}

/// The status bar: mode, current view, pending keys and sync state.
pub fn modeline(f: &mut Frame, area: Rect, app: &App) {
    let mut left = vec![badge(app)];
    let name = if app.search.is_empty() {
        app.view.title(&app.store)
    } else {
        format!("/{}", app.search)
    };
    left.push(Span::styled(
        format!(" {name} "),
        Style::default().add_modifier(Modifier::BOLD),
    ));
    left.push(Span::styled(format!("{} tasks", app.task_count()), dim()));
    if let Some(task) = app.selected_task() {
        left.push(Span::styled(
            format!("  {}", app.store.project_name(&task.project_id)),
            dim(),
        ));
    }

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

    let right_width: usize = right.iter().map(|span| text_width(&span.content)).sum();
    f.render_widget(Paragraph::new(Line::from(left)), area);
    if right_width < area.width as usize {
        let rect = Rect {
            x: area.x + area.width - right_width as u16,
            width: right_width as u16,
            ..area
        };
        f.render_widget(Paragraph::new(Line::from(right)), rect);
    }
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
        f.render_widget(&prompt.input, input_area);
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
        None if matches!(app.mode, Mode::Normal) => Line::from(Span::styled(
            " SPC menu · a add · x done · / search · ? help",
            dim(),
        )),
        None => Line::default(),
    };
    f.render_widget(Paragraph::new(line), area);
}
