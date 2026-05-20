//! Game manager with event-sourcing dispatch pattern.
//!
//! All mutations go through [`GameManager::dispatch`], which atomically:
//! 1. Creates an event from the payload,
//! 2. Applies it to produce a new state (validating),
//! 3. Persists the event to SQLite,
//! 4. Updates the in-memory cache and notifies SSE subscribers.

use chrono::Utc;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

use crate::apply::{apply, replay};
use crate::error::GameError;
use crate::event_store::EventStore;
use crate::events::{EventPayload, GameEvent};
use crate::game_state::GameState;

struct Inner {
    store: EventStore,
    state: Option<GameState>,
}

pub struct GameManager {
    inner: Mutex<Inner>,
    tx: broadcast::Sender<GameState>,
    /// Serializes lazy initialization in `get_state` so concurrent first
    /// requests (overlay SSE, iOS, timer) don't each create/load a game.
    init_lock: Mutex<()>,
}

impl GameManager {
    pub fn new(store: EventStore) -> Self {
        let (tx, _rx) = broadcast::channel(128);
        GameManager {
            inner: Mutex::new(Inner { store, state: None }),
            tx,
            init_lock: Mutex::new(()),
        }
    }

    /// Subscribe to state changes (used by the SSE stream).
    pub fn subscribe(&self) -> broadcast::Receiver<GameState> {
        self.tx.subscribe()
    }

    /// Re-broadcast the current state to SSE subscribers without a mutation.
    /// Used by the timer to push per-second updates over the same stream.
    pub async fn notify_current(&self) {
        if let Some(state) = self.inner.lock().await.state.clone() {
            let _ = self.tx.send(state);
        }
    }

    /// Get the current game state, or an error if no game is active.
    pub async fn state(&self) -> Result<GameState, GameError> {
        self.inner
            .lock()
            .await
            .state
            .clone()
            .ok_or(GameError::NoActiveGame)
    }

    pub async fn has_active_game(&self) -> bool {
        self.inner.lock().await.state.is_some()
    }

    /// Atomically apply and persist an event. Returns the new state.
    pub async fn dispatch(&self, payload: EventPayload) -> Result<GameState, GameError> {
        let mut inner = self.inner.lock().await;
        let current = inner.state.clone().ok_or(GameError::NoActiveGame)?;
        let event = GameEvent::new(current.game_id, current.version, payload);
        // Apply first to validate, then persist only after a successful apply.
        let new_state = apply(&current, &event);
        inner.store.append(&event)?;
        inner.state = Some(new_state.clone());
        // Non-blocking broadcast; ignored if there are no subscribers.
        let _ = self.tx.send(new_state.clone());
        Ok(new_state)
    }

    /// Get the current game state, auto-creating/loading if none exists.
    pub async fn get_state(&self) -> Result<GameState, GameError> {
        if let Some(state) = self.inner.lock().await.state.clone() {
            return Ok(state);
        }

        // Hold the init lock across the create/load decision; re-check once we
        // have it, since another task may have initialized while we waited.
        let _init = self.init_lock.lock().await;
        if let Some(state) = self.inner.lock().await.state.clone() {
            return Ok(state);
        }

        if sample_game_enabled() {
            self.create_sample_game().await?;
        } else {
            let most_recent = self.inner.lock().await.store.get_most_recent_game_id()?;
            if let Some(game_id) = most_recent {
                self.load_game(game_id).await?;
            } else {
                self.create_game("trouble_brewing").await?;
            }
        }
        self.state().await
    }

    pub async fn create_game(&self, script_name: &str) -> Result<GameState, GameError> {
        let mut inner = self.inner.lock().await;
        let game_id = Uuid::new_v4();
        let event = GameEvent {
            id: Uuid::new_v4(),
            game_id,
            sequence: 0,
            timestamp: Utc::now(),
            payload: EventPayload::GameCreated {
                script_name: script_name.to_string(),
            },
        };
        let initial = GameState::initial(game_id, "");
        let new_state = apply(&initial, &event);
        inner.store.append(&event)?;
        inner.state = Some(new_state.clone());
        Ok(new_state)
    }

