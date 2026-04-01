use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub salt: String,
    pub password_hash: String,
    pub subscribed: bool,
    pub subscription_plan: Option<String>,
    pub subscription_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl UserAccount {
    pub fn has_full_access(&self, now: DateTime<Utc>) -> bool {
        if let Some(expires_at) = self.subscription_expires_at {
            return expires_at > now;
        }
        self.subscribed
    }

    pub fn is_subscribed(&self, now: DateTime<Utc>) -> bool {
        self.has_full_access(now)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub session_token: String,
    pub user_id: Uuid,
    pub username: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub code: String,
    pub label: String,
    pub days: i64,
    pub price_text: String,
    pub description: String,
    pub active: bool,
}
