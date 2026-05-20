//! Countdown timer state with a background tick task and APNS push updates.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use crate::apns::ApnsManager;
use crate::game_manager::GameManager;
use crate::sound::{SoundFx, SoundName};

const DEFAULT_SECONDS: i64 = 5 * 60;

struct TimerInner {
    is_running: bool,
    seconds: i64,
}

pub struct TimerState {
    inner: Mutex<TimerInner>,
    apns: ApnsManager,
    sound: SoundFx,
    manager: Arc<GameManager>,
    ticking: AtomicBool,
}

impl TimerState {
    pub fn new(manager: Arc<GameManager>) -> Self {
        TimerState {
            inner: Mutex::new(TimerInner {
                is_running: false,
                seconds: DEFAULT_SECONDS,
            }),
            apns: ApnsManager::new(),
            sound: SoundFx::new(),
            manager,
            ticking: AtomicBool::new(false),
        }
    }

    pub fn apns(&self) -> &ApnsManager {
        &self.apns
    }

    /// Spawn the once-per-second tick task. Idempotent.
    pub fn spawn_ticker(self: &Arc<Self>) {
        if self.ticking.swap(true, Ordering::SeqCst) {
            return;
        }
        let this = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                let changed = {
                    let mut inner = this.inner.lock().await;
                    if inner.is_running {
                        if inner.seconds > 0 {
                            inner.seconds -= 1;
                        } else {
                            inner.is_running = false;
                            inner.seconds = 0;
                            let _ = this.sound.play(SoundName::Timer);
                        }
                        true
                    } else {
                        false
                    }
                };
                // Push the per-second update to SSE subscribers (only while running).
                if changed {
                    this.manager.notify_current().await;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    pub async fn get_seconds(&self) -> i64 {
        self.inner.lock().await.seconds
    }

    pub async fn get_is_running(&self) -> bool {
        self.inner.lock().await.is_running
    }

    async fn player_counts(&self) -> (i64, i64) {
        match self.manager.get_state().await {
            Ok(state) => (
                state.living_player_count() as i64,
                state.players.len() as i64,
            ),
            Err(_) => (0, 0),
        }
    }

    pub async fn set_is_running(&self, new_value: bool) {
        let seconds = {
            let mut inner = self.inner.lock().await;
            inner.is_running = new_value;
            inner.seconds
        };
        let (alive, total) = self.player_counts().await;
        self.apns
            .send_timer_update(seconds, new_value, alive, total)
            .await;
        self.manager.notify_current().await;
    }

    pub async fn set_seconds(&self, new_value: i64) {
        let (seconds, is_running) = {
            let mut inner = self.inner.lock().await;
            inner.seconds = new_value.max(0);
            (inner.seconds, inner.is_running)
        };
        let (alive, total) = self.player_counts().await;
        self.apns
            .send_timer_update(seconds, is_running, alive, total)
            .await;
        self.manager.notify_current().await;
    }

    pub async fn add_seconds(&self, additional: i64) {
        let current = self.get_seconds().await;
        self.set_seconds(current + additional).await;
    }

    /// Push a Live Activity update with current timer state and given counts.
    pub async fn push_live_activity_update(&self, players_alive: i64, total_players: i64) {
        let seconds = self.get_seconds().await;
        let is_running = self.get_is_running().await;
        self.apns
            .send_timer_update(seconds, is_running, players_alive, total_players)
            .await;
    }
}
