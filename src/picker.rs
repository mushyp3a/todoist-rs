use tui_textarea::TextArea;

use crate::view::View;

#[derive(Debug, Clone)]
pub enum PickTarget {
    View(View),
    Project(String),
}

#[derive(Debug, Clone)]
pub struct PickItem {
    pub label: String,
    pub target: PickTarget,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PickerKind {
    /// Go to a view, project or label.
    Jump,
    /// Move the task with this id to a project.
    MoveTask(String),
}

/// A fuzzy-filtered list of choices (like Doom's `SPC SPC`).
pub struct Picker {
    pub kind: PickerKind,
    pub title: &'static str,
    pub input: TextArea<'static>,
    pub items: Vec<PickItem>,
    /// Indices into `items`, best match first.
    pub matches: Vec<usize>,
    pub selected: usize,
}

impl Picker {
    pub fn new(kind: PickerKind, title: &'static str, items: Vec<PickItem>) -> Self {
        let mut picker = Picker {
            kind,
            title,
            input: TextArea::default(),
            items,
            matches: Vec::new(),
            selected: 0,
        };
        picker.refilter();
        picker
    }

    pub fn query(&self) -> String {
        self.input.lines().join("")
    }

    pub fn refilter(&mut self) {
        let query = self.query();
        let mut scored: Vec<(i32, usize)> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| fuzzy_score(&query, &item.label).map(|score| (score, i)))
            .collect();
        // Stable sort keeps the original order among equal scores.
        scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        self.matches = scored.into_iter().map(|(_, i)| i).collect();
        self.selected = 0;
    }

    pub fn step(&mut self, delta: isize) {
        if self.matches.is_empty() {
            return;
        }
        let len = self.matches.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(len) as usize;
    }

    pub fn chosen(&self) -> Option<&PickItem> {
        self.matches.get(self.selected).map(|&i| &self.items[i])
    }
}

/// Subsequence match: every query character must appear in order. Consecutive
/// and word-start matches score higher; `None` means no match.
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let mut score = 0;
    let mut pos = 0;
    let mut previous: Option<usize> = None;
    for wanted in query.to_lowercase().chars().filter(|c| !c.is_whitespace()) {
        let found = (pos..text.len()).find(|&i| text[i] == wanted)?;
        score += 1;
        if previous == Some(found.wrapping_sub(1)) {
            score += 3;
        }
        if found == 0 || !text[found - 1].is_alphanumeric() {
            score += 2;
        }
        previous = Some(found);
        pos = found + 1;
    }
    Some(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything() {
        assert!(fuzzy_score("", "anything").is_some());
    }

    #[test]
    fn matches_subsequences_case_insensitively() {
        assert!(fuzzy_score("gdv", "Game Dev").is_some());
        assert!(fuzzy_score("xyz", "Game Dev").is_none());
    }

    #[test]
    fn contiguous_beats_scattered() {
        assert!(fuzzy_score("dev", "game dev") > fuzzy_score("dev", "d e v"));
    }
}
