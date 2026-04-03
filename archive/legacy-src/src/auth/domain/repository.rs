use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::auth::domain::{SubscriptionPlan, UserAccount};

#[derive(Debug, Clone)]
pub struct NewUserAccount {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub salt: String,
    pub password_hash: String,
}

#[derive(Debug, Clone)]
pub struct NewSession {
    pub session_token: String,
    pub user_id: Uuid,
    pub username: String,
    pub expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait AuthRepository: Send + Sync {
    async fn ensure_schema(&self) -> Result<()>;

    async fn register_user_with_session(
        &self,
        user: NewUserAccount,
        session: NewSession,
    ) -> Result<UserAccount>;

    async fn find_user_by_username(&self, username: &str) -> Result<Option<UserAccount>>;

    async fn create_session(&self, session: NewSession) -> Result<()>;

    async fn find_user_by_session_token(&self, token: &str) -> Result<Option<UserAccount>>;

    async fn delete_session(&self, token: &str) -> Result<()>;

    async fn list_active_plans(&self) -> Result<Vec<SubscriptionPlan>>;

    async fn activate_subscription(
        &self,
        session_token: &str,
        plan_code: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<UserAccount>;

    async fn list_favorite_symbols(&self, session_token: &str) -> Result<Vec<String>>;

    async fn add_favorite_symbol(&self, session_token: &str, symbol: &str) -> Result<Vec<String>>;

    async fn remove_favorite_symbol(
        &self,
        session_token: &str,
        symbol: &str,
    ) -> Result<Vec<String>>;
}
