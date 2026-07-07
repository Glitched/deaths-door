//! Ephemeral live-vote state: the running tally shown on the overlay while
//! hands go around the circle.
//!
//! Like the timer, this is broadcast state, not game history — only the
//! *confirmed* nomination becomes an event. Every change re-broadcasts the
//! game state so SSE subscribers (the projector overlay) see the tally climb.

use std::sync::{Arc, Mutex};

use serde::Serialize;
use utoipa::ToSchema;

use crate::game_manager::GameManager;
use crate::lock::LockExt;

/// A vote currently being tallied, surfaced in `/game/state` and SSE frames.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct VoteInProgress {
    /// The nominated player.
    pub player_name: String,
    /// Voters selected so far; the running tally is this list's length.
    pub voters: Vec<String>,
}

pub struct VoteSession {
    inner: Mutex<Option<VoteInProgress>>,
    manager: Arc<GameManager>,
}

impl VoteSession {
    pub fn new(manager: Arc<GameManager>) -> Self {
        VoteSession {
            inner: Mutex::new(None),
            manager,
        }
    }

    /// The vote being tallied right now, if any.
    pub fn current(&self) -> Option<VoteInProgress> {
        self.inner.lock_recover().clone()
    }

    /// Start (or restart) a tally for a nominee and broadcast the change.
    pub async fn start(&self, player_name: String) {
        *self.inner.lock_recover() = Some(VoteInProgress {
            player_name,
            voters: Vec::new(),
        });
        self.manager.notify_current().await;
    }

    /// Replace the selected voters and broadcast; `None` if no vote is open.
    pub async fn set_voters(&self, voters: Vec<String>) -> Option<VoteInProgress> {
        let updated = {
            let mut inner = self.inner.lock_recover();
            let session = inner.as_mut()?;
            session.voters = voters;
            session.clone()
        };
        self.manager.notify_current().await;
        Some(updated)
    }

    /// Clear the tally and broadcast (used by the explicit cancel endpoint).
    pub async fn cancel(&self) -> Option<VoteInProgress> {
        let taken = self.inner.lock_recover().take();
        if taken.is_some() {
            self.manager.notify_current().await;
        }
        taken
    }

    /// Clear without broadcasting — for callers about to dispatch an event
    /// (the dispatch broadcast will already carry the cleared tally).
    pub fn clear_silent(&self) -> Option<VoteInProgress> {
        self.inner.lock_recover().take()
    }

    /// Put a tally back (e.g. a confirm that failed validation), silently.
    pub fn restore(&self, session: VoteInProgress) {
        *self.inner.lock_recover() = Some(session);
    }
}
