use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

const DEFAULT_CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database_name: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub db: i64,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let path = resolve_config_path();
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        parse_config(&raw).with_context(|| format!("failed to parse config file: {}", path.display()))
    }
}

impl DatabaseConfig {
    pub fn postgres_dsn(&self) -> String {
        let auth = if self.password.is_empty() {
            self.username.clone()
        } else {
            format!("{}:{}", self.username, self.password)
        };
        format!(
            "postgres://{}@{}:{}/{}?connect_timeout={}",
            auth,
            self.host,
            self.port,
            self.database_name,
            self.connect_timeout_seconds
        )
    }
}

fn resolve_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("BB_MARKET_CONFIG") {
        return Path::new(&path).to_path_buf();
    }
    PathBuf::from(DEFAULT_CONFIG_PATH)
}

fn parse_config(raw: &str) -> Result<AppConfig> {
    let mut section = String::new();
    let mut values: HashMap<String, HashMap<String, String>> = HashMap::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if section.is_empty() {
            return Err(anyhow!("config key outside of section: {}", key.trim()));
        }
        values
            .entry(section.clone())
            .or_default()
            .insert(key.trim().to_string(), normalize_value(value));
    }

    Ok(AppConfig {
        database: DatabaseConfig {
            host: required_string(&values, "database", "host")?,
            port: required_parse(&values, "database", "port")?,
            username: required_string(&values, "database", "username")?,
            password: optional_string(&values, "database", "password").unwrap_or_default(),
            database_name: required_string(&values, "database", "database_name")?,
            max_connections: required_parse(&values, "database", "max_connections")?,
            min_connections: required_parse(&values, "database", "min_connections")?,
            connect_timeout_seconds: required_parse(&values, "database", "connect_timeout_seconds")?,
            idle_timeout_seconds: required_parse(&values, "database", "idle_timeout_seconds")?,
        },
        redis: RedisConfig {
            host: required_string(&values, "redis", "host")?,
            port: required_parse(&values, "redis", "port")?,
            username: optional_string(&values, "redis", "username").unwrap_or_default(),
            password: optional_string(&values, "redis", "password").unwrap_or_default(),
            db: required_parse(&values, "redis", "db")?,
        },
    })
}

fn normalize_value(value: &str) -> String {
    let raw = value.trim();
    if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
        raw[1..raw.len() - 1].to_string()
    } else {
        raw.to_string()
    }
}

fn required_string(
    values: &HashMap<String, HashMap<String, String>>,
    section: &str,
    key: &str,
) -> Result<String> {
    optional_string(values, section, key)
        .ok_or_else(|| anyhow!("missing config: [{}].{}", section, key))
}

fn optional_string(
    values: &HashMap<String, HashMap<String, String>>,
    section: &str,
    key: &str,
) -> Option<String> {
    values.get(section).and_then(|section_map| section_map.get(key)).cloned()
}

fn required_parse<T>(
    values: &HashMap<String, HashMap<String, String>>,
    section: &str,
    key: &str,
) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let raw = required_string(values, section, key)?;
    raw.parse::<T>()
        .map_err(|err| anyhow!("invalid config [{}].{}: {}", section, key, err))
}
