use std::collections::BTreeSet;

use crate::{projects::Project, sections::Section, tasks::Task};

/// Everything fetched from Todoist.
#[derive(Debug, Default)]
pub struct Store {
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub sections: Vec<Section>,
}

impl Store {
    pub fn task(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == id)
    }

    pub fn task_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|task| task.id == id)
    }

    /// Replaces the task with the same id, or adds it.
    pub fn upsert(&mut self, task: Task) {
        match self.task_mut(&task.id) {
            Some(existing) => *existing = task,
            None => self.tasks.push(task),
        }
    }

    pub fn remove(&mut self, id: &str) -> Option<Task> {
        let index = self.tasks.iter().position(|task| task.id == id)?;
        Some(self.tasks.remove(index))
    }

    /// Ids of every sub-task below `id`, at any depth.
    pub fn descendants(&self, id: &str) -> Vec<String> {
        let mut found = Vec::new();
        let mut queue = vec![id.to_string()];
        while let Some(parent) = queue.pop() {
            for task in &self.tasks {
                if task.parent_id.as_deref() == Some(parent.as_str()) {
                    found.push(task.id.clone());
                    queue.push(task.id.clone());
                }
            }
        }
        found
    }

    pub fn project(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|project| project.id == id)
    }

    pub fn project_name(&self, id: &str) -> &str {
        self.project(id)
            .map_or("?", |project| project.name.as_str())
    }

    pub fn inbox_id(&self) -> Option<&str> {
        self.projects
            .iter()
            .find(|project| project.inbox_project)
            .map(|project| project.id.as_str())
    }

    /// Every label in use, sorted.
    pub fn labels(&self) -> Vec<String> {
        let labels: BTreeSet<&String> = self.tasks.iter().flat_map(|task| &task.labels).collect();
        labels.into_iter().cloned().collect()
    }
}
