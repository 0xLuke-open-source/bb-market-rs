//! 本地认证服务。
//!
//! 这一层实现项目自己的最小用户体系：
//! - 注册 / 登录
//! - 基于 Cookie 的会话
//! - 当前登录用户查询
//!
//! 目标是保护 Dashboard 和模拟交易接口，而不是提供完整 IAM 能力。

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use uuid::Uuid;

const SESSION_COOKIE_NAME: &str = "bbm_session";
const SESSION_TTL_DAYS: i64 = 7;
const PUBLIC_SYMBOL_LIMIT: usize = 10;

#[derive(Clone)]
pub struct AuthService {
    inner: Arc<Mutex<AuthState>>,
    users_file: Arc<PathBuf>,
}

struct AuthState {
    users: HashMap<String, StoredUser>,
    sessions: HashMap<String, SessionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredUser {
    username: String,
    display_name: String,
    salt: String,
    password_hash: String,
    #[serde(default)]
    subscribed: bool,
    #[serde(default)]
    subscription_plan: Option<String>,
    #[serde(default)]
    subscription_expires_at: Option<String>,
    created_at: String,
}

#[derive(Debug, Clone)]
struct SessionRecord {
    username: String,
    expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedUsers {
    users: Vec<StoredUser>,
}

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

impl AuthService {
    /// 从本地目录加载认证服务。
    ///
    /// 当前只持久化用户，不持久化 session；进程重启后需要重新登录。
    pub fn new(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        fs::create_dir_all(&data_dir)?;
        let users_file = data_dir.join("users.json");

        let persisted: PersistedUsers = if users_file.exists() {
            let raw = fs::read_to_string(&users_file)?;
            serde_json::from_str(&raw)?
        } else {
            PersistedUsers::default()
        };

        let mut users = HashMap::new();
        for user in persisted.users {
            users.insert(user.username.clone(), user);
        }

        Ok(Self {
            inner: Arc::new(Mutex::new(AuthState {
                users,
                sessions: HashMap::new(),
            })),
            users_file: Arc::new(users_file),
        })
    }

    /// 注册新用户，并立即创建登录会话。
    pub async fn register(&self, req: AuthRequest) -> Result<(AuthStatusResponse, String)> {
        let username = normalize_username(&req.username)?;
        validate_password(&req.password)?;
        let display_name = normalize_display_name(req.display_name.as_deref(), &username)?;

        let salt = Uuid::new_v4().to_string();
        let password_hash = hash_password(&salt, &req.password);
        let user = StoredUser {
            username: username.clone(),
            display_name: display_name.clone(),
            salt,
            password_hash,
            subscribed: false,
            subscription_plan: None,
            subscription_expires_at: None,
            created_at: Utc::now().to_rfc3339(),
        };

        let mut guard = self.inner.lock().await;
        if guard.users.contains_key(&username) {
            return Err(anyhow!("用户名已存在"));
        }
        guard.users.insert(username.clone(), user);
        self.persist_users(&guard.users)?;

        let token = create_session(&mut guard.sessions, &username, &display_name);
        Ok((
            auth_status(
                Some(AuthUserJson {
                    username,
                    display_name,
                }),
                None,
            ),
            token,
        ))
    }

    /// 校验用户名密码，并创建登录会话。
    pub async fn login(&self, req: AuthRequest) -> Result<(AuthStatusResponse, String)> {
        let username = normalize_username(&req.username)?;

        let mut guard = self.inner.lock().await;
        let user = guard
            .users
            .get(&username)
            .cloned()
            .ok_or_else(|| anyhow!("用户名或密码错误"))?;

        if hash_password(&user.salt, &req.password) != user.password_hash {
            return Err(anyhow!("用户名或密码错误"));
        }

        let token = create_session(&mut guard.sessions, &user.username, &user.display_name);
        Ok((
            auth_status(
                Some(AuthUserJson {
                    username: user.username.clone(),
                    display_name: user.display_name.clone(),
                }),
                Some(&user),
            ),
            token,
        ))
    }

    /// 用于前端启动时查询当前登录状态。
    pub async fn me_from_headers(&self, headers: &HeaderMap) -> AuthStatusResponse {
        self.status_from_headers(headers).await
    }

    /// 从请求头中取出 session token。
    pub fn session_token_from_headers(&self, headers: &HeaderMap) -> Option<String> {
        extract_cookie(headers, SESSION_COOKIE_NAME)
    }

    /// 注销当前会话。
    pub async fn logout(&self, token: &str) {
        let mut guard = self.inner.lock().await;
        guard.sessions.remove(token);
    }

    /// 返回当前支持的套餐列表。
    pub fn plans(&self) -> Vec<SubscriptionPlanJson> {
        subscription_plans()
    }

