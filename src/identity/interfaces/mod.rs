use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::identity::application::{
    AuthApplicationService, AuthCommand, AuthStatusView, AuthUserView, SubscriptionPlanView,
};

const SESSION_COOKIE_NAME: &str = "bbm_session";
const SESSION_TTL_DAYS: i64 = 7;

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

#[derive(Clone)]
pub struct AuthService {
    application: AuthApplicationService,
}

impl AuthService {
    pub fn new(application: AuthApplicationService) -> Self {
        Self { application }
    }

    pub async fn register(&self, req: AuthRequest) -> anyhow::Result<(AuthStatusResponse, String)> {
        let result = self.application.register(req.into()).await?;
        Ok((result.status.into(), result.session_token))
    }

    pub async fn login(&self, req: AuthRequest) -> anyhow::Result<(AuthStatusResponse, String)> {
        let result = self.application.login(req.into()).await?;
        Ok((result.status.into(), result.session_token))
    }

    pub async fn me_from_headers(&self, headers: &HeaderMap) -> AuthStatusResponse {
        self.status_from_headers(headers).await.into()
    }

    pub fn session_token_from_headers(&self, headers: &HeaderMap) -> Option<String> {
        extract_cookie(headers, SESSION_COOKIE_NAME)
    }

    pub async fn logout(&self, token: &str) {
        let _ = self.application.logout(token).await;
    }

    pub async fn plans(&self) -> Vec<SubscriptionPlanJson> {
        self.application
            .plans()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(Into::into)
            .collect()
    }

    pub async fn subscribe(
        &self,
        token: &str,
        plan_code: &str,
    ) -> anyhow::Result<AuthStatusResponse> {
        Ok(self.application.subscribe(token, plan_code).await?.into())
    }

    pub async fn favorite_symbols(&self, token: &str) -> anyhow::Result<Vec<String>> {
        self.application.list_favorite_symbols(token).await
    }

    pub async fn add_favorite_symbol(
        &self,
        token: &str,
        symbol: &str,
    ) -> anyhow::Result<Vec<String>> {
        self.application.add_favorite_symbol(token, symbol).await
    }

    pub async fn remove_favorite_symbol(
        &self,
        token: &str,
        symbol: &str,
    ) -> anyhow::Result<Vec<String>> {
        self.application.remove_favorite_symbol(token, symbol).await
    }

    pub fn session_cookie(token: &str) -> String {
        format!(
            "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
            SESSION_COOKIE_NAME,
            token,
            SESSION_TTL_DAYS * 24 * 60 * 60
        )
    }

    pub fn clear_cookie() -> String {
        format!(
            "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
            SESSION_COOKIE_NAME
        )
    }

    pub async fn status_from_headers(&self, headers: &HeaderMap) -> AuthStatusView {
        let token = extract_cookie(headers, SESSION_COOKIE_NAME);
        self.application
            .me_by_token(token.as_deref())
            .await
            .unwrap_or_default()
    }
}

fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let item = part.trim();
        let (key, value) = item.split_once('=')?;
        if key == name {
            return Some(value.to_string());
        }
    }
    None
}

impl From<AuthRequest> for AuthCommand {
    fn from(value: AuthRequest) -> Self {
        Self {
            username: value.username,
            password: value.password,
            display_name: value.display_name,
        }
    }
}

impl From<AuthUserView> for AuthUserJson {
    fn from(value: AuthUserView) -> Self {
        Self {
            username: value.username,
            display_name: value.display_name,
        }
    }
}

impl From<AuthStatusView> for AuthStatusResponse {
    fn from(value: AuthStatusView) -> Self {
        Self {
            authenticated: value.authenticated,
            subscribed: value.subscribed,
            full_access: value.full_access,
            symbol_limit: value.symbol_limit,
            subscription_plan: value.subscription_plan,
            subscription_expires_at: value.subscription_expires_at,
            user: value.user.map(Into::into),
        }
    }
}

impl From<SubscriptionPlanView> for SubscriptionPlanJson {
    fn from(value: SubscriptionPlanView) -> Self {
        Self {
            code: value.code,
            label: value.label,
            days: value.days,
            price_text: value.price_text,
            description: value.description,
        }
    }
}
