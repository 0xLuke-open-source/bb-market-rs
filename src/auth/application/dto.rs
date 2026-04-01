use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthUserJson {
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthStatusResponse {
    pub authenticated: bool,
    pub subscribed: bool,
    pub full_access: bool,
    pub symbol_limit: Option<usize>,
    pub subscription_plan: Option<String>,
    pub subscription_expires_at: Option<String>,
    pub user: Option<AuthUserJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubscribeRequest {
    pub plan_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionPlanJson {
    pub code: String,
    pub label: String,
    pub days: i64,
    pub price_text: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSessionResult {
    pub status: AuthStatusResponse,
    pub session_token: String,
}
