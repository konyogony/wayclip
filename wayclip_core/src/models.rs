use crate::ClipJsonData;
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow, Deserialize, Clone)]
pub struct User {
    pub id: Uuid,
    pub github_id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
    pub tier: SubscriptionTier,
    pub created_at: DateTime<Utc>,
    pub is_banned: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "subscription_tier", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionTier {
    Free,
    Tier1,
    Tier2,
    Tier3,
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Clip {
    pub id: Uuid,
    pub user_id: Uuid,
    pub file_name: String,
    pub file_size: i64,
    pub public_url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserProfile {
    #[serde(flatten)]
    pub user: User,
    pub storage_used: i64,
    pub storage_limit: i64,
    pub clip_count: i64,
}

#[derive(Debug, Serialize, FromRow, Deserialize, Clone)]
pub struct HostedClipInfo {
    pub id: Uuid,
    pub file_name: String,
}

#[derive(Debug)]
pub struct UnifiedClipData {
    pub name: String,
    pub full_filename: String,

    pub local_path: Option<String>,
    pub local_data: Option<ClipJsonData>,
    pub created_at: DateTime<Local>,

    pub is_hosted: bool,
    pub hosted_id: Option<uuid::Uuid>,
}
