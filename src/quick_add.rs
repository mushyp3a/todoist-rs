//! Best-effort mirror of Todoist's quick-add syntax, used to highlight what
//! will be recognised and to complete `#project` and `@label` names. Todoist
//! parses the text server-side, so this is cosmetic and never authoritative.

use std::ops::Range;

use crate::picker::fuzzy_score;

const MAX_SUGGESTIONS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Due,
    /// Todoist's p1 (highest) to p4.
    Priority(u8),
    Project,
    Label,
}

/// The highlighted parts of `line` as byte ranges, in order and without overlap.
/// Text that means nothing to Todoist is left out.
pub fn tokenize(line: &str) -> Vec<(Range<usize>, TokenKind)> {
    let words = words(line);
    let mut tokens: Vec<Option<TokenKind>> = vec![None; words.len()];
    for (i, word) in words.iter().enumerate() {
        let text = &line[word.clone()];
        tokens[i] = if text.len() > 1 && text.starts_with('#') {
            Some(TokenKind::Project)
        } else if text.len() > 1 && text.starts_with('@') {
            Some(TokenKind::Label)
        } else {
            priority(text).map(TokenKind::Priority)
        };
    }

    let free: Vec<bool> = tokens.iter().map(Option::is_none).collect();
    let lower: Vec<String> = words
        .iter()
        .map(|word| {
            line[word.clone()]
                .trim_end_matches([',', '.', ';', '!'])
                .to_lowercase()
        })
        .collect();
    let classes = classify(&lower, &free);

    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if let Some(kind) = tokens[i] {
            out.push((words[i].clone(), kind));
            i += 1;
            continue;
        }
        if classes[i] == Class::Other {
            i += 1;
            continue;
        }
        let mut end = i;
        while end < words.len() && classes[end] != Class::Other {
            end += 1;
        }
        // Connectors at the edges belong to the surrounding text, except
        // leading ones like "on" and "every" that introduce a date.
        let mut first = i;
        let mut last = end;
        while first < last && matches!(lower[first].as_str(), "and" | "of" | "from" | "until") {
            first += 1;
        }
        while last > first && classes[last - 1] == Class::Weak {
            last -= 1;
        }
        if (first..last).any(|k| classes[k] == Class::Strong) {
            out.push((words[first].start..words[last - 1].end, TokenKind::Due));
        }
        i = end;
    }
    out
}

/// Whitespace-separated words as byte ranges. A `#"quoted project"` is one word.
fn words(line: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut chars = line.char_indices().peekable();
    while let Some(&(start, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        let quoted = line[start..].starts_with("#\"");
        let mut end = line.len();
        let mut open = quoted;
        let mut skip = if quoted { 2 } else { 0 };
        while let Some(&(at, c)) = chars.peek() {
            if skip > 0 {
                skip -= 1;
                chars.next();
                continue;
            }
            if c == '"' && open {
                open = false;
            } else if c.is_whitespace() && !open {
                end = at;
                break;
            }
            chars.next();
        }
        out.push(start..end);
    }
    out
}

fn priority(word: &str) -> Option<u8> {
    let lower = word.to_ascii_lowercase();
    match lower.as_bytes() {
        [b'p', level @ b'1'..=b'4'] => Some(level - b'0'),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    /// Clearly part of a date or time.
    Strong,
    /// Joins date words ("next", "at", "on") but means nothing alone.
    Weak,
    Other,
}

const WEEKDAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];
const WEEKDAY_SHORT: [&str; 10] = [
    "mon", "tue", "tues", "wed", "thu", "thur", "thurs", "fri", "sat", "sun",
];
const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];
const MONTH_SHORT: [&str; 9] = [
    "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep", "sept",
];
const UNITS: [&str; 14] = [
    "day", "days", "week", "weeks", "month", "months", "year", "years", "hour", "hours", "min",
    "mins", "minute", "minutes",
];

