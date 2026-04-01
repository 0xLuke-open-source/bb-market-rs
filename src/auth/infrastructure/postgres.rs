use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::auth::domain::{
    AuthRepository, NewSession, NewUserAccount, SubscriptionPlan, UserAccount,
};
use crate::postgres::PgPool;

#[derive(Clone)]
pub struct PostgresAuthRepository {
    pool: Arc<PgPool>,
}

impl PostgresAuthRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    async fn resolve_session_user(&self, session_token: &str) -> Result<Uuid> {
        let client = self.pool.acquire().await?;
        let row = client
            .client()
            .query_opt(
                "select user_id
                   from identity.user_session
                  where session_token = $1
                    and expires_at > now()",
                &[&session_token],
            )
            .await?
            .ok_or_else(|| anyhow!("请先登录"))?;
        let user_id: Uuid = row.get("user_id");
        Ok(user_id)
    }

    async fn list_favorite_symbols_by_user_id(&self, user_id: Uuid) -> Result<Vec<String>> {
        let client = self.pool.acquire().await?;
        let rows = client
            .client()
            .query(
                "select symbol
                   from identity.user_favorite_symbol
                  where user_id = $1
                  order by created_at desc, symbol asc",
                &[&user_id],
            )
            .await?;
        Ok(rows.into_iter().map(|row| row.get("symbol")).collect())
    }
}

#[async_trait]
impl AuthRepository for PostgresAuthRepository {
    async fn ensure_schema(&self) -> Result<()> {
        let client = self.pool.acquire().await?;
        client
            .client()
            .batch_execute(include_str!("../../../sql/postgres/auth_identity.sql"))
            .await?;
        Ok(())
    }

    async fn register_user_with_session(
        &self,
        user: NewUserAccount,
        session: NewSession,
    ) -> Result<UserAccount> {
        let mut client = self.pool.acquire().await?;
        let tx = client.client_mut().transaction().await?;

        let row = tx
            .query_opt(
                "select user_id from identity.user_account where username = $1",
                &[&user.username],
            )
            .await?;
        if row.is_some() {
            return Err(anyhow!("用户名已存在"));
        }

        tx.execute(
            "insert into identity.user_account (
                user_id, username, display_name, salt, password_hash,
                subscribed, subscription_plan, subscription_expires_at,
                created_at, updated_at, last_login_at
            ) values ($1, $2, $3, $4, $5, false, null, null, now(), now(), now())",
            &[&user.user_id, &user.username, &user.display_name, &user.salt, &user.password_hash],
        )
        .await?;

