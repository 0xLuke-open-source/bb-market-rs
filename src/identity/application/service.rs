use std::fmt::Write as _;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::identity::application::dto::{
    AuthCommand, AuthSessionResult, AuthStatusView, AuthUserView, SubscriptionPlanView,
};
use crate::identity::domain::{
    AuthRepository, NewSession, NewUserAccount, SubscriptionPlan, UserAccount,
};

const SESSION_TTL_DAYS: i64 = 7;
const PUBLIC_SYMBOL_LIMIT: usize = 10;

#[derive(Clone)]
pub struct AuthApplicationService {
    repository: Arc<dyn AuthRepository>,
}

impl AuthApplicationService {
    pub fn new(repository: Arc<dyn AuthRepository>) -> Self {
        Self { repository }
    }

    pub async fn ensure_ready(&self) -> Result<()> {
        self.repository.ensure_schema().await
    }

    pub async fn register(&self, req: AuthCommand) -> Result<AuthSessionResult> {
        let username = normalize_username(&req.username)?;
        validate_password(&req.password)?;
        let display_name = normalize_display_name(req.display_name.as_deref(), &username)?;

        if self
            .repository
            .find_user_by_username(&username)
            .await?
            .is_some()
        {
            return Err(anyhow!("用户名已存在"));
        }

        let salt = Uuid::new_v4().to_string();
        let password_hash = hash_password(&salt, &req.password);
        let user = NewUserAccount {
            user_id: Uuid::new_v4(),
            username: username.clone(),
            display_name: display_name.clone(),
            salt,
            password_hash,
        };
        let session = NewSession {
            session_token: Uuid::new_v4().to_string(),
            user_id: user.user_id,
            username: username.clone(),
            expires_at: Utc::now() + Duration::days(SESSION_TTL_DAYS),
        };
        let persisted = self
            .repository
            .register_user_with_session(user, session.clone())
            .await?;

        Ok(AuthSessionResult {
            status: auth_status(Some(auth_user(&persisted)), Some(&persisted)),
            session_token: session.session_token,
        })
    }

    pub async fn login(&self, req: AuthCommand) -> Result<AuthSessionResult> {
        let username = normalize_username(&req.username)?;
        let user = self
            .repository
            .find_user_by_username(&username)
            .await?
            .ok_or_else(|| anyhow!("用户名或密码错误"))?;

        if hash_password(&user.salt, &req.password) != user.password_hash {
            return Err(anyhow!("用户名或密码错误"));
        }

        let session = NewSession {
            session_token: Uuid::new_v4().to_string(),
            user_id: user.user_id,
            username: user.username.clone(),
            expires_at: Utc::now() + Duration::days(SESSION_TTL_DAYS),
        };
        self.repository.create_session(session.clone()).await?;

        Ok(AuthSessionResult {
            status: auth_status(Some(auth_user(&user)), Some(&user)),
            session_token: session.session_token,
        })
    }

    pub async fn me_by_token(&self, token: Option<&str>) -> Result<AuthStatusView> {
        let Some(token) = token else {
            return Ok(auth_status(None, None));
        };
        let user = self.repository.find_user_by_session_token(token).await?;
        Ok(match user {
            Some(user) => auth_status(Some(auth_user(&user)), Some(&user)),
            None => auth_status(None, None),
        })
    }

    pub async fn logout(&self, token: &str) -> Result<()> {
        self.repository.delete_session(token).await
    }

    pub async fn plans(&self) -> Result<Vec<SubscriptionPlanView>> {
        let plans = self.repository.list_active_plans().await?;
        Ok(plans.into_iter().map(plan_to_view).collect())
    }

    pub async fn subscribe(&self, token: &str, plan_code: &str) -> Result<AuthStatusView> {
        let plan = self
            .repository
            .list_active_plans()
            .await?
            .into_iter()
            .find(|plan| plan.code == plan_code)
            .ok_or_else(|| anyhow!("无效的订阅套餐"))?;
        let expires_at = Utc::now() + Duration::days(plan.days);
        let user = self
            .repository
            .activate_subscription(token, plan_code, expires_at)
            .await?;
        Ok(auth_status(Some(auth_user(&user)), Some(&user)))
    }

