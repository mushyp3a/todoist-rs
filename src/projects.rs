use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub child_order: i32,
    #[serde(default)]
    pub inbox_project: bool,
}
