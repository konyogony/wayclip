use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct ClipData {
    pub name: String,
    pub path: String,
    pub length: f64,
    pub size: u64,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
    pub tags: Vec<Tag>,
    pub liked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tag {
    pub name: String,
    pub color: String,
}

#[derive(Deserialize, Clone)]
pub struct ClipJsonData {
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub liked: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Payload {
    pub message: String,
}
