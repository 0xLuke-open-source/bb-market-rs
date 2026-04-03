#[derive(Debug, Clone, Default)]
pub struct AuthUserView {
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Default)]
pub struct AuthStatusView {
    pub authenticated: bool,
    pub subscribed: bool,
    pub full_access: bool,
    pub symbol_limit: Option<usize>,
    pub subscription_plan: Option<String>,
    pub subscription_expires_at: Option<String>,
    pub user: Option<AuthUserView>,
}

#[derive(Debug, Clone, Default)]
pub struct AuthCommand {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionPlanView {
    pub code: String,
    pub label: String,
    pub days: i64,
    pub price_text: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct AuthSessionResult {
    pub status: AuthStatusView,
    pub session_token: String,
}
