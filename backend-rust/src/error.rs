//! Domain error types.
//!
//! Replaces the stringly-typed `Result<T, String>` used during the initial port
//! with proper enums, so callers can match on failure kinds and the HTTP layer
//! can map them to status codes via `From` impls.

use uuid::Uuid;

/// Errors from the SQLite event store.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("event (de)serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("invalid stored UUID: {0}")]
    Uuid(#[from] uuid::Error),
}

/// Errors from game management (dispatch, load, rewind, fork).
#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("No active game")]
    NoActiveGame,
    #[error("{0}")]
    InvalidVersion(String),
    #[error("No events found for game {0}")]
    GameNotFound(Uuid),
    #[error("Cannot replay empty event list")]
    EmptyReplay,
    #[error(transparent)]
    Storage(#[from] StoreError),
}
