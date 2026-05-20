//! APNS manager for sending Live Activity push updates.
//!
//! Gracefully degrades when an APNS key is not configured — the iOS app falls
//! back to its local countdown timer in that case (the normal path here).
//!
//! Pushes are **coalesced and rate-limited**: rapid updates (e.g. a `/timer/set`
//! that sets seconds then starts, or adding a table of players) collapse to the
//! latest state and are sent at most once per [`MIN_PUSH_INTERVAL`], so we don't
//! trip Apple's Live Activity rate limit (HTTP 429). The iOS app counts down
//! locally from `endTime` between pushes, so infrequent pushes are sufficient.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;

use crate::config;
use crate::lock::LockExt;

/// Offset between the Unix epoch and Swift's reference date (Jan 1, 2001).
const SWIFT_EPOCH_OFFSET: f64 = 978_307_200.0;

/// Minimum spacing between Live Activity pushes. Updates that arrive sooner
/// coalesce to the most recent state.
const MIN_PUSH_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Serialize)]
struct JwtClaims {
    iss: String,
    iat: u64,
}

#[derive(Clone, Copy)]
struct TimerUpdate {
    seconds: i64,
    is_running: bool,
    players_alive: i64,
    total_players: i64,
}

/// Coalescing rate-limiter state.
#[derive(Default)]
struct Throttle {
    last_sent: Option<Instant>,
    pending: Option<TimerUpdate>,
    flush_scheduled: bool,
}

struct ApnsInner {
    push_tokens: Mutex<HashSet<String>>,
    key: Option<String>,
    key_id: Option<String>,
    team_id: String,
    bundle_id: String,
    available: bool,
    /// Reused HTTP/2 client (built once, with a request timeout).
    client: reqwest::Client,
    throttle: Mutex<Throttle>,
}

pub struct ApnsManager {
    inner: Arc<ApnsInner>,
}

impl ApnsManager {
    pub fn new() -> Self {
        let (key_id, key) = load_key();
        let available = key.is_some();
        if !available {
            tracing::warn!(
                "APNS not configured, Live Activity push updates disabled. \
                 Missing: APNS key file (backend/keys/AuthKey_*.p8)"
            );
        } else {
            tracing::info!(
                "APNS configured: key={:?}, bundle={}",
                key_id,
                config::APNS_BUNDLE_ID
            );
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        ApnsManager {
            inner: Arc::new(ApnsInner {
                push_tokens: Mutex::new(HashSet::new()),
                key,
                key_id,
                team_id: config::APNS_TEAM_ID.to_string(),
                bundle_id: config::APNS_BUNDLE_ID.to_string(),
                available,
                client,
                throttle: Mutex::new(Throttle::default()),
            }),
        }
    }

    pub fn is_available(&self) -> bool {
        self.inner.available
    }

    pub fn register_token(&self, token: String) {
        let preview: String = token.chars().take(8).collect();
        self.inner.push_tokens.lock_recover().insert(token);
        tracing::info!("Registered APNS push token: {preview}...");
    }

    /// Queue a Live Activity update. Coalesces rapid calls and sends at most the
    /// latest state once per [`MIN_PUSH_INTERVAL`]. Fire-and-forget; no-ops when
    /// APNS isn't configured.
    pub async fn send_timer_update(
        &self,
        seconds: i64,
        is_running: bool,
        players_alive: i64,
        total_players: i64,
    ) {
        if !self.inner.available {
            return;
        }
        let update = TimerUpdate {
            seconds,
            is_running,
            players_alive,
            total_players,
        };

        // Record the latest desired state; schedule one flush if none pending.
        let delay = {
            let mut t = self.inner.throttle.lock_recover();
            t.pending = Some(update);
            if t.flush_scheduled {
                return; // an in-flight flush will pick up the latest pending state
            }
            t.flush_scheduled = true;
            match t.last_sent {
                Some(last) => MIN_PUSH_INTERVAL.saturating_sub(last.elapsed()),
                None => Duration::ZERO,
            }
        };

        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            let update = {
                let mut t = inner.throttle.lock_recover();
                t.flush_scheduled = false;
                t.last_sent = Some(Instant::now());
                t.pending.take()
            };
            if let Some(update) = update {
                inner.do_send(update).await;
            }
        });
    }
}

