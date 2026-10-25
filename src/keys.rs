//! Routes a key press to the current mode.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    app::{App, Mode, Overlay},
    keymap::{self, Lookup},
    picker::Picker,
    prompt::{Prompt, PromptKind},
    quick_add,
    task_edit::{Field, TaskEdit},
};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    app.dirty = true;
    // Each handler puts the mode back if it stays open.
    match std::mem::replace(&mut app.mode, Mode::Normal) {
        Mode::Normal => normal_key(app, key),
        Mode::Prompt(prompt) => prompt_key(app, prompt, key),
        Mode::Picker(picker) => picker_key(app, picker, key),
        Mode::Edit(edit) => edit_key(app, edit, key),
        Mode::Overlay(overlay) => overlay_key(app, overlay, key),
    }
}

fn is_ctrl(key: &KeyEvent, c: char) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char(c)
}

fn clear_pending(app: &mut App) {
    app.pending_keys.clear();
    app.pending_since = None;
}

fn normal_key(app: &mut App, key: KeyEvent) {
    let Some(token) = keymap::token(&key) else {
        return;
    };
    // Escape abandons a half-typed sequence before it means "cancel".
    if token == "ESC" && !app.pending_keys.is_empty() {
        clear_pending(app);
        return;
    }

    app.pending_keys.push(token);
    match keymap::lookup(&app.pending_keys) {
        Lookup::Action(action) => {
            clear_pending(app);
            app.run(action);
        }
        Lookup::Pending => {
            app.pending_since.get_or_insert_with(Instant::now);
        }
        Lookup::Unbound => {
            let sequence = app.pending_keys.join(" ");
            clear_pending(app);
            app.say(format!("{sequence} is undefined"));
        }
    }
}

fn prompt_key(app: &mut App, mut prompt: Prompt, key: KeyEvent) {
    let searching = matches!(prompt.kind, PromptKind::Search);
    let adding = matches!(prompt.kind, PromptKind::Add { .. });
    let cancel = key.code == KeyCode::Esc || is_ctrl(&key, 'g');

    let text = prompt.text();
    if let Some(suggest) = prompt.suggest.as_mut() {
        if cancel {
            // The first Esc only closes the suggestions.
            prompt.suggest = None;
            app.mode = Mode::Prompt(prompt);
            return;
        }
        if key.code == KeyCode::Down || is_ctrl(&key, 'n') || is_ctrl(&key, 'j') {
            suggest.step(1);
            app.mode = Mode::Prompt(prompt);
            return;
        }
        if key.code == KeyCode::Up || is_ctrl(&key, 'p') || is_ctrl(&key, 'k') {
            suggest.step(-1);
            app.mode = Mode::Prompt(prompt);
            return;
        }
        if key.code == KeyCode::Tab || key.code == KeyCode::Enter {
            let (text, cursor) = suggest.accept(&text);
            prompt.set_text(&text, cursor);
            prompt.suggest = None;
            app.mode = Mode::Prompt(prompt);
            return;
        }
    }

    if cancel {
        if searching {
            app.search.clear();
            app.rebuild();
        }
        return;
    }
    if key.code == KeyCode::Enter {
        app.submit_prompt(prompt);
        return;
    }

    prompt.input.input(key);
    if searching {
        // Filter as you type.
        app.search = prompt.text();
        app.rebuild();
    }
    if adding {
        let projects: Vec<&str> = app
            .store
            .projects
            .iter()
            .map(|project| project.name.as_str())
            .collect();
        prompt.suggest = quick_add::suggest(
            &prompt.text(),
            prompt.cursor(),
            &projects,
            &app.store.labels(),
        );
    }
    app.mode = Mode::Prompt(prompt);
}

fn picker_key(app: &mut App, mut picker: Picker, key: KeyEvent) {
    if key.code == KeyCode::Esc || is_ctrl(&key, 'g') {
        return;
    }
    if key.code == KeyCode::Enter {
        app.choose(picker);
        return;
    }

    if key.code == KeyCode::Down || is_ctrl(&key, 'n') || is_ctrl(&key, 'j') {
        picker.step(1);
    } else if key.code == KeyCode::Up || is_ctrl(&key, 'p') || is_ctrl(&key, 'k') {
        picker.step(-1);
    } else {
        picker.input.input(key);
        picker.refilter();
    }
    app.mode = Mode::Picker(picker);
}

fn edit_key(app: &mut App, mut edit: Box<TaskEdit>, key: KeyEvent) {
    if edit.confirm_discard {
        match key.code {
            KeyCode::Char('y') => return,
            _ => edit.confirm_discard = false,
        }
        app.mode = Mode::Edit(edit);
        return;
    }

    // `C-c C-c` saves and `C-c C-k` aborts, as in an Emacs capture buffer.
    if edit.chord {
        edit.chord = false;
        if is_ctrl(&key, 'c') {
            if app.save_edit(&edit) {
                return;
            }
        } else if is_ctrl(&key, 'k') {
            return;
        }
        app.mode = Mode::Edit(edit);
        return;
    }

    if is_ctrl(&key, 'c') {
        edit.chord = true;
    } else if is_ctrl(&key, 's') {
        if app.save_edit(&edit) {
            return;
        }
    } else {
        match key.code {
            KeyCode::Esc => {
                if !edit.is_dirty() {
                    return;
                }
                edit.confirm_discard = true;
            }
            KeyCode::Tab => edit.focus_next(),
            KeyCode::BackTab => edit.focus_previous(),
            KeyCode::Enter if edit.focus == Field::Description => {
                edit.description.insert_newline();
            }
            KeyCode::Enter => {
                if app.save_edit(&edit) {
                    return;
                }
            }
            _ if edit.focus == Field::Priority => match key.code {
                KeyCode::Char(c @ '1'..='4') => edit.priority = c as u8 - b'0',
                KeyCode::Left | KeyCode::Char('h') => edit.priority = edit.priority.max(2) - 1,
                KeyCode::Right | KeyCode::Char('l') => edit.priority = (edit.priority + 1).min(4),
                _ => {}
            },
            _ => {
                if let Some(area) = edit.active_area() {
                    area.input(key);
                }
            }
        }
    }
    app.mode = Mode::Edit(edit);
}

fn overlay_key(app: &mut App, overlay: Overlay, key: KeyEvent) {
    let page = 10;
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => return,
        KeyCode::Char('j') | KeyCode::Down => {
            app.overlay_scroll = app.overlay_scroll.saturating_add(1)
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.overlay_scroll = app.overlay_scroll.saturating_sub(1)
        }
        KeyCode::Char('g') => app.overlay_scroll = 0,
        KeyCode::Char('G') => app.overlay_scroll = u16::MAX / 2,
        _ if is_ctrl(&key, 'd') => app.overlay_scroll = app.overlay_scroll.saturating_add(page),
        _ if is_ctrl(&key, 'u') => app.overlay_scroll = app.overlay_scroll.saturating_sub(page),
        _ => {}
    }
    app.mode = Mode::Overlay(overlay);
}