fn is_month(word: &str) -> bool {
    MONTHS.contains(&word) || MONTH_SHORT.contains(&word) || matches!(word, "oct" | "nov" | "dec")
}

fn is_number(word: &str) -> bool {
    !word.is_empty() && word.len() <= 2 && word.bytes().all(|b| b.is_ascii_digit())
}

/// Dates and times that are unambiguous on their own.
fn is_date_word(word: &str) -> bool {
    matches!(
        word,
        "today"
            | "tomorrow"
            | "tmr"
            | "tmrw"
            | "tonight"
            | "yesterday"
            | "noon"
            | "midnight"
            | "daily"
            | "weekly"
            | "monthly"
            | "yearly"
            | "weekdays"
            | "weekend"
            | "workdays"
    ) || WEEKDAYS.contains(&word)
        || WEEKDAY_SHORT.contains(&word)
        || is_time(word)
        || is_numeric_date(word)
        || is_ordinal(word)
}

/// "5pm", "5:30pm", "17:30".
fn is_time(word: &str) -> bool {
    let body = word
        .strip_suffix("am")
        .or_else(|| word.strip_suffix("pm"));
    let (digits, suffixed) = match body {
        Some(body) => (body, true),
        None => (word, false),
    };
    let (hours, minutes) = match digits.split_once(':') {
        Some((h, m)) => (h, Some(m)),
        None => (digits, None),
    };
    let numeric = |s: &str, max: usize| {
        !s.is_empty() && s.len() <= max && s.bytes().all(|b| b.is_ascii_digit())
    };
    if !numeric(hours, 2) {
        return false;
    }
    match minutes {
        Some(m) => numeric(m, 2),
        None => suffixed,
    }
}

/// "2026-10-24", "24/10", "24/10/2026".
fn is_numeric_date(word: &str) -> bool {
    for sep in ['-', '/'] {
        let parts: Vec<&str> = word.split(sep).collect();
        if (2..=3).contains(&parts.len())
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.len() <= 4 && p.bytes().all(|b| b.is_ascii_digit()))
        {
            return true;
        }
    }
    false
}

/// "1st", "22nd".
fn is_ordinal(word: &str) -> bool {
    word.strip_suffix("st")
        .or_else(|| word.strip_suffix("nd"))
        .or_else(|| word.strip_suffix("rd"))
        .or_else(|| word.strip_suffix("th"))
        .is_some_and(is_number)
}

fn classify(lower: &[String], free: &[bool]) -> Vec<Class> {
    let word = |i: usize| lower.get(i).map_or("", String::as_str);
    let before = |i: usize| if i == 0 { "" } else { word(i - 1) };
    let mut classes = vec![Class::Other; lower.len()];
    for i in 0..lower.len() {
        if !free[i] {
            continue;
        }
        let w = word(i);
        classes[i] = if is_date_word(w) {
            Class::Strong
        } else if is_month(w) {
            // "may" is also a verb, so it needs a day next to it.
            if w != "may" || is_number(before(i)) || is_number(word(i + 1)) {
                Class::Strong
            } else {
                Class::Other
            }
        } else if is_number(w) {
            let next = word(i + 1);
            let prev = before(i);
            if is_month(prev)
                || is_month(next)
                || prev == "at"
                || (matches!(prev, "in" | "every" | "next" | "ev" | "after") && UNITS.contains(&next))
            {
                Class::Strong
            } else {
                Class::Other
            }
        } else if UNITS.contains(&w) {
            let after_number = i > 0 && is_number(before(i)) && classes[i - 1] == Class::Strong;
            if after_number || matches!(before(i), "next" | "this" | "every" | "ev") {
                Class::Strong
            } else {
                Class::Other
            }
        } else if matches!(
            w,
            "next" | "this" | "every" | "ev" | "at" | "on" | "in" | "by" | "and" | "of" | "from"
                | "until" | "after" | "the"
        ) {
            Class::Weak
        } else {
            Class::Other
        };
    }
    classes
}

