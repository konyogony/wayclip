use serde::Deserialize;
use std::collections::HashMap;
use wayclip_core::models::SubscriptionTier;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub storage_type: String,
    pub public_url: String,
    pub local_storage_path: Option<String>,
    pub discord_webhook_url: Option<String>,
    pub sftp_host: Option<String>,
    pub sftp_port: Option<u16>,
    pub sftp_user: Option<String>,
    pub sftp_password: Option<String>,
    pub sftp_remote_path: Option<String>,
    pub sftp_public_url: Option<String>,

    pub limit_free: String,
    pub limit_tier1: String,
    pub limit_tier2: String,
    pub limit_tier3: String,
}

impl Settings {
    pub fn new() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::Environment::default())
            .build()?;
        settings.try_deserialize()
    }

    pub fn get_tier_limits(&self) -> HashMap<SubscriptionTier, i64> {
        let mut map = HashMap::new();
        map.insert(
            SubscriptionTier::Free,
            parse_size(&self.limit_free).expect("Invalid format for LIMIT_FREE"),
        );
        map.insert(
            SubscriptionTier::Tier1,
            parse_size(&self.limit_tier1).expect("Invalid format for LIMIT_TIER1"),
        );
        map.insert(
            SubscriptionTier::Tier2,
            parse_size(&self.limit_tier2).expect("Invalid format for LIMIT_TIER2"),
        );
        map.insert(
            SubscriptionTier::Tier3,
            parse_size(&self.limit_tier3).expect("Invalid format for LIMIT_TIER3"),
        );
        map
    }
}

fn parse_size(size_str: &str) -> Result<i64, String> {
    let s = size_str.trim().to_uppercase();
    let (value_str, unit) = s.split_at(s.trim_end_matches(|c: char| c.is_alphabetic()).len());

    let value = value_str
        .parse::<i64>()
        .map_err(|_| format!("Invalid number in size string: '{}'", size_str))?;

    const KB: i64 = 1024;
    const MB: i64 = 1024 * KB;
    const GB: i64 = 1024 * MB;
    const TB: i64 = 1024 * GB;

    match unit {
        "B" | "" => Ok(value),
        "KB" => Ok(value * KB),
        "MB" => Ok(value * MB),
        "GB" => Ok(value * GB),
        "TB" => Ok(value * TB),
        _ => Err(format!(
            "Unknown size unit '{}' in string '{}'",
            unit, size_str
        )),
    }
}
