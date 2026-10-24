use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Task {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub project_id: String,
    pub section_id: Option<String>,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub labels: Vec<String>,
    /// The API's scale: 4 is the highest priority (shown as P1), 1 the default.
    #[serde(default)]
    pub priority: u8,
    #[serde(default)]
    pub child_order: i32,
    #[serde(default)]
    pub is_collapsed: bool,
    pub due: Option<Due>,
    pub duration: Option<Duration>,
}

impl Task {
    /// Priority as the user sees it: 1 (highest) to 4 (default).
    pub fn ui_priority(&self) -> u8 {
        5 - self.priority.clamp(1, 4)
    }

    pub fn is_recurring(&self) -> bool {
        self.due.as_ref().is_some_and(|due| due.is_recurring)
    }
}

/// Converts the user-facing priority (1 = highest) to the API's.
pub fn api_priority(ui_priority: u8) -> u8 {
    5 - ui_priority.clamp(1, 4)
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

impl Due {
    /// A due date set locally, before the server has confirmed it.
    pub fn local(date: NaiveDate, time: Option<NaiveTime>) -> Due {
        let mut due = Due {
            string: String::new(),
            date,
            time,
            is_recurring: false,
        };
        due.string = due.api_string();
        due
    }

    /// Text that makes the API recreate this due date.
    pub fn api_string(&self) -> String {
        if self.is_recurring {
            return self.string.clone();
        }
        match self.time {
            Some(time) => format!("{} {}", self.date.format("%Y-%m-%d"), time.format("%H:%M")),
            None => self.date.format("%Y-%m-%d").to_string(),
        }
    }

    pub fn same_as(&self, other: &Due) -> bool {
        self.date == other.date
            && self.time == other.time
            && self.is_recurring == other.is_recurring
    }
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

    #[test]
    fn priority_scales_round_trip() {
        for ui in 1..=4 {
            assert_eq!(5 - api_priority(ui), ui);
        }
    }

    #[test]
    fn local_due_round_trips_through_api_string() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 6).unwrap();
        let due = Due::local(date, NaiveTime::from_hms_opt(17, 0, 0));
        assert_eq!(due.api_string(), "2026-11-06 17:00");
        assert_eq!(Due::local(date, None).api_string(), "2026-11-06");
    }
}
