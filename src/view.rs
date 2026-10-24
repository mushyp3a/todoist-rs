//! What is shown: which tasks a view contains and how they are laid out as rows.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{Datelike, NaiveDate, NaiveTime};

use crate::{projects::Project, store::Store, tasks::Task};

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Inbox,
    Today,
    Upcoming,
    All,
    Overdue,
    Project(String),
    Label(String),
}

impl View {
    pub fn title(&self, store: &Store) -> String {
        match self {
            View::Inbox => "Inbox".into(),
            View::Today => "Today".into(),
            View::Upcoming => "Upcoming".into(),
            View::All => "All tasks".into(),
            View::Overdue => "Overdue".into(),
            View::Project(id) => store.project_name(id).to_string(),
            View::Label(label) => format!("@{label}"),
        }
    }

    pub fn includes(&self, task: &Task, store: &Store, today: NaiveDate) -> bool {
        match self {
            View::Inbox => store.inbox_id() == Some(task.project_id.as_str()),
            View::Today => task.due.as_ref().is_some_and(|due| due.date <= today),
            View::Upcoming => task.due.is_some(),
            View::All => true,
            View::Overdue => task.due.as_ref().is_some_and(|due| due.date < today),
            View::Project(id) => task.project_id == *id,
            View::Label(label) => task.labels.contains(label),
        }
    }

    /// Dated views group tasks by day instead of by section or project.
    fn is_dated(&self) -> bool {
        matches!(self, View::Today | View::Upcoming | View::Overdue)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sort {
    Natural,
    Date,
    Priority,
}

impl Sort {
    pub fn label(self) -> &'static str {
        match self {
            Sort::Natural => "natural",
            Sort::Date => "date",
            Sort::Priority => "priority",
        }
    }

    pub fn next(self) -> Sort {
        match self {
            Sort::Natural => Sort::Date,
            Sort::Date => Sort::Priority,
            Sort::Priority => Sort::Natural,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeaderKind {
    Plain,
    Overdue,
    Today,
}

#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: String,
    pub depth: u16,
    pub has_children: bool,
    pub collapsed: bool,
    /// Sub-tasks hidden because this row is collapsed.
    pub hidden: usize,
}

#[derive(Debug, Clone)]
pub enum Row {
    Header { text: String, kind: HeaderKind },
    Task(TaskRow),
}

impl Row {
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Row::Task(row) => Some(&row.id),
            Row::Header { .. } => None,
        }
    }
}

pub struct RowQuery<'a> {
    pub view: &'a View,
    pub search: &'a str,
    pub sort: Sort,
    pub collapsed: &'a HashSet<String>,
    pub today: NaiveDate,
}

pub fn build_rows(store: &Store, query: &RowQuery) -> Vec<Row> {
    let searching = !query.search.is_empty();
    let candidates: Vec<&Task> = store
        .tasks
        .iter()
        .filter(|task| {
            if searching {
                matches_search(task, store, query.search)
            } else {
                query.view.includes(task, store, query.today)
            }
        })
        .collect();

    if !searching && query.view.is_dated() {
        rows_by_day(candidates, query)
    } else {
        rows_by_group(store, candidates, query, searching)
    }
}

fn matches_search(task: &Task, store: &Store, needle: &str) -> bool {
    let needle = needle.to_lowercase();
    task.content.to_lowercase().contains(&needle)
        || task.description.to_lowercase().contains(&needle)
        || task
            .labels
            .iter()
            .any(|label| label.to_lowercase().contains(&needle))
        || store
            .project_name(&task.project_id)
            .to_lowercase()
            .contains(&needle)
}

/// "Today", "Tomorrow", or e.g. "Mon 26 Oct".
pub fn day_label(date: NaiveDate, today: NaiveDate) -> String {
    let weekday_date = if date.year() == today.year() {
        date.format("%a %-d %b").to_string()
    } else {
        date.format("%a %-d %b %Y").to_string()
    };
    match (date - today).num_days() {
        0 => format!("Today · {weekday_date}"),
        1 => format!("Tomorrow · {weekday_date}"),
        _ => weekday_date,
    }
}

