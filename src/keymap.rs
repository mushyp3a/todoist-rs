//! Every normal-mode key binding, in one table.
//!
//! The table drives dispatch, the which-key popup and the help screen, so they
//! can't disagree. Sequences are space-separated tokens: `g t`, `SPC m e`, `C-d`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Down,
    Up,
    Top,
    Bottom,
    HalfPageDown,
    HalfPageUp,
    NextHeader,
    PrevHeader,
    /// `h`: collapse the task, or move to the sidebar.
    Left,
    /// `l`: expand the task, or move to the list.
    Right,
    Open,
    FocusSidebar,
    FocusList,
    ToggleSidebar,
    ToggleFold,
    Fold,
    Unfold,
    FoldAll,
    UnfoldAll,
    Edit,
    Complete,
    Delete,
    Undo,
    Priority(u8),
    Postpone(i64),
    SetDue,
    Rename,
    EditLabels,
    Move,
    Add,
    AddSubtask,
    Search,
    Cancel,
    GotoToday,
    GotoUpcoming,
    GotoInbox,
    GotoAll,
    GotoOverdue,
    Jump,
    Refresh,
    SortNatural,
    SortDate,
    SortPriority,
    CycleSort,
    Help,
    Messages,
    Quit,
}

use Action::*;

pub const BINDINGS: &[(&str, &str, Action)] = &[
    // Movement
    ("j", "next line", Down),
    ("down", "next line", Down),
    ("k", "previous line", Up),
    ("up", "previous line", Up),
    ("g g", "top", Top),
    ("G", "bottom", Bottom),
    ("C-d", "half page down", HalfPageDown),
    ("C-u", "half page up", HalfPageUp),
    ("}", "next section", NextHeader),
    ("{", "previous section", PrevHeader),
    ("h", "collapse / sidebar", Left),
    ("left", "collapse / sidebar", Left),
    ("l", "expand / list", Right),
    ("right", "expand / list", Right),
    ("RET", "open", Open),
    // Folding
    ("TAB", "toggle fold", ToggleFold),
    ("z a", "toggle fold", ToggleFold),
    ("z o", "open fold", Unfold),
    ("z c", "close fold", Fold),
    ("z R", "open all folds", UnfoldAll),
    ("z M", "close all folds", FoldAll),
    // Tasks
    ("e", "edit task", Edit),
    ("x", "complete", Complete),
    ("d d", "delete", Delete),
    ("u", "undo", Undo),
    ("1", "priority 1", Priority(1)),
    ("2", "priority 2", Priority(2)),
    ("3", "priority 3", Priority(3)),
    ("4", "priority 4", Priority(4)),
    (">", "postpone a day", Postpone(1)),
    ("<", "move a day earlier", Postpone(-1)),
    ("D", "set due date", SetDue),
    ("r", "rename", Rename),
    ("T", "edit labels", EditLabels),
    ("m", "move to project", Move),
    ("a", "add task", Add),
    ("A", "add sub-task", AddSubtask),
    // Search and views
    ("/", "search", Search),
    ("ESC", "cancel / clear search", Cancel),
    ("g t", "today", GotoToday),
    ("g u", "upcoming", GotoUpcoming),
    ("g i", "inbox", GotoInbox),
    ("g a", "all tasks", GotoAll),
    ("g o", "overdue", GotoOverdue),
    ("g p", "jump to project / label", Jump),
    ("g r", "refresh", Refresh),
    ("s n", "sort: natural", SortNatural),
    ("s d", "sort: date", SortDate),
    ("s p", "sort: priority", SortPriority),
    ("?", "help", Help),
    ("q", "quit", Quit),
    // Leader
    ("SPC SPC", "jump to view / project / label", Jump),
    ("SPC /", "search", Search),
    ("SPC a", "add task", Add),
    ("SPC A", "add sub-task", AddSubtask),
    ("SPC u", "undo", Undo),
    ("SPC r r", "refresh", Refresh),
    ("SPC t s", "toggle sidebar", ToggleSidebar),
    ("SPC t o", "cycle sort order", CycleSort),
    ("SPC w h", "focus sidebar", FocusSidebar),
    ("SPC w l", "focus list", FocusList),
    ("SPC m e", "edit", Edit),
    ("SPC m d", "due date", SetDue),
    ("SPC m r", "rename", Rename),
    ("SPC m t", "labels", EditLabels),
    ("SPC m m", "move to project", Move),
    ("SPC m x", "complete", Complete),
    ("SPC m k", "delete", Delete),
    ("SPC m a", "add sub-task", AddSubtask),
    ("SPC m >", "postpone a day", Postpone(1)),
    ("SPC m <", "move a day earlier", Postpone(-1)),
    ("SPC m 1", "priority 1", Priority(1)),
    ("SPC m 2", "priority 2", Priority(2)),
    ("SPC m 3", "priority 3", Priority(3)),
    ("SPC m 4", "priority 4", Priority(4)),
    ("SPC h h", "help", Help),
    ("SPC h m", "messages", Messages),
    ("SPC q q", "quit", Quit),
];

