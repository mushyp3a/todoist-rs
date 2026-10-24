use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Section {
    pub id: String,
    pub project_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub section_order: i32,
}