/// Names that complete the `#` or `@` word under the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestions {
    /// Byte range of the whole word being completed, sigil included.
    pub range: Range<usize>,
    pub sigil: char,
    pub items: Vec<String>,
    pub selected: usize,
}

/// Suggestions for the `#project` or `@label` word the cursor is in, if any.
/// `cursor` counts chars, as the text area reports it.
pub fn suggest(
    line: &str,
    cursor: usize,
    projects: &[&str],
    labels: &[String],
) -> Option<Suggestions> {
    let cursor_byte = line
        .char_indices()
        .nth(cursor)
        .map_or(line.len(), |(at, _)| at);
    let range = words(line)
        .into_iter()
        .find(|word| word.start < cursor_byte && cursor_byte <= word.end)?;
    let word = &line[range.clone()];
    let sigil = word.chars().next().filter(|c| matches!(c, '#' | '@'))?;
    let typed = &word[1..cursor_byte - range.start];
    let query = typed.strip_prefix('"').unwrap_or(typed);
    // A closed quote means the name is already complete.
    if typed.len() > 1 && typed.starts_with('"') && typed.ends_with('"') {
        return None;
    }

    let names: Vec<&str> = match sigil {
        '#' => projects.to_vec(),
        _ => labels.iter().map(String::as_str).collect(),
    };
    let mut scored: Vec<(i32, &str)> = names
        .into_iter()
        .filter_map(|name| fuzzy_score(query, name).map(|score| (score, name)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase())));
    scored.dedup_by(|a, b| a.1 == b.1);
    let items: Vec<String> = scored
        .into_iter()
        .take(MAX_SUGGESTIONS)
        .map(|(_, name)| name.to_string())
        .collect();

    // Nothing left to complete.
    if items.is_empty() || (items.len() == 1 && items[0].eq_ignore_ascii_case(query)) {
        return None;
    }
    Some(Suggestions {
        range,
        sigil,
        items,
        selected: 0,
    })
}

impl Suggestions {
    pub fn step(&mut self, delta: isize) {
        let len = self.items.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(len) as usize;
    }