fn task_row(task: &Task) -> Row {
    Row::Task(TaskRow {
        id: task.id.clone(),
        depth: 0,
        has_children: false,
        collapsed: false,
        hidden: 0,
    })
}

fn rows_by_day(candidates: Vec<&Task>, query: &RowQuery) -> Vec<Row> {
    let today = query.today;
    let mut overdue: Vec<&Task> = Vec::new();
    let mut days: BTreeMap<NaiveDate, Vec<&Task>> = BTreeMap::new();
    for task in candidates {
        let Some(due) = &task.due else { continue };
        if due.date < today {
            overdue.push(task);
        } else {
            days.entry(due.date).or_default().push(task);
        }
    }

    let within_day = |a: &&Task, b: &&Task| {
        let key = |task: &Task| {
            let time = task.due.as_ref().and_then(|due| due.time);
            (
                time.unwrap_or(NaiveTime::MIN),
                std::cmp::Reverse(task.priority),
            )
        };
        match query.sort {
            Sort::Priority => (std::cmp::Reverse(a.priority), key(a).0)
                .cmp(&(std::cmp::Reverse(b.priority), key(b).0)),
            _ => key(a).cmp(&key(b)),
        }
        .then(a.child_order.cmp(&b.child_order))
    };

    let mut rows = Vec::new();
    if !overdue.is_empty() {
        overdue.sort_by(|a, b| {
            let date = |task: &Task| task.due.as_ref().map(|due| (due.date, due.time));
            date(a).cmp(&date(b)).then(b.priority.cmp(&a.priority))
        });
        rows.push(Row::Header {
            text: format!("Overdue ({})", overdue.len()),
            kind: HeaderKind::Overdue,
        });
        rows.extend(overdue.into_iter().map(task_row));
    }
    for (date, mut tasks) in days {
        if *query.view == View::Overdue {
            break;
        }
        tasks.sort_by(within_day);
        rows.push(Row::Header {
            text: day_label(date, today),
            kind: if date == today {
                HeaderKind::Today
            } else {
                HeaderKind::Plain
            },
        });
        rows.extend(tasks.into_iter().map(task_row));
    }
    rows
}

/// Projects in sidebar order (inbox first, then the tree depth-first), with depth.
pub fn ordered_projects(store: &Store) -> Vec<(&Project, u16)> {
    fn visit<'a>(
        store: &'a Store,
        parent: Option<&str>,
        depth: u16,
        out: &mut Vec<(&'a Project, u16)>,
    ) {
        let mut children: Vec<&Project> = store
            .projects
            .iter()
            .filter(|project| !project.inbox_project && project.parent_id.as_deref() == parent)
            .collect();
        children.sort_by_key(|project| project.child_order);
        for child in children {
            out.push((child, depth));
            visit(store, Some(&child.id), depth + 1, out);
        }
    }

    let mut out: Vec<(&Project, u16)> = store
        .projects
        .iter()
        .filter(|project| project.inbox_project)
        .map(|project| (project, 0))
        .collect();
    visit(store, None, 0, &mut out);
    out
}

fn rows_by_group(
    store: &Store,
    candidates: Vec<&Task>,
    query: &RowQuery,
    force_projects: bool,
) -> Vec<Row> {
    let mut rows = Vec::new();
    let by_section = !force_projects && matches!(query.view, View::Inbox | View::Project(_));

    if by_section {
        let project_id = match query.view {
            View::Project(id) => Some(id.as_str()),
            _ => store.inbox_id(),
        };
        let in_project = |task: &&Task| Some(task.project_id.as_str()) == project_id;

        let unsectioned: Vec<&Task> = candidates
            .iter()
            .filter(|task| in_project(task) && task.section_id.is_none())
            .copied()
            .collect();
        push_tree(&mut rows, unsectioned, query);

        let mut sections: Vec<_> = store
            .sections
            .iter()
            .filter(|section| Some(section.project_id.as_str()) == project_id)
            .collect();
        sections.sort_by_key(|section| section.section_order);
        for section in sections {
            let tasks: Vec<&Task> = candidates
                .iter()
                .filter(|task| task.section_id.as_deref() == Some(section.id.as_str()))
                .copied()
                .collect();
            if tasks.is_empty() {
                continue;
            }
            rows.push(Row::Header {
                text: section.name.clone(),
                kind: HeaderKind::Plain,
            });
            push_tree(&mut rows, tasks, query);
        }
    } else {
        for (project, _) in ordered_projects(store) {
            let tasks: Vec<&Task> = candidates
                .iter()
                .filter(|task| task.project_id == project.id)
                .copied()
                .collect();
            if tasks.is_empty() {
                continue;
            }
            rows.push(Row::Header {
                text: project.name.clone(),
                kind: HeaderKind::Plain,
            });
            push_tree(&mut rows, tasks, query);
        }
    }
    rows
}