    pub async fn load_game(&self, game_id: Uuid) -> Result<GameState, GameError> {
        let mut inner = self.inner.lock().await;
        let events = inner.store.get_events(game_id, None)?;
        if events.is_empty() {
            return Err(GameError::GameNotFound(game_id));
        }
        let state = replay(&events)?;
        inner.state = Some(state.clone());
        Ok(state)
    }

    pub async fn rewind(&self, to_version: i64) -> Result<GameState, GameError> {
        let mut inner = self.inner.lock().await;
        let current = inner.state.clone().ok_or(GameError::NoActiveGame)?;
        if to_version < 1 || to_version > current.version {
            return Err(GameError::InvalidVersion(format!(
                "Invalid version {to_version} (current: {})",
                current.version
            )));
        }
        inner
            .store
            .delete_after_sequence(current.game_id, to_version - 1)?;
        let events = inner.store.get_events(current.game_id, None)?;
        let state = replay(&events)?;
        inner.state = Some(state.clone());
        Ok(state)
    }

    pub async fn fork(&self, from_version: i64) -> Result<GameState, GameError> {
        let mut inner = self.inner.lock().await;
        let current = inner.state.clone().ok_or(GameError::NoActiveGame)?;
        if from_version < 1 || from_version > current.version {
            return Err(GameError::InvalidVersion(format!(
                "Invalid version {from_version} (current: {})",
                current.version
            )));
        }
        let new_game_id = inner.store.fork_game(current.game_id, from_version - 1)?;
        let events = inner.store.get_events(new_game_id, None)?;
        let state = replay(&events)?;
        inner.state = Some(state.clone());
        Ok(state)
    }

    pub async fn get_history(&self) -> Result<Vec<GameEvent>, GameError> {
        let inner = self.inner.lock().await;
        let current = inner.state.clone().ok_or(GameError::NoActiveGame)?;
        Ok(inner.store.get_events(current.game_id, None)?)
    }

    pub async fn list_games(&self) -> Result<Vec<Uuid>, GameError> {
        Ok(self.inner.lock().await.store.get_all_game_ids()?)
    }

    async fn create_sample_game(&self) -> Result<(), GameError> {
        self.create_game("trouble_brewing").await?;

        let roles = [
            "Imp",
            "Baron",
            "Poisoner",
            "Scarlet Woman", // evil
            "Recluse",
            "Librarian",
            "Empath",
            "Investigator", // good
            "Mayor",
            "Fortune Teller",
            "Slayer",
            "Monk",
            "Virgin", // good
        ];
        self.dispatch(EventPayload::RolesIncluded {
            names: roles.iter().map(|s| s.to_string()).collect(),
        })
        .await?;

        let players = [
            ("Ryan", "Baron", "evil"),
            ("Yash", "Virgin", "good"),
            ("Other Ryan", "Imp", "evil"),
            ("Other Yash", "Poisoner", "evil"),
            ("Yet Another Ryan", "Scarlet Woman", "evil"),
            ("Yet Another Yash", "Recluse", "good"),
            ("Even More Ryan", "Librarian", "good"),
            ("Even More Yash", "Empath", "good"),
            ("Even Even More Ryan", "Mayor", "good"),
            ("Even Even More Yash", "Fortune Teller", "good"),
            ("Claude", "Monk", "good"),
        ];
        for (name, role, alignment) in players {
            self.dispatch(EventPayload::PlayerAdded {
                player_name: name.to_string(),
                character_name: role.to_string(),
                alignment: alignment.to_string(),
            })
            .await?;
        }

        for (player, effect) in [
            ("Yash", "Drunk"),
            ("Yash", "No Ability"),
            ("Ryan", "Is The Demon"),
        ] {
            self.dispatch(EventPayload::StatusEffectAdded {
                player_name: player.to_string(),
                effect: effect.to_string(),
            })
            .await?;
        }

        self.dispatch(EventPayload::PlayerAliveSet {
            player_name: "Yash".to_string(),
            is_alive: false,
            cleared_effects: Vec::new(),
        })
        .await?;
        self.dispatch(EventPayload::DeadVoteUsedSet {
            player_name: "Yash".to_string(),
            has_used_dead_vote: true,
        })
        .await?;

        Ok(())
    }
}

fn sample_game_enabled() -> bool {
    std::env::var("SAMPLE_GAME")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true"))
        .unwrap_or(false)
}
