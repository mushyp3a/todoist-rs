use chrono::{Datelike, Local};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, ListItem},
};
use textwrap::fill;

use crate::tasks::{Due, Task};

pub fn centered_rect(horizontal: Constraint, vertical: Constraint, r: Rect) -> Rect {
    let popup_layout = match vertical {
        Constraint::Percentage(percent_y) => Layout::vertical([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r),
        Constraint::Length(height) => {
            let vertical_margin = (r.height.saturating_sub(height)) / 2;
            Layout::vertical([
                Constraint::Length(vertical_margin),
                Constraint::Length(height),
                Constraint::Length(vertical_margin),
            ])
            .split(r)
        }
        _ => panic!("Unsupported vertical constraint"),
    };

    match horizontal {
        Constraint::Percentage(percent_x) => Layout::horizontal([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1],
        Constraint::Length(width) => {
            let horizontal_margin = (r.width.saturating_sub(width)) / 2;
            Layout::horizontal([
                Constraint::Length(horizontal_margin),
                Constraint::Length(width),
                Constraint::Length(horizontal_margin),
            ])
            .split(popup_layout[1])[1]
        }
        _ => panic!("Unsupported horizontal constraint"),
    }
}

/// A bordered input box, highlighted when it is the focused field.
pub fn field_block(title: &str, active: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .fg(if active {
            Color::Indexed(47)
        } else {
            Color::White
        })
}

/// "25 Oct", "25 Oct 2027" when not this year, plus " 17:00" for timed tasks
/// and " ↻" for recurring ones.
pub fn format_due(due: &Due) -> String {
    let format = if due.date.year() == Local::now().year() {
        "%-d %b"
    } else {
        "%-d %b %Y"
    };
    let mut text = due.date.format(format).to_string();
    if let Some(time) = due.time {
        text.push_str(&time.format(" %H:%M").to_string());
    }
    if due.is_recurring {
        text.push_str(" ↻");
    }
    text
}

fn priority_label(priority: u8) -> Option<&'static str> {
    // The API's priority 4 is the UI's P1; 1 is the default and not shown.
    match priority {
        4 => Some("P1"),
        3 => Some("P2"),
        2 => Some("P3"),
        _ => None,
    }
}

pub fn generate_list_item<'a>(task: &Task, children: u16, width: usize) -> ListItem<'a> {
    let mut text = format!(
        "[ ] {} {}",
        if children > 0 { "⤷" } else { " " },
        task.content
    );
    if let Some(due) = &task.due {
        text.push_str(&format!("  ({})", format_due(due)));
    }
    if let Some(priority) = priority_label(task.priority) {
        text.push_str(&format!("  {priority}"));
    }

    ListItem::new(Text::styled(
        fill(&text, width.saturating_sub(4)),
        Style::default().fg(Color::White),
    ))
}