    /// 激活当前用户的本地订阅状态。
    pub async fn subscribe(&self, token: &str, plan_code: &str) -> Result<AuthStatusResponse> {
        let plan = subscription_plan_by_code(plan_code)?;
        let mut guard = self.inner.lock().await;
        cleanup_expired_sessions(&mut guard.sessions);
        let session = guard
            .sessions
            .get(token)
            .cloned()
            .ok_or_else(|| anyhow!("请先登录"))?;
        let user = guard
            .users
            .get_mut(&session.username)
            .ok_or_else(|| anyhow!("用户不存在"))?;
        user.subscribed = true;
        user.subscription_plan = Some(plan.code.clone());
        user.subscription_expires_at = Some((Utc::now() + Duration::days(plan.days)).to_rfc3339());
        let display_name = user.display_name.clone();
        let username = user.username.clone();
        let user_snapshot = user.clone();
        self.persist_users(&guard.users)?;
        Ok(auth_status(
            Some(AuthUserJson {
                username,
                display_name,
            }),
            Some(&user_snapshot),
        ))
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

    /// 读取当前请求对应的完整访问状态。
    pub async fn status_from_headers(&self, headers: &HeaderMap) -> AuthStatusResponse {
        let Some(token) = extract_cookie(headers, SESSION_COOKIE_NAME) else {
            return auth_status(None, None);
        };

        let mut guard = self.inner.lock().await;
        cleanup_expired_sessions(&mut guard.sessions);
        let Some(session) = guard.sessions.get(&token) else {
            return auth_status(None, None);
        };
        let Some(user) = guard.users.get(&session.username) else {
            return auth_status(None, None);
        };

        auth_status(
            Some(AuthUserJson {
                username: user.username.clone(),
                display_name: user.display_name.clone(),
            }),
            Some(user),
        )
    }

    fn persist_users(&self, users: &HashMap<String, StoredUser>) -> Result<()> {
        let mut list: Vec<StoredUser> = users.values().cloned().collect();
        list.sort_by(|left, right| left.username.cmp(&right.username));
        let raw = serde_json::to_string_pretty(&PersistedUsers { users: list })?;
        fs::write(self.users_file.as_ref(), raw)?;
        Ok(())
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

fn create_session(
    sessions: &mut HashMap<String, SessionRecord>,
    username: &str,
    _display_name: &str,
) -> String {
    cleanup_expired_sessions(sessions);
    let token = Uuid::new_v4().to_string();
    sessions.insert(
        token.clone(),
        SessionRecord {
            username: username.to_string(),
            expires_at: Utc::now() + Duration::days(SESSION_TTL_DAYS),
        },
    );
    token
}

fn cleanup_expired_sessions(sessions: &mut HashMap<String, SessionRecord>) {
    let now = Utc::now();
    sessions.retain(|_, session| session.expires_at > now);
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

fn auth_status(user: Option<AuthUserJson>, stored_user: Option<&StoredUser>) -> AuthStatusResponse {
    let (subscribed, full_access, subscription_plan, subscription_expires_at) =
        subscription_state(stored_user);
    AuthStatusResponse {
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
    stored_user: Option<&StoredUser>,
) -> (bool, bool, Option<String>, Option<String>) {
    let Some(user) = stored_user else {
        return (false, false, None, None);
    };

    if let Some(expires_at) = user.subscription_expires_at.as_deref() {
        if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(expires_at) {
            let active = expiry.with_timezone(&Utc) > Utc::now();
            return (
                active,
                active,
                user.subscription_plan.clone(),
                Some(expires_at.to_string()),
            );
        }
    }

    if user.subscribed {
        return (
            true,
            true,
            user.subscription_plan
                .clone()
                .or_else(|| Some("legacy".to_string())),
            user.subscription_expires_at.clone(),
        );
    }

    (
        false,
        false,
        user.subscription_plan.clone(),
        user.subscription_expires_at.clone(),
    )
}

fn subscription_plans() -> Vec<SubscriptionPlanJson> {
    vec![
        SubscriptionPlanJson {
            code: "pro_week".to_string(),
            label: "PRO 周卡".to_string(),
            days: 7,
            price_text: "98 USDT".to_string(),
            description: "适合短期盯盘，解锁全部币种、全量信号和实时推送。".to_string(),
        },
        SubscriptionPlanJson {
            code: "pro_month".to_string(),
            label: "PRO 月卡".to_string(),
            days: 30,
            price_text: "298 USDT".to_string(),
            description: "适合日常交易使用，30 天内查看全量市场和完整快照。".to_string(),
        },
        SubscriptionPlanJson {
            code: "pro_year".to_string(),
            label: "PRO 年卡".to_string(),
            days: 365,
            price_text: "1998 USDT".to_string(),
            description: "适合长期使用，全年解锁全部币种和完整实时数据。".to_string(),
        },
    ]
}

fn subscription_plan_by_code(code: &str) -> Result<SubscriptionPlanJson> {
    subscription_plans()
        .into_iter()
        .find(|plan| plan.code == code)
        .ok_or_else(|| anyhow!("无效的订阅套餐"))
}
