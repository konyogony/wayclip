#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Settings {
    pub clip_length_s: u16,
    pub storage_path: String,
    pub enable_sound: bool,
}
