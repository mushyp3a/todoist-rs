use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem},
};

use super::utils::{dim, text_width, truncate, ACCENT};
use crate::{
    app::{App, Focus},
    view::SidebarEntry,
};

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default().borders(Borders::RIGHT).border_style(dim());
    let inner_width = block.inner(area).width as usize;

    let items: Vec<ListItem> = app
        .sidebar
        .iter()
        .map(|entry| match entry {
            SidebarEntry::Header(text) => ListItem::new(Line::from(Span::styled(
                format!(" {text}"),
                dim().add_modifier(Modifier::BOLD),
            ))),
            SidebarEntry::Item {
                label,
                depth,
                count,
                foldable,
                folded,
                ..
            } => {
                let marker = match (foldable, folded) {
                    (true, true) => "▸ ",
                    (true, false) => "▾ ",
                    _ => "  ",
                };
                let count = if *count > 0 {
                    count.to_string()
                } else {
                    String::new()
                };
                let left = format!(" {}{marker}", "  ".repeat(*depth as usize));
                let room = inner_width.saturating_sub(text_width(&left) + text_width(&count) + 2);
                let label = truncate(label, room);
                let pad = inner_width.saturating_sub(
                    text_width(&left) + text_width(&label) + text_width(&count) + 1,
                );
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{left}{label}")),
                    Span::raw(" ".repeat(pad)),
                    Span::styled(count, dim()),
                ]))
            }
        })
        .collect();

    let highlight = if app.focus == Focus::Sidebar {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    };
    let list = List::new(items).block(block).highlight_style(highlight);
    f.render_stateful_widget(list, area, &mut app.sidebar_state);
}