/// Names of key prefixes, shown by which-key as `+name`.
const GROUPS: &[(&str, &str)] = &[
    ("SPC", "leader"),
    ("SPC m", "task"),
    ("SPC t", "toggle"),
    ("SPC w", "window"),
    ("SPC r", "refresh"),
    ("SPC h", "help"),
    ("SPC q", "quit"),
    ("g", "go"),
    ("s", "sort"),
    ("z", "fold"),
    ("d", "delete"),
];

pub enum Lookup {
    Action(Action),
    /// A valid prefix; more keys are needed.
    Pending,
    Unbound,
}

fn tokens(sequence: &str) -> impl Iterator<Item = &str> {
    sequence.split(' ')
}

pub fn lookup(pending: &[String]) -> Lookup {
    let mut is_prefix = false;
    for (sequence, _, action) in BINDINGS {
        let mut sequence_tokens = tokens(sequence);
        let mut matched = 0;
        for want in pending {
            match sequence_tokens.next() {
                Some(token) if token == want => matched += 1,
                _ => break,
            }
        }
        if matched < pending.len() {
            continue;
        }
        if sequence_tokens.next().is_none() {
            return Lookup::Action(*action);
        }
        is_prefix = true;
    }
    if is_prefix {
        Lookup::Pending
    } else {
        Lookup::Unbound
    }
}

pub struct Hint {
    pub key: String,
    pub text: String,
}

/// What can follow `pending`: the which-key popup's entries.
pub fn hints(pending: &[String]) -> Vec<Hint> {
    let mut hints: Vec<Hint> = Vec::new();
    for (sequence, description, _) in BINDINGS {
        let parts: Vec<&str> = tokens(sequence).collect();
        if parts.len() <= pending.len() || !parts.iter().zip(pending).all(|(a, b)| a == b) {
            continue;
        }
        let key = parts[pending.len()].to_string();
        if hints.iter().any(|hint| hint.key == key) {
            continue;
        }
        let text = if parts.len() == pending.len() + 1 {
            description.to_string()
        } else {
            let prefix = parts[..=pending.len()].join(" ");
            let name = GROUPS
                .iter()
                .find(|(group, _)| *group == prefix)
                .map_or("more", |(_, name)| *name);
            format!("+{name}")
        };
        hints.push(Hint { key, text });
    }
    hints
}

/// Turns a key press into a binding token; `None` for keys with no token.
pub fn token(key: &KeyEvent) -> Option<String> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    Some(match key.code {
        KeyCode::Char(' ') if !ctrl => "SPC".into(),
        KeyCode::Char(c) if ctrl => format!("C-{}", c.to_ascii_lowercase()),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "RET".into(),
        KeyCode::Tab => "TAB".into(),
        KeyCode::Esc => "ESC".into(),
        KeyCode::Up => "up".into(),
        KeyCode::Down => "down".into(),
        KeyCode::Left => "left".into(),
        KeyCode::Right => "right".into(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|key| key.to_string()).collect()
    }

    #[test]
    fn single_and_multi_key_bindings_resolve() {
        assert!(matches!(
            lookup(&seq(&["x"])),
            Lookup::Action(Action::Complete)
        ));
        assert!(matches!(lookup(&seq(&["g"])), Lookup::Pending));
        assert!(matches!(
            lookup(&seq(&["g", "t"])),
            Lookup::Action(Action::GotoToday)
        ));
        assert!(matches!(
            lookup(&seq(&["SPC", "m", "e"])),
            Lookup::Action(Action::Edit)
        ));
        assert!(matches!(lookup(&seq(&["g", "z"])), Lookup::Unbound));
    }

    #[test]
    fn which_key_lists_next_keys_and_groups() {
        let leader = hints(&seq(&["SPC"]));
        assert!(leader
            .iter()
            .any(|hint| hint.key == "m" && hint.text == "+task"));
        assert!(leader
            .iter()
            .any(|hint| hint.key == "a" && hint.text == "add task"));
        let go = hints(&seq(&["g"]));
        assert!(go
            .iter()
            .any(|hint| hint.key == "t" && hint.text == "today"));
    }

    #[test]
    fn no_sequence_is_both_a_binding_and_a_prefix() {
        for (a, _, _) in BINDINGS {
            for (b, _, _) in BINDINGS {
                assert!(
                    a == b || !b.starts_with(&format!("{a} ")),
                    "{a} shadows {b}"
                );
            }
        }
    }
}
