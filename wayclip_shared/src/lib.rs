use std::fs::{create_dir_all, read_to_string, write};

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Settings {
    pub clip_name_formatting: String,
}

impl Settings {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir().unwrap().join("wayclip");
        let settings_path = config_dir.join("settings.json");

        if !config_dir.exists() {
            create_dir_all(&config_dir).unwrap();
        }

        if !settings_path.exists() {
            let default_settings = Settings {
                clip_name_formatting: String::from("%Y%m%d_%H%M%S"),
            };
            write(
                &settings_path,
                serde_json::to_string(&default_settings).unwrap(),
            )
            .unwrap();
            return default_settings;
        }

        Self::load()
    }

    pub fn load() -> Self {
        let config_dir = dirs::config_dir().unwrap().join("wayclip");
        let settings_path = config_dir.join("settings.json");

        let settings_json = read_to_string(&settings_path).unwrap();
        serde_json::from_str(&settings_json).unwrap()
    }

    pub fn update(key: &str, value: &str) {
        let mut settings = Self::load();
        match key {
            "clip_name_formatting" => settings.clip_name_formatting = value.to_string(),
            _ => panic!("invalid key"),
        }

        let config_dir = dirs::config_dir().unwrap().join("wayclip");
        let settings_path = config_dir.join("settings.json");
        write(&settings_path, serde_json::to_string(&settings).unwrap()).unwrap();
    }

    pub fn save(&self) {
        let config_dir = dirs::config_dir().unwrap().join("wayclip");
        let settings_path = config_dir.join("settings.json");

        write(&settings_path, serde_json::to_string(self).unwrap()).unwrap();
    }
}
