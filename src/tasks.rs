use std::collections::HashMap;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime};
use ratatui::widgets::ListState;
use serde::Deserialize;

#[derive(Debug, Default)]
pub struct Tasks {
    pub tasks: Vec<Task>,
    pub filter: Filter,
    pub state: ListState,
    pub tasks_with_children: HashMap<String, u16>,
    pub display_tasks: Vec<usize>,
}

#[derive(Debug)]
pub enum SortCriterion {
    Priority,
    Date,
}

impl Tasks {
    pub fn new(items: Vec<Task>) -> Tasks {
        Tasks {
            tasks: items,
            filter: Filter::Today,
            state: ListState::default(),
            tasks_with_children: HashMap::new(),
            display_tasks: Vec::new(),
        }
    }

    pub fn find_tasks_with_children(&mut self) {
        self.tasks_with_children.clear();
        for task in &self.tasks {
            if let Some(parent_id) = &task.parent_id {
                *self
                    .tasks_with_children
                    .entry(parent_id.clone())
                    .or_insert(0) += 1;
            }
        }
    }

    pub fn sort_tasks(&mut self, criterion: SortCriterion) {
        match criterion {
            SortCriterion::Priority => {
                self.display_tasks.sort_by(|a, b| {
                    let task_a = &self.tasks[*a];
                    let task_b = &self.tasks[*b];
                    task_b.priority.cmp(&task_a.priority)
                });
            }
            SortCriterion::Date => {
                self.display_tasks.sort_by(|a, b| {
                    let task_a = &self.tasks[*a];
                    let task_b = &self.tasks[*b];
                    match (&task_a.due, &task_b.due) {
                        (Some(due_a), Some(due_b)) => {
                            (due_a.date, due_a.time).cmp(&(due_b.date, due_b.time))
                        }
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
            }
        }
    }

    pub fn filter_task_list(&mut self) {
        self.state = ListState::default();
        self.display_tasks = Vec::new();
        for (index, task) in self.tasks.iter().enumerate() {
            if task.parent_id.is_some() {
                continue;
            }
            match &self.filter {
                Filter::All => {
                    self.display_tasks.push(index);
                }
                Filter::Today => {
                    let today = Local::now().date_naive();
                    if let Some(due) = &task.due {
                        if due.date == today {
                            self.display_tasks.push(index);
                        }
                    }
                }
                Filter::ProjectId(project_id) => {
                    if task.project_id == *project_id {
                        self.display_tasks.push(index);
                    }
                }
                Filter::Overdue => {
                    let today = Local::now().date_naive();
                    if let Some(due) = &task.due {
                        if due.date < today {
                            self.display_tasks.push(index);
                        }
                    }
                }
            }
        }
    }

    pub fn next(&mut self) {
        if self.display_tasks.is_empty() {
            self.state.select(None);
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.display_tasks.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.display_tasks.is_empty() {
            self.state.select(None);
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.display_tasks.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        let offset = self.state.offset();
        self.state.select(None);
        *self.state.offset_mut() = offset;
    }
}

#[derive(Debug, Default)]
pub enum Filter {
    #[default]
    All,
    Today,
    Overdue,
    ProjectId(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Task {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub project_id: String,
    pub section_id: Option<String>,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub labels: Vec<String>,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub priority: u8,
    pub due: Option<Due>,
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Duration {
    pub amount: u32,
    pub unit: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "RawDue")]
pub struct Due {
    /// Todoist's natural-language form of the date, e.g. "every monday".
    pub string: String,
    pub date: NaiveDate,
    /// Local time of day; only set for tasks due at a specific time.
    pub time: Option<NaiveTime>,
    pub is_recurring: bool,
}

#[derive(Deserialize)]
struct RawDue {
    #[serde(default)]
    string: String,
    date: String,
    #[serde(default)]
    is_recurring: bool,
}

impl TryFrom<RawDue> for Due {
    type Error = String;

    fn try_from(raw: RawDue) -> Result<Self, String> {
        let (date, time) = parse_due_date(&raw.date)?;
        Ok(Due {
            string: raw.string,
            date,
            time,
            is_recurring: raw.is_recurring,
        })
    }
}

/// The API sends due dates in three shapes: a plain date, a floating datetime
/// (no timezone, e.g. "tomorrow 5pm") and a UTC datetime (tasks with a timezone).
fn parse_due_date(s: &str) -> Result<(NaiveDate, Option<NaiveTime>), String> {
    if let Ok(utc) = DateTime::parse_from_rfc3339(s) {
        let local = utc.with_timezone(&Local);
        return Ok((local.date_naive(), Some(local.time())));
    }
    if let Ok(floating) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Ok((floating.date(), Some(floating.time())));
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map(|date| (date, None))
        .map_err(|e| format!("invalid due date {s:?}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_date() {
        let (date, time) = parse_due_date("2026-10-28").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 10, 28).unwrap());
        assert_eq!(time, None);
    }

    #[test]
    fn parses_floating_datetime() {
        let (date, time) = parse_due_date("2026-10-25T17:00:00").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 10, 25).unwrap());
        assert_eq!(time, NaiveTime::from_hms_opt(17, 0, 0));
    }

    #[test]
    fn parses_utc_datetime() {
        let (_, time) = parse_due_date("2026-10-24T13:00:00Z").unwrap();
        assert!(time.is_some());
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_due_date("tomorrow").is_err());
    }

    #[test]
    fn task_with_timed_due_deserializes() {
        let json = r#"{"id":"1","content":"x","due":{"date":"2026-10-25T17:00:00","string":"25 Oct 5:00 PM","is_recurring":false}}"#;
        let task: Task = serde_json::from_str(json).unwrap();
        assert_eq!(task.due.unwrap().time, NaiveTime::from_hms_opt(17, 0, 0));
    }
}