impl ApnsInner {
    fn make_jwt(&self) -> Result<String, String> {
        let key = self.key.as_ref().ok_or("no APNS key")?;
        let key_id = self.key_id.as_ref().ok_or("no APNS key id")?;
        let claims = JwtClaims {
            iss: self.team_id.clone(),
            iat: unix_now() as u64,
        };
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(key_id.clone());
        let encoding_key = EncodingKey::from_ec_pem(key.as_bytes()).map_err(|e| e.to_string())?;
        encode(&header, &claims, &encoding_key).map_err(|e| e.to_string())
    }

    /// Actually send the update to every registered token.
    async fn do_send(&self, update: TimerUpdate) {
        let tokens: Vec<String> = {
            let guard = self.push_tokens.lock_recover();
            if guard.is_empty() {
                tracing::info!("APNS: no push tokens registered, skipping");
                return;
            }
            guard.iter().cloned().collect()
        };

        tracing::info!(
            "APNS: pushing update (alive={}/{}, timer={}s, run={})",
            update.players_alive,
            update.total_players,
            update.seconds,
            update.is_running
        );

        let end_time = (unix_now_f64() + update.seconds as f64) - SWIFT_EPOCH_OFFSET;
        let apns_payload = json!({
            "aps": {
                "timestamp": unix_now(),
                "event": "update",
                "content-state": {
                    "running": update.is_running,
                    "endTime": end_time,
                    "playersAlive": update.players_alive,
                    "totalPlayers": update.total_players,
                },
            },
        });

        let token_jwt = match self.make_jwt() {
            Ok(jwt) => jwt,
            Err(e) => {
                tracing::warn!("APNS: failed to sign JWT: {e}");
                return;
            }
        };
        let topic = format!("{}.push-type.liveactivity", self.bundle_id);

        let mut stale_tokens: Vec<String> = Vec::new();
        for token in &tokens {
            let url = format!("{}/3/device/{}", config::APNS_HOST, token);
            let result = self
                .client
                .post(&url)
                .json(&apns_payload)
                .header("authorization", format!("bearer {token_jwt}"))
                .header("apns-topic", &topic)
                .header("apns-push-type", "liveactivity")
                .header("apns-priority", "10")
                .send()
                .await;

            match result {
                Ok(resp) if resp.status().as_u16() == 200 => {
                    let preview: String = token.chars().take(8).collect();
                    tracing::info!("APNS push sent to {preview}...");
                }
                Ok(resp) if resp.status().as_u16() == 410 => {
                    stale_tokens.push(token.clone());
                }
                Ok(resp) => {
                    tracing::warn!("APNS push failed ({})", resp.status());
                }
                Err(e) => tracing::warn!("APNS request failed: {e}"),
            }
        }

        if !stale_tokens.is_empty() {
            let mut guard = self.push_tokens.lock_recover();
            for t in stale_tokens {
                guard.remove(&t);
            }
        }
    }
}

impl Default for ApnsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Find and load the `.p8` key file from the candidate key directories.
fn load_key() -> (Option<String>, Option<String>) {
    let key_file = config::apns_key_globs().into_iter().find_map(|pattern| {
        glob::glob(&pattern.to_string_lossy())
            .ok()
            .and_then(|mut paths| paths.find_map(Result::ok))
    });
    let Some(key_file) = key_file else {
        return (None, None);
    };

    let filename = key_file
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let key_id = filename.replace("AuthKey_", "").replace(".p8", "");

    match std::fs::read_to_string(&key_file) {
        Ok(content) => (Some(key_id), Some(content)),
        Err(e) => {
            tracing::warn!("Failed to read APNS key file {:?}: {e}", key_file);
            (None, None)
        }
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn unix_now_f64() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
