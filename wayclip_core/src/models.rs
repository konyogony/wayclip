use chrono::{DateTime, Utc};
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "subscription_tier", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionTier {
    Free,
    Paid,
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
}