/// Lays `tasks` out as a tree: sub-tasks below their parent, honouring folds.
fn push_tree(rows: &mut Vec<Row>, tasks: Vec<&Task>, query: &RowQuery) {
    let ids: HashSet<&str> = tasks.iter().map(|task| task.id.as_str()).collect();
    let mut children: HashMap<&str, Vec<&Task>> = HashMap::new();
    let mut roots: Vec<&Task> = Vec::new();
    for task in tasks {
        match task.parent_id.as_deref() {
            Some(parent) if ids.contains(parent) => children.entry(parent).or_default().push(task),
            _ => roots.push(task),
        }
    }

    let order = |a: &&Task, b: &&Task| match query.sort {
        Sort::Natural => a.child_order.cmp(&b.child_order),
        Sort::Date => {
            let key = |task: &Task| task.due.as_ref().map(|due| (due.date, due.time));
            // Tasks without a date go last.
            match (key(a), key(b)) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
            .then(a.child_order.cmp(&b.child_order))
        }
        Sort::Priority => b
            .priority
            .cmp(&a.priority)
            .then(a.child_order.cmp(&b.child_order)),
    };
    roots.sort_by(order);
    for list in children.values_mut() {
        list.sort_by(order);
    }

    fn count(children: &HashMap<&str, Vec<&Task>>, id: &str) -> usize {
        children.get(id).map_or(0, |kids| {
            kids.iter().map(|kid| 1 + count(children, &kid.id)).sum()
        })
    }

    fn visit(
        rows: &mut Vec<Row>,
        children: &HashMap<&str, Vec<&Task>>,
        task: &Task,
        depth: u16,
        collapsed: &HashSet<String>,
    ) {
        let kids = children.get(task.id.as_str());
        let is_collapsed = kids.is_some() && collapsed.contains(&task.id);
        rows.push(Row::Task(TaskRow {
            id: task.id.clone(),
            depth,
            has_children: kids.is_some(),
            collapsed: is_collapsed,
            hidden: if is_collapsed {
                count(children, &task.id)
            } else {
                0
            },
        }));
        if let (Some(kids), false) = (kids, is_collapsed) {
            for kid in kids {
                visit(rows, children, kid, depth + 1, collapsed);
            }
        }
    }

    for root in roots {
        visit(rows, &children, root, 0, query.collapsed);
    }
}

