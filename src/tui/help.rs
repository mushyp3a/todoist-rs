use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::utils::{dim, ACCENT};
use crate::{
    app::Level,
    keymap::{Action, BINDINGS},
};

struct Entry {
    keys: Vec<&'static str>,
    description: &'static str,
    action: Action,
}

/// Bindings under `prefix`, with keys that do the same thing merged ("j, down").
fn section(title: &'static str, belongs: impl Fn(&str) -> bool) -> Vec<Line<'static>> {
    let mut entries: Vec<Entry> = Vec::new();
    for (sequence, description, action) in BINDINGS.iter().filter(|(s, _, _)| belongs(s)) {
        match entries
            .iter_mut()
            .find(|entry| entry.action == *action && entry.description == *description)
        {
            Some(entry) => entry.keys.push(sequence),
            None => entries.push(Entry {
                keys: vec![sequence],
                description,
                action: *action,
            }),
        }
    }

    let mut lines = vec![
        Line::default(),
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    for entry in entries {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<16}", entry.keys.join(", ")),
                Style::default().fg(ACCENT),
            ),
            Span::raw(entry.description),
        ]));
    }
    lines
}

pub fn help(f: &mut Frame, scroll: u16) {
    let mut lines = vec![Line::from(Span::styled(
        "Press SPC (or g, s, z) and wait: a menu shows what comes next.",
        dim(),
    ))];
    lines.extend(section("Normal mode", |s| !s.starts_with("SPC")));
    lines.extend(section("Leader: SPC", |s| {
        s.starts_with("SPC") && !s.starts_with("SPC m")
    }));
    lines.extend(section("Task commands: SPC m", |s| s.starts_with("SPC m")));
    let panels: [(&str, &[(&str, &str)]); 2] = [
        (
            "Edit panel",
            &[
                ("C-c C-c, C-s", "save"),
                ("Esc, C-c C-k", "cancel"),
                ("Tab, S-Tab", "next / previous field"),
                ("Enter", "save (new line in the description)"),
            ],
        ),
        (
            "Reschedule popup",
            &[
                ("C-h / C-l", "previous / next day"),
                ("C-k / C-j", "previous / next week"),
                ("H / L, C-p / C-n", "previous / next month"),
                ("Enter", "set the highlighted date"),
                ("t m w n s x", "today, tomorrow, Monday, +1 week, weekend, no date"),
                ("Esc, C-g", "cancel"),
            ],
        ),
    ];
    for (title, rows) in panels {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            title,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));
        for (keys, text) in rows {
            lines.push(Line::from(vec![
                Span::styled(format!("  {keys:<18}"), Style::default().fg(ACCENT)),
                Span::raw(*text),
            ]));
        }
    }

    let area = f.area().inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .title_bottom(Line::from(" j/k scroll · q close ").centered());
    f.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), area);
}

pub fn messages(f: &mut Frame, log: &[(Level, String)], scroll: u16) {
    let lines: Vec<Line> = if log.is_empty() {
        vec![Line::from(Span::styled("No messages yet.", dim()))]
    } else {
        log.iter()
            .rev()
            .map(|(level, text)| {
                let style = match level {
                    Level::Error => Style::default().fg(Color::Red),
                    Level::Info => Style::default(),
                };
                Line::from(Span::styled(text.clone(), style))
            })
            .collect()
    };
    let area = f.area().inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Messages (newest first) ")
        .title_bottom(Line::from(" j/k scroll · q close ").centered());
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(ratatui::widgets::Wrap { trim: false })
            .scroll((scroll, 0)),
        area,
    );
}
