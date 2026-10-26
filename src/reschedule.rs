//! The reschedule popup: quick presets and a month calendar for picking a due date.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

/// A date picked for a task, or `None` to clear its due date.
pub type Choice = Option<NaiveDate>;

pub struct Reschedule {
    pub task_id: String,
    pub cursor: NaiveDate,
    pub today: NaiveDate,
    /// The task's due date before rescheduling.
    pub current: Option<NaiveDate>,
}

impl Reschedule {
    pub fn new(task_id: String, today: NaiveDate, current: Option<NaiveDate>) -> Self {
        Reschedule {
            task_id,
            cursor: current.unwrap_or(today),
            today,
            current,
        }
    }

    pub fn move_days(&mut self, days: i64) {
        self.cursor += Duration::days(days);
    }

    pub fn move_months(&mut self, months: i32) {
        self.cursor = shift_months(self.cursor, months);
    }
}

/// Preset keys, their labels, and the date each picks (`None` clears the date).
pub fn presets(today: NaiveDate) -> [(char, &'static str, Choice); 6] {
    [
        ('t', "today", Some(today)),
        ('m', "tomorrow", Some(today + Duration::days(1))),
        ('w', "monday", Some(next_weekday(today, Weekday::Mon))),
        ('n', "+1 week", Some(today + Duration::days(7))),
        ('s', "weekend", Some(weekend(today))),
        ('x', "no date", None),
    ]
}

/// The first `weekday` after `date`.
fn next_weekday(date: NaiveDate, weekday: Weekday) -> NaiveDate {
    let ahead = (weekday.num_days_from_monday() + 7 - date.weekday().num_days_from_monday()) % 7;
    date + Duration::days(if ahead == 0 { 7 } else { ahead as i64 })
}

/// The coming Saturday, or `date` itself if it is already the weekend.
fn weekend(date: NaiveDate) -> NaiveDate {
    match date.weekday() {
        Weekday::Sat | Weekday::Sun => date,
        _ => next_weekday(date, Weekday::Sat),
    }
}

/// `date` moved by whole months, keeping the day where the month is long enough.
pub fn shift_months(date: NaiveDate, months: i32) -> NaiveDate {
    let index = date.year() * 12 + date.month0() as i32 + months;
    let (year, month) = (index.div_euclid(12), index.rem_euclid(12) as u32 + 1);
    (1..=date.day())
        .rev()
        .find_map(|day| NaiveDate::from_ymd_opt(year, month, day))
        .unwrap_or(date)
}

/// The weeks (Monday first) that cover `date`'s month; days outside it are `None`.
pub fn month_grid(date: NaiveDate) -> Vec<[Option<NaiveDate>; 7]> {
    let first = date.with_day(1).expect("day 1 exists");
    let mut weeks = Vec::new();
    let mut day = first;
    while day.month() == first.month() {
        let mut week = [None; 7];
        let mut slot = day.weekday().num_days_from_monday() as usize;
        while slot < 7 && day.month() == first.month() {
            week[slot] = Some(day);
            day += Duration::days(1);
            slot += 1;
        }
        weeks.push(week);
    }
    weeks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn months_clamp_to_the_end_of_a_short_month() {
        assert_eq!(shift_months(date(2026, 1, 31), 1), date(2026, 2, 28));
        assert_eq!(shift_months(date(2028, 1, 31), 1), date(2028, 2, 29));
        assert_eq!(shift_months(date(2026, 3, 31), -1), date(2026, 2, 28));
    }

    #[test]
    fn months_cross_year_boundaries() {
        assert_eq!(shift_months(date(2026, 12, 15), 1), date(2027, 1, 15));
        assert_eq!(shift_months(date(2026, 1, 15), -1), date(2025, 12, 15));
        assert_eq!(shift_months(date(2026, 6, 15), 18), date(2027, 12, 15));
    }

    #[test]
    fn presets_land_on_the_expected_days() {
        // 2026-10-28 is a Wednesday.
        let p = presets(date(2026, 10, 28));
        assert_eq!(p[0].2, Some(date(2026, 10, 28)));
        assert_eq!(p[1].2, Some(date(2026, 10, 29)));
        assert_eq!(p[2].2, Some(date(2026, 11, 2)));
        assert_eq!(p[3].2, Some(date(2026, 11, 4)));
        assert_eq!(p[4].2, Some(date(2026, 10, 31)));
        assert_eq!(p[5].2, None);
    }

    #[test]
    fn monday_is_always_in_the_future() {
        assert_eq!(next_weekday(date(2026, 10, 26), Weekday::Mon), date(2026, 11, 2));
        assert_eq!(next_weekday(date(2026, 10, 25), Weekday::Mon), date(2026, 10, 26));
    }

    #[test]
    fn weekend_stays_put_on_saturday_and_sunday() {
        assert_eq!(weekend(date(2026, 10, 31)), date(2026, 10, 31));
        assert_eq!(weekend(date(2026, 11, 1)), date(2026, 11, 1));
        assert_eq!(weekend(date(2026, 10, 26)), date(2026, 10, 31));
    }

    #[test]
    fn grid_covers_the_month_monday_first() {
        // October 2026 starts on a Thursday and has 31 days.
        let grid = month_grid(date(2026, 10, 15));
        assert_eq!(grid.len(), 5);
        assert_eq!(grid[0][..3], [None, None, None]);
        assert_eq!(grid[0][3], Some(date(2026, 10, 1)));
        assert_eq!(grid[4][5], Some(date(2026, 10, 31)));
        assert_eq!(grid[4][6], None);
        let days = grid.iter().flatten().flatten().count();
        assert_eq!(days, 31);
    }

    #[test]
    fn grid_handles_a_month_that_fills_six_weeks() {
        // August 2026 starts on a Saturday and has 31 days.
        assert_eq!(month_grid(date(2026, 8, 1)).len(), 6);
    }

    #[test]
    fn cursor_starts_on_the_current_due_date() {
        let r = Reschedule::new("1".into(), date(2026, 10, 25), Some(date(2026, 12, 1)));
        assert_eq!(r.cursor, date(2026, 12, 1));
        let r = Reschedule::new("1".into(), date(2026, 10, 25), None);
        assert_eq!(r.cursor, date(2026, 10, 25));
    }
}
