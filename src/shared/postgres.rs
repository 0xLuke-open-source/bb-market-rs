use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use tokio::sync::Notify;
use tokio_postgres::{Client, NoTls};

use crate::shared::config::DatabaseConfig;

pub struct PgPool {
    config: DatabaseConfig,
    idle: Mutex<Vec<IdleClient>>,
    total: AtomicUsize,
    notify: Notify,
    min_connections: usize,
    max_connections: usize,
    idle_timeout: Duration,
}

struct IdleClient {
    client: Client,
    idle_since: Instant,
}

pub struct PooledClient {
    client: Option<Client>,
    pool: Arc<PgPool>,
}

impl PgPool {
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        let min_connections = config.min_connections as usize;
        let max_connections = config.max_connections as usize;
        if max_connections == 0 {
            return Err(anyhow!("database.max_connections 必须大于 0"));
        }
        if min_connections > max_connections {
            return Err(anyhow!(
                "database.min_connections 不能大于 database.max_connections"
            ));
        }

        let pool = Self {
            idle: Mutex::new(Vec::with_capacity(max_connections)),
            total: AtomicUsize::new(0),
            notify: Notify::new(),
            idle_timeout: Duration::from_secs(config.idle_timeout_seconds),
            min_connections,
            max_connections,
            config,
        };

        for _ in 0..min_connections {
            let client = pool.create_client().await?;
            pool.push_idle(client);
            pool.total.fetch_add(1, Ordering::SeqCst);
        }

        Ok(pool)
    }

    pub async fn acquire(self: &Arc<Self>) -> Result<PooledClient> {
        loop {
            if let Some(client) = self.try_take_idle() {
                return Ok(PooledClient {
                    client: Some(client),
                    pool: self.clone(),
                });
            }

            if self.try_reserve_new_slot() {
                match self.create_client().await {
                    Ok(client) => {
                        return Ok(PooledClient {
                            client: Some(client),
                            pool: self.clone(),
                        });
                    }
                    Err(err) => {
                        self.total.fetch_sub(1, Ordering::SeqCst);
                        self.notify.notify_one();
                        return Err(err);
                    }
                }
            }

            self.notify.notified().await;
        }
    }

    fn try_take_idle(&self) -> Option<Client> {
        let mut idle = self.idle.lock().expect("postgres pool mutex poisoned");
        while let Some(entry) = idle.pop() {
            if self.should_reap(&entry) {
                self.total.fetch_sub(1, Ordering::SeqCst);
                continue;
            }
            return Some(entry.client);
        }
        None
    }

    fn push_idle(&self, client: Client) {
        let mut idle = self.idle.lock().expect("postgres pool mutex poisoned");
        idle.push(IdleClient {
            client,
            idle_since: Instant::now(),
        });
        drop(idle);
        self.notify.notify_one();
    }

    fn should_reap(&self, entry: &IdleClient) -> bool {
        self.total.load(Ordering::SeqCst) > self.min_connections
            && entry.idle_since.elapsed() >= self.idle_timeout
    }

    fn try_reserve_new_slot(&self) -> bool {
        loop {
            let current = self.total.load(Ordering::SeqCst);
            if current >= self.max_connections {
                return false;
            }
            if self
                .total
                .compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return true;
            }
        }
    }

    async fn create_client(&self) -> Result<Client> {
        let (client, connection) =
            tokio_postgres::connect(&self.config.postgres_dsn(), NoTls).await?;
        tokio::spawn(async move {
            if let Err(err) = connection.await {
                eprintln!("postgres connection error: {}", err);
            }
        });
        Ok(client)
    }
}

impl PooledClient {
    pub fn client(&self) -> &Client {
        self.client.as_ref().expect("pooled client already taken")
    }

    pub fn client_mut(&mut self) -> &mut Client {
        self.client.as_mut().expect("pooled client already taken")
    }
}

impl Drop for PooledClient {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            self.pool.push_idle(client);
        }
    }
}
