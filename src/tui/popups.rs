use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use super::utils::{centered_rect, dim, field_block, ACCENT};
use crate::{
    picker::Picker,
    task_edit::{Field, TaskEdit},
};

pub fn edit(f: &mut Frame, edit: &mut TaskEdit) {
    let height = 20.min(f.area().height);
    let area = centered_rect(
        Constraint::Percentage(70),
        Constraint::Length(height),
        f.area(),
    );
    f.render_widget(Clear, area);

    let footer = if edit.confirm_discard {
        Line::from(Span::styled(
            " Discard changes? (y/n) ",
            Style::default().fg(Color::Red),
        ))
    } else if edit.chord {
        Line::from(Span::styled(
            " C-c … C-c save · C-k abort ",
            Style::default().fg(Color::Yellow),
        ))
    } else {
        Line::from(vec![
            " ".into(),
            Span::styled("C-c C-c", Style::default().fg(ACCENT)),
            " save  ".into(),
            Span::styled("Esc", Style::default().fg(ACCENT)),
            " cancel  ".into(),
            Span::styled("Tab", Style::default().fg(ACCENT)),
            " next field ".into(),
        ])
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Edit task ")
        .title_bottom(footer.centered());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let [content, description, due, labels, priority, _] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(6),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .areas(inner);

    let focus = edit.focus;
    edit.content
        .set_block(field_block("Task", focus == Field::Content));
    edit.description.set_block(field_block(
        "Description (Enter = new line)",
        focus == Field::Description,
    ));
    edit.due.set_block(field_block("Due", focus == Field::Due));
    edit.labels.set_block(field_block(
        "Labels (space separated)",
        focus == Field::Labels,
    ));
    f.render_widget(&edit.content, content);
    f.render_widget(&edit.description, description);
    f.render_widget(&edit.due, due);
    f.render_widget(&edit.labels, labels);

    let colors = [Color::Red, Color::Yellow, Color::Blue, Color::Reset];
    let mut spans = vec![Span::raw(" ")];
    for level in 1..=4u8 {
        let mut style = Style::default().fg(colors[level as usize - 1]);
        if level == edit.priority {
            style = style.add_modifier(Modifier::REVERSED | Modifier::BOLD);
        }
        spans.push(Span::styled(format!(" P{level} "), style));
        spans.push(Span::raw(" "));
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)).block(field_block(
            "Priority (h/l or 1-4)",
            focus == Field::Priority,
        )),
        priority,
    );
}

pub fn picker(f: &mut Frame, picker: &mut Picker) {
    let wanted = (picker.matches.len() as u16 + 4).clamp(6, 18);
    let area = centered_rect(
        Constraint::Percentage(60),
        Constraint::Length(wanted.min(f.area().height)),
        f.area(),
    );
    f.render_widget(Clear, area);

    let count = format!(" {}/{} ", picker.matches.len(), picker.items.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", picker.title))
        .title_bottom(Line::from(Span::styled(count, dim())).right_aligned());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let [input_area, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(inner);
    let [prompt_area, text_area] =
        Layout::horizontal([Constraint::Length(2), Constraint::Min(1)]).areas(input_area);
    f.render_widget(
        Paragraph::new(Span::styled("› ", Style::default().fg(ACCENT))),
        prompt_area,
    );
    f.render_widget(&picker.input, text_area);

    let items: Vec<ListItem> = picker
        .matches
        .iter()
        .map(|&i| ListItem::new(Line::from(format!(" {}", picker.items[i].label))))
        .collect();
    let mut state = ratatui::widgets::ListState::default();
    if !items.is_empty() {
        state.select(Some(picker.selected));
    }
    f.render_stateful_widget(
        List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED)),
        list_area,
        &mut state,
    );
}
