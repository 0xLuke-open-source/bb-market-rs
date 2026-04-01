use std::sync::Arc;

use anyhow::Result;
use axum::http::HeaderMap;

use crate::auth::application::AuthApplicationService;
use crate::auth::infrastructure::postgres::PostgresAuthRepository;
use crate::postgres::PgPool;

const SESSION_COOKIE_NAME: &str = "bbm_session";
const SESSION_TTL_DAYS: i64 = 7;

#[derive(Clone)]
pub struct AuthService {
    application: AuthApplicationService,
}

impl AuthService {
    pub async fn new(pool: Arc<PgPool>) -> Result<Self> {
        let repository = Arc::new(PostgresAuthRepository::new(pool));
        let application = AuthApplicationService::new(repository);
        application.ensure_ready().await?;
        Ok(Self { application })
    }

    pub async fn register(
        &self,
        req: crate::auth::application::AuthRequest,
    ) -> Result<(crate::auth::application::AuthStatusResponse, String)> {
        let result = self.application.register(req).await?;
        Ok((result.status, result.session_token))
    }

    pub async fn login(
        &self,
        req: crate::auth::application::AuthRequest,
    ) -> Result<(crate::auth::application::AuthStatusResponse, String)> {
        let result = self.application.login(req).await?;
        Ok((result.status, result.session_token))
    }

    pub async fn me_from_headers(
        &self,
        headers: &HeaderMap,
    ) -> crate::auth::application::AuthStatusResponse {
        self.status_from_headers(headers).await
    }

    pub fn session_token_from_headers(&self, headers: &HeaderMap) -> Option<String> {
        extract_cookie(headers, SESSION_COOKIE_NAME)
    }

    pub async fn logout(&self, token: &str) {
        let _ = self.application.logout(token).await;
    }

    pub async fn plans(&self) -> Vec<crate::auth::application::SubscriptionPlanJson> {
        self.application.plans().await.unwrap_or_default()
    }

    pub async fn subscribe(
        &self,
        token: &str,
        plan_code: &str,
    ) -> Result<crate::auth::application::AuthStatusResponse> {
        self.application.subscribe(token, plan_code).await
    }

    pub async fn favorite_symbols(&self, token: &str) -> Result<Vec<String>> {
        self.application.list_favorite_symbols(token).await
    }

    pub async fn add_favorite_symbol(&self, token: &str, symbol: &str) -> Result<Vec<String>> {
        self.application.add_favorite_symbol(token, symbol).await
    }

    pub async fn remove_favorite_symbol(&self, token: &str, symbol: &str) -> Result<Vec<String>> {
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

    pub async fn status_from_headers(
        &self,
        headers: &HeaderMap,
    ) -> crate::auth::application::AuthStatusResponse {
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

pub use crate::auth::application::{
    AuthRequest, AuthStatusResponse, SubscribeRequest,
};
