use chrono::NaiveDate;
use ratatui::{
    prelude::*,
    widgets::{List, ListItem, Paragraph},
};

use super::utils::{dim, text_width, truncate, ACCENT};
use crate::{
    app::{App, Focus},
    tasks::{Due, Task},
    view::{HeaderKind, Row, TaskRow, View},
};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let [title_area, body] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);
    app.list_height = body.height as usize;

    f.render_widget(Paragraph::new(title(app)), title_area);

    if app.rows.is_empty() {
        let message = if !app.loaded {
            "Loading…"
        } else if !app.search.is_empty() {
            "No matches — Esc clears the search"
        } else {
            "Nothing here — press a to add a task"
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(format!("  {message}"), dim()))),
            body,
        );
        return;
    }

    let today = app.today();
    // Date headers already say which day it is.
    let show_dates =
        !app.search.is_empty() || !matches!(app.view, View::Today | View::Upcoming | View::Overdue);
    let items: Vec<ListItem> = app
        .rows
        .iter()
        .filter_map(|row| match row {
            Row::Header { text, kind } => Some(header_item(text, *kind)),
            Row::Task(task_row) => app
                .store
                .task(&task_row.id)
                .map(|task| task_item(task, task_row, today, show_dates, body.width as usize)),
        })
        .collect();

    let highlight = if app.focus == Focus::List {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    f.render_stateful_widget(
        List::new(items).highlight_style(highlight),
        body,
        &mut app.list_state,
    );
}

fn title(app: &App) -> Line<'static> {
    let name = if app.search.is_empty() {
        app.view.title(&app.store)
    } else {
        format!("Search: {}", app.search)
    };
    let mut spans = vec![
        Span::styled(
            format!(" {name}"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  {}", app.task_count()), dim()),
    ];
    if app.sort != crate::view::Sort::Natural {
        spans.push(Span::styled(
            format!("  sorted by {}", app.sort.label()),
            dim(),
        ));
    }
    Line::from(spans)
}

fn header_item(text: &str, kind: HeaderKind) -> ListItem<'static> {
    let style = match kind {
        HeaderKind::Overdue => Style::default().fg(Color::Red),
        HeaderKind::Today => Style::default().fg(Color::Green),
        HeaderKind::Plain => Style::default().fg(ACCENT),
    }
    .add_modifier(Modifier::BOLD);
    ListItem::new(Line::from(Span::styled(format!(" {text}"), style)))
}

fn priority_color(task: &Task) -> Style {
    match task.ui_priority() {
        1 => Style::default().fg(Color::Red),
        2 => Style::default().fg(Color::Yellow),
        3 => Style::default().fg(Color::Blue),
        _ => dim(),
    }
}

/// "today", "tomorrow", "25 Oct" (with the year when it isn't this year).
fn day_text(date: NaiveDate, today: NaiveDate) -> String {
    use chrono::Datelike;
    match (date - today).num_days() {
        0 => "today".into(),
        1 => "tomorrow".into(),
        -1 => "yesterday".into(),
        _ if date.year() == today.year() => date.format("%-d %b").to_string(),
        _ => date.format("%-d %b %Y").to_string(),
    }
}

fn due_text(due: &Due, today: NaiveDate, show_date: bool) -> String {
    let mut parts = Vec::new();
    if show_date {
        parts.push(day_text(due.date, today));
    }
    if let Some(time) = due.time {
        parts.push(time.format("%H:%M").to_string());
    }
    if due.is_recurring {
        parts.push("↻".into());
    }
    parts.join(" ")
}

fn task_item(
    task: &Task,
    row: &TaskRow,
    today: NaiveDate,
    show_dates: bool,
    width: usize,
) -> ListItem<'static> {
    let marker = match (row.has_children, row.collapsed) {
        (true, true) => "▸ ",
        (true, false) => "▾ ",
        _ => "  ",
    };
    let left_prefix = format!(" {}{marker}", "  ".repeat(row.depth as usize));

    // Right-hand metadata: labels, then the due date.
    let labels = task
        .labels
        .iter()
        .map(|label| format!("@{label}"))
        .collect::<Vec<_>>()
        .join(" ");
    let (due, due_style) = match &task.due {
        Some(due) => {
            let style = if due.date < today {
                Style::default().fg(Color::Red)
            } else if due.date == today {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(ACCENT)
            };
            (due_text(due, today, show_dates), style)
        }
        None => (String::new(), Style::default()),
    };
    let right_width = text_width(&labels)
        + text_width(&due)
        + usize::from(!labels.is_empty() && !due.is_empty())
        + 1;

    let mut suffix = String::new();
    if !task.description.is_empty() {
        suffix.push_str(" ≡");
    }
    if row.hidden > 0 {
        suffix.push_str(&format!(" (+{})", row.hidden));
    }

    let used = text_width(&left_prefix) + 2 + text_width(&suffix) + right_width;
    let content = truncate(&task.content, width.saturating_sub(used + 1));
    let pad = width.saturating_sub(used + text_width(&content));

    let mut spans = vec![
        Span::raw(left_prefix),
        Span::styled("● ", priority_color(task)),
        Span::raw(content),
        Span::styled(suffix, dim()),
        Span::raw(" ".repeat(pad)),
    ];
    if !labels.is_empty() {
        spans.push(Span::styled(
            labels.clone(),
            Style::default().fg(Color::Magenta),
        ));
        if !due.is_empty() {
            spans.push(Span::raw(" "));
        }
    }
    spans.push(Span::styled(due, due_style));
    spans.push(Span::raw(" "));
    ListItem::new(Line::from(spans))
}
