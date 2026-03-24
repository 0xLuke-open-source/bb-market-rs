use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::web::state::{AccessInfoJson, FullSnapshot, SharedDashboardState, TraderStateJson};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedDashboardCache {
    generated_at_ms: u64,
    snapshot: FullSnapshot,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub async fn load_dashboard_cache(
    dash: &SharedDashboardState,
    path: &str,
    max_age: Duration,
) -> std::io::Result<bool> {
    let raw = match fs::read_to_string(path).await {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err),
    };

    let cache: PersistedDashboardCache = match serde_json::from_str(&raw) {
        Ok(cache) => cache,
        Err(_) => return Ok(false),
    };

    let age_ms = now_ms().saturating_sub(cache.generated_at_ms);
    if age_ms > max_age.as_millis() as u64 {
        return Ok(false);
    }

    let mut state = dash.write().await;
    state.replace_from_snapshot(cache.snapshot);
    Ok(true)
}

pub async fn persist_dashboard_cache(
    dash: &SharedDashboardState,
    path: &str,
) -> std::io::Result<()> {
    let snapshot = {
        let state = dash.read().await;
        state.to_cache_snapshot()
    };

    let cache = PersistedDashboardCache {
        generated_at_ms: now_ms(),
        snapshot: FullSnapshot {
            trader: TraderStateJson::default(),
            access: AccessInfoJson::default(),
            ..snapshot
        },
    };

    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent).await?;
    }

    let encoded = serde_json::to_vec(&cache)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    fs::write(path, encoded).await
}