    /// The line with the selected name filled in, and the cursor (in chars)
    /// just after it.
    pub fn accept(&self, line: &str) -> (String, usize) {
        let name = &self.items[self.selected];
        let word = if name.contains(char::is_whitespace) {
            format!("{}\"{name}\"", self.sigil)
        } else {
            format!("{}{name}", self.sigil)
        };
        let rest = &line[self.range.end..];
        let gap = if rest.starts_with(char::is_whitespace) { "" } else { " " };
        let before = &line[..self.range.start];
        let cursor = before.chars().count() + word.chars().count() + gap.len();
        (format!("{before}{word}{gap}{rest}"), cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str) -> Vec<(&str, TokenKind)> {
        tokenize(line)
            .into_iter()
            .map(|(range, kind)| (&line[range], kind))
            .collect()
    }

    #[test]
    fn highlights_every_kind() {
        assert_eq!(
            kinds("buy milk tomorrow 5pm p1 #shopping @errands"),
            vec![
                ("tomorrow 5pm", TokenKind::Due),
                ("p1", TokenKind::Priority(1)),
                ("#shopping", TokenKind::Project),
                ("@errands", TokenKind::Label),
            ]
        );
    }

    #[test]
    fn joins_date_and_time() {
        assert_eq!(kinds("call today 2pm"), vec![("today 2pm", TokenKind::Due)]);
        assert_eq!(
            kinds("gym every friday at 6:30pm please"),
            vec![("every friday at 6:30pm", TokenKind::Due)]
        );
        assert_eq!(kinds("pay rent 1st"), vec![("1st", TokenKind::Due)]);
        assert_eq!(kinds("trip 24 oct"), vec![("24 oct", TokenKind::Due)]);
        assert_eq!(
            kinds("water plants in 3 days"),
            vec![("in 3 days", TokenKind::Due)]
        );
        assert_eq!(kinds("due 2026-10-24"), vec![("2026-10-24", TokenKind::Due)]);
    }

    #[test]
    fn ignores_plain_words() {
        assert!(kinds("buy 3 apples in the fridge").is_empty());
        assert!(kinds("I may go").is_empty());
        assert_eq!(kinds("may 5"), vec![("may 5", TokenKind::Due)]);
        // A trailing connector is not part of the date.
        assert_eq!(kinds("today and"), vec![("today", TokenKind::Due)]);
    }

    #[test]
    fn priority_needs_a_whole_word() {
        assert!(kinds("p5 app1 p12").is_empty());
        assert_eq!(kinds("P2"), vec![("P2", TokenKind::Priority(2))]);
    }

    #[test]
    fn sigils_must_start_a_word() {
        assert!(kinds("mail a@b.com about c#").is_empty());
        assert_eq!(
            kinds("#\"My Project\" x"),
            vec![("#\"My Project\"", TokenKind::Project)]
        );
        // Still typing the quoted name.
        assert_eq!(kinds("#\"My Pro"), vec![("#\"My Pro", TokenKind::Project)]);
    }

    #[test]
    fn multibyte_text_keeps_char_boundaries() {
        assert_eq!(
            kinds("café ☕ tomorrow #café"),
            vec![
                ("tomorrow", TokenKind::Due),
                ("#café", TokenKind::Project)
            ]
        );
    }

    const PROJECTS: [&str; 3] = ["Shopping", "Work", "Side Projects"];

    fn labels() -> Vec<String> {
        vec!["errands".into(), "email".into()]
    }

    #[test]
    fn suggests_projects_by_prefix_and_fuzzily() {
        let s = suggest("buy milk #sh", 12, &PROJECTS, &labels()).unwrap();
        assert_eq!(s.sigil, '#');
        assert_eq!(s.items[0], "Shopping");
        let s = suggest("#wk", 3, &PROJECTS, &labels()).unwrap();
        assert_eq!(s.items, vec!["Work"]);
    }

    #[test]
    fn suggests_labels() {
        let s = suggest("x @e", 4, &PROJECTS, &labels()).unwrap();
        assert_eq!(s.items, vec!["email", "errands"]);
    }

    #[test]
    fn no_suggestions_outside_a_sigil_word() {
        assert!(suggest("buy milk", 8, &PROJECTS, &labels()).is_none());
        assert!(suggest("#sh milk", 8, &PROJECTS, &labels()).is_none());
        assert!(suggest("#zzz", 4, &PROJECTS, &labels()).is_none());
        // Already complete.
        assert!(suggest("#Work", 5, &PROJECTS, &labels()).is_none());
    }

    #[test]
    fn accept_replaces_the_word() {
        let s = suggest("buy #sh", 7, &PROJECTS, &labels()).unwrap();
        assert_eq!(s.accept("buy #sh"), ("buy #Shopping ".to_string(), 14));
    }

    #[test]
    fn accept_quotes_names_with_spaces_and_keeps_the_rest() {
        let line = "#side tomorrow";
        let s = suggest(line, 5, &PROJECTS, &labels()).unwrap();
        let (text, cursor) = s.accept(line);
        assert_eq!(text, "#\"Side Projects\" tomorrow");
        assert_eq!(cursor, 16);
    }

    #[test]
    fn suggests_while_inside_a_quoted_name() {
        let s = suggest("#\"Side Pr", 9, &PROJECTS, &labels()).unwrap();
        assert_eq!(s.items, vec!["Side Projects"]);
    }

    #[test]
    fn step_wraps() {
        let mut s = suggest("@e", 2, &PROJECTS, &labels()).unwrap();
        s.step(-1);
        assert_eq!(s.selected, 1);
        s.step(1);
        assert_eq!(s.selected, 0);
    }
}