        tx.execute(
            "insert into identity.user_session (
                session_token, user_id, username, created_at, expires_at
            ) values ($1, $2, $3, now(), $4)",
            &[&session.session_token, &session.user_id, &session.username, &session.expires_at],
        )
        .await?;

        let row = tx
            .query_one(
                "select user_id, username, display_name, salt, password_hash,
                        subscribed, subscription_plan, subscription_expires_at,
                        created_at, updated_at, last_login_at
                   from identity.user_account
                  where user_id = $1",
                &[&user.user_id],
            )
            .await?;

        tx.commit().await?;
        Ok(map_user(&row))
    }

    async fn find_user_by_username(&self, username: &str) -> Result<Option<UserAccount>> {
        let client = self.pool.acquire().await?;
        let row = client
            .client()
            .query_opt(
                "select user_id, username, display_name, salt, password_hash,
                        subscribed, subscription_plan, subscription_expires_at,
                        created_at, updated_at, last_login_at
                   from identity.user_account
                  where username = $1",
                &[&username],
            )
            .await?;
        Ok(row.map(|row| map_user(&row)))
    }

    async fn create_session(&self, session: NewSession) -> Result<()> {
        let mut client = self.pool.acquire().await?;
        let tx = client.client_mut().transaction().await?;
        tx.execute(
            "delete from identity.user_session where expires_at <= now()",
            &[],
        )
        .await?;
        tx.execute(
            "insert into identity.user_session (
                session_token, user_id, username, created_at, expires_at
            ) values ($1, $2, $3, now(), $4)",
            &[&session.session_token, &session.user_id, &session.username, &session.expires_at],
        )
        .await?;
        tx.execute(
            "update identity.user_account set last_login_at = now(), updated_at = now() where user_id = $1",
            &[&session.user_id],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn find_user_by_session_token(&self, token: &str) -> Result<Option<UserAccount>> {
        let client = self.pool.acquire().await?;
        client
            .client()
            .execute(
                "delete from identity.user_session where expires_at <= now()",
                &[],
            )
            .await?;
        let row = client
            .client()
            .query_opt(
                "select u.user_id, u.username, u.display_name, u.salt, u.password_hash,
                        u.subscribed, u.subscription_plan, u.subscription_expires_at,
                        u.created_at, u.updated_at, u.last_login_at
                   from identity.user_session s
                   join identity.user_account u on u.user_id = s.user_id
                  where s.session_token = $1 and s.expires_at > now()",
                &[&token],
            )
            .await?;
        Ok(row.map(|row| map_user(&row)))
    }

    async fn delete_session(&self, token: &str) -> Result<()> {
        let client = self.pool.acquire().await?;
        client
            .client()
            .execute(
                "delete from identity.user_session where session_token = $1",
                &[&token],
            )
            .await?;
        Ok(())
    }

    async fn list_active_plans(&self) -> Result<Vec<SubscriptionPlan>> {
        let client = self.pool.acquire().await?;
        let rows = client
            .client()
            .query(
                "select plan_code, plan_label, duration_days, price_text, description, active
                   from identity.subscription_plan
                  where active = true
                  order by sort_order asc, plan_code asc",
                &[],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| SubscriptionPlan {
                code: row.get("plan_code"),
                label: row.get("plan_label"),
                days: i64::from(row.get::<_, i32>("duration_days")),
                price_text: row.get("price_text"),
                description: row.get("description"),
                active: row.get("active"),
            })
            .collect())
    }

    async fn activate_subscription(
        &self,
        session_token: &str,
        plan_code: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<UserAccount> {
        let mut client = self.pool.acquire().await?;
        let tx = client.client_mut().transaction().await?;

        let session_row = tx
            .query_opt(
                "select user_id, username from identity.user_session where session_token = $1 and expires_at > now()",
                &[&session_token],
            )
            .await?
            .ok_or_else(|| anyhow!("请先登录"))?;
        let user_id: Uuid = session_row.get("user_id");
        let username: String = session_row.get("username");

        let plan_row = tx
            .query_opt(
                "select plan_code from identity.subscription_plan where plan_code = $1 and active = true",
                &[&plan_code],
            )
            .await?;
        if plan_row.is_none() {
            return Err(anyhow!("无效的订阅套餐"));
        }

        tx.execute(
            "update identity.user_subscription
                set status = 'SUPERSEDED', updated_at = now()
              where user_id = $1 and status = 'ACTIVE'",
            &[&user_id],
        )
        .await?;

        let subscription_id = Uuid::new_v4();
        tx.execute(
            "insert into identity.user_subscription (
                subscription_id, user_id, username, plan_code,
                started_at, expires_at, status, created_at, updated_at
            ) values ($1, $2, $3, $4, now(), $5, 'ACTIVE', now(), now())",
            &[&subscription_id, &user_id, &username, &plan_code, &expires_at],
        )
        .await?;

        tx.execute(
            "update identity.user_account
                set subscribed = true,
                    subscription_plan = $1,
                    subscription_expires_at = $2,
                    updated_at = now()
              where user_id = $3",
            &[&plan_code, &expires_at, &user_id],
        )
        .await?;

        let row = tx
            .query_one(
                "select user_id, username, display_name, salt, password_hash,
                        subscribed, subscription_plan, subscription_expires_at,
                        created_at, updated_at, last_login_at
                   from identity.user_account
                  where user_id = $1",
                &[&user_id],
            )
            .await?;
        tx.commit().await?;
        Ok(map_user(&row))
    }

    async fn list_favorite_symbols(&self, session_token: &str) -> Result<Vec<String>> {
        let user_id = self.resolve_session_user(session_token).await?;
        self.list_favorite_symbols_by_user_id(user_id).await
    }

    async fn add_favorite_symbol(&self, session_token: &str, symbol: &str) -> Result<Vec<String>> {
        let mut client = self.pool.acquire().await?;
        let tx = client.client_mut().transaction().await?;
        let row = tx
            .query_opt(
                "select user_id
                   from identity.user_session
                  where session_token = $1
                    and expires_at > now()",
                &[&session_token],
            )
            .await?
            .ok_or_else(|| anyhow!("请先登录"))?;
        let user_id: Uuid = row.get("user_id");
        tx.execute(
            "insert into identity.user_favorite_symbol (user_id, symbol, created_at)
             values ($1, $2, now())
             on conflict (user_id, symbol) do nothing",
            &[&user_id, &symbol],
        )
        .await?;
        let rows = tx
            .query(
                "select symbol
                   from identity.user_favorite_symbol
                  where user_id = $1
                  order by created_at desc, symbol asc",
                &[&user_id],
            )
            .await?;
        tx.commit().await?;
        Ok(rows.into_iter().map(|row| row.get("symbol")).collect())
    }

    async fn remove_favorite_symbol(
        &self,
        session_token: &str,
        symbol: &str,
    ) -> Result<Vec<String>> {
        let mut client = self.pool.acquire().await?;
        let tx = client.client_mut().transaction().await?;
        let row = tx
            .query_opt(
                "select user_id
                   from identity.user_session
                  where session_token = $1
                    and expires_at > now()",
                &[&session_token],
            )
            .await?
            .ok_or_else(|| anyhow!("请先登录"))?;
        let user_id: Uuid = row.get("user_id");
        tx.execute(
            "delete from identity.user_favorite_symbol
              where user_id = $1 and symbol = $2",
            &[&user_id, &symbol],
        )
        .await?;
        let rows = tx
            .query(
                "select symbol
                   from identity.user_favorite_symbol
                  where user_id = $1
                  order by created_at desc, symbol asc",
                &[&user_id],
            )
            .await?;
        tx.commit().await?;
        Ok(rows.into_iter().map(|row| row.get("symbol")).collect())
    }
}

fn map_user(row: &Row) -> UserAccount {
    UserAccount {
        user_id: row.get("user_id"),
        username: row.get("username"),
        display_name: row.get("display_name"),
        salt: row.get("salt"),
        password_hash: row.get("password_hash"),
        subscribed: row.get("subscribed"),
        subscription_plan: row.get("subscription_plan"),
        subscription_expires_at: row.get("subscription_expires_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        last_login_at: row.get("last_login_at"),
    }
}