#[derive(Debug, Clone)]
pub enum SidebarEntry {
    Header(&'static str),
    Item {
        view: View,
        label: String,
        depth: u16,
        count: usize,
        /// Projects with sub-projects can be folded.
        foldable: bool,
        folded: bool,
    },
}

pub fn sidebar_entries(
    store: &Store,
    folded_projects: &HashSet<String>,
    today: NaiveDate,
) -> Vec<SidebarEntry> {
    let count = |view: &View| {
        store
            .tasks
            .iter()
            .filter(|task| view.includes(task, store, today))
            .count()
    };
    let item = |view: View, label: String, depth: u16, foldable: bool, folded: bool| {
        let count = count(&view);
        SidebarEntry::Item {
            view,
            label,
            depth,
            count,
            foldable,
            folded,
        }
    };

    let mut entries = vec![
        item(View::Inbox, "Inbox".into(), 0, false, false),
        item(View::Today, "Today".into(), 0, false, false),
        item(View::Upcoming, "Upcoming".into(), 0, false, false),
        item(View::All, "All".into(), 0, false, false),
        SidebarEntry::Header("Projects"),
    ];

    let mut hidden_below: Option<u16> = None;
    for (project, depth) in ordered_projects(store) {
        if project.inbox_project {
            continue;
        }
        if let Some(limit) = hidden_below {
            if depth > limit {
                continue;
            }
            hidden_below = None;
        }
        let foldable = store
            .projects
            .iter()
            .any(|other| other.parent_id.as_deref() == Some(project.id.as_str()));
        let folded = foldable && folded_projects.contains(&project.id);
        if folded {
            hidden_below = Some(depth);
        }
        entries.push(item(
            View::Project(project.id.clone()),
            project.name.clone(),
            depth,
            foldable,
            folded,
        ));
    }

    let labels = store.labels();
    if !labels.is_empty() {
        entries.push(SidebarEntry::Header("Labels"));
        for label in labels {
            entries.push(item(
                View::Label(label.clone()),
                format!("@{label}"),
                0,
                false,
                false,
            ));
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{projects::Project, sections::Section};

    fn task(id: &str, project: &str, parent: Option<&str>, order: i32) -> Task {
        serde_json::from_value(serde_json::json!({
            "id": id, "project_id": project, "parent_id": parent,
            "content": id, "child_order": order,
        }))
        .unwrap()
    }

    fn dated(id: &str, date: &str) -> Task {
        serde_json::from_value(serde_json::json!({
            "id": id, "project_id": "p", "content": id,
            "due": {"date": date, "string": date, "is_recurring": false},
        }))
        .unwrap()
    }

    fn store(tasks: Vec<Task>) -> Store {
        Store {
            tasks,
            projects: vec![Project {
                id: "p".into(),
                name: "Proj".into(),
                parent_id: None,
                child_order: 0,
                inbox_project: false,
            }],
            sections: vec![Section {
                id: "s".into(),
                project_id: "p".into(),
                name: "Sec".into(),
                section_order: 0,
            }],
        }
    }

    fn ids(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Header { text, .. } => format!("#{text}"),
                Row::Task(task) => format!("{}{}", "-".repeat(task.depth as usize), task.id),
            })
            .collect()
    }

    fn query<'a>(view: &'a View, collapsed: &'a HashSet<String>) -> RowQuery<'a> {
        RowQuery {
            view,
            search: "",
            sort: Sort::Natural,
            collapsed,
            today: NaiveDate::from_ymd_opt(2026, 10, 24).unwrap(),
        }
    }

    #[test]
    fn subtasks_nest_under_parent_and_sections_get_headers() {
        let mut in_section = task("c", "p", None, 5);
        in_section.section_id = Some("s".into());
        let store = store(vec![
            task("b", "p", Some("a"), 2),
            task("a", "p", None, 1),
            in_section,
        ]);
        let none = HashSet::new();
        let view = View::Project("p".into());
        let rows = build_rows(&store, &query(&view, &none));
        assert_eq!(ids(&rows), ["a", "-b", "#Sec", "c"]);
    }

    #[test]
    fn collapsed_parent_hides_children_and_counts_them() {
        let store = store(vec![task("a", "p", None, 1), task("b", "p", Some("a"), 2)]);
        let collapsed: HashSet<String> = ["a".to_string()].into();
        let view = View::Project("p".into());
        let rows = build_rows(&store, &query(&view, &collapsed));
        assert_eq!(ids(&rows), ["a"]);
        let Row::Task(row) = &rows[0] else { panic!() };
        assert_eq!(row.hidden, 1);
    }

    #[test]
    fn today_lists_overdue_first_then_today() {
        let store = store(vec![
            dated("t", "2026-10-24"),
            dated("o", "2026-10-20"),
            dated("f", "2026-10-30"),
        ]);
        let none = HashSet::new();
        let rows = build_rows(&store, &query(&View::Today, &none));
        assert_eq!(
            ids(&rows),
            ["#Overdue (1)", "o", "#Today · Sat 24 Oct", "t"]
        );
    }

    #[test]
    fn search_ignores_the_view_and_matches_everywhere() {
        let store = store(vec![
            task("alpha", "p", None, 1),
            task("beta", "p", None, 2),
        ]);
        let none = HashSet::new();
        let mut q = query(&View::Today, &none);
        q.search = "ALP";
        assert_eq!(ids(&build_rows(&store, &q)), ["#Proj", "alpha"]);
    }
}