    pub async fn list_favorite_symbols(&self, token: &str) -> Result<Vec<String>> {
        self.repository.list_favorite_symbols(token).await
    }

    pub async fn add_favorite_symbol(&self, token: &str, symbol: &str) -> Result<Vec<String>> {
        let normalized = normalize_symbol(symbol)?;
        self.repository
            .add_favorite_symbol(token, &normalized)
            .await
    }

    pub async fn remove_favorite_symbol(&self, token: &str, symbol: &str) -> Result<Vec<String>> {
        let normalized = normalize_symbol(symbol)?;
        self.repository
            .remove_favorite_symbol(token, &normalized)
            .await
    }
}

fn auth_user(user: &UserAccount) -> AuthUserView {
    AuthUserView {
        username: user.username.clone(),
        display_name: user.display_name.clone(),
    }
}

fn plan_to_view(plan: SubscriptionPlan) -> SubscriptionPlanView {
    SubscriptionPlanView {
        code: plan.code,
        label: plan.label,
        days: plan.days,
        price_text: plan.price_text,
        description: plan.description,
    }
}

fn normalize_username(input: &str) -> Result<String> {
    let username = input.trim().to_ascii_lowercase();
    if username.len() < 3 || username.len() > 32 {
        return Err(anyhow!("用户名长度需要在 3 到 32 之间"));
    }
    if !username
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        return Err(anyhow!("用户名只能包含字母、数字、点、下划线和中划线"));
    }
    Ok(username)
}

fn normalize_display_name(input: Option<&str>, fallback: &str) -> Result<String> {
    let display_name = input.unwrap_or("").trim();
    if display_name.is_empty() {
        return Ok(fallback.to_string());
    }
    if display_name.chars().count() > 32 {
        return Err(anyhow!("显示名称不能超过 32 个字符"));
    }
    Ok(display_name.to_string())
}

fn validate_password(password: &str) -> Result<()> {
    if password.len() < 6 {
        return Err(anyhow!("密码至少需要 6 位"));
    }
    if password.len() > 128 {
        return Err(anyhow!("密码长度不能超过 128 位"));
    }
    Ok(())
}

fn normalize_symbol(input: &str) -> Result<String> {
    let normalized = input
        .trim()
        .to_ascii_uppercase()
        .replace([' ', '/', '_', '-'], "");
    if normalized.is_empty() {
        return Err(anyhow!("币种不能为空"));
    }
    let symbol = if normalized.ends_with("USDT") {
        normalized
    } else {
        format!("{normalized}USDT")
    };
    if symbol.len() > 32 {
        return Err(anyhow!("币种长度不合法"));
    }
    if !symbol.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err(anyhow!("币种格式不合法"));
    }
    Ok(symbol)
}

fn hash_password(salt: &str, password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut out, "{:02x}", byte);
    }
    out
}

fn auth_status(user: Option<AuthUserView>, stored_user: Option<&UserAccount>) -> AuthStatusView {
    let now = Utc::now();
    let (subscribed, full_access, subscription_plan, subscription_expires_at) =
        subscription_state(stored_user, now);
    AuthStatusView {
        authenticated: user.is_some(),
        subscribed,
        full_access: user.is_some() && full_access,
        symbol_limit: if user.is_some() && full_access {
            None
        } else {
            Some(PUBLIC_SYMBOL_LIMIT)
        },
        subscription_plan,
        subscription_expires_at,
        user,
    }
}

fn subscription_state(
    stored_user: Option<&UserAccount>,
    now: chrono::DateTime<Utc>,
) -> (bool, bool, Option<String>, Option<String>) {
    let Some(user) = stored_user else {
        return (false, false, None, None);
    };

    let full_access = user.has_full_access(now);
    let subscribed = user.is_subscribed(now);
    (
        subscribed,
        full_access,
        user.subscription_plan.clone(),
        user.subscription_expires_at.map(|value| value.to_rfc3339()),
    )
}
