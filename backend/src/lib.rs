//! Death's Door — Blood on the Clocktower game management system (Rust port).
//!
//! Mirrors the Python/FastAPI backend: event-sourced game state persisted to
//! SQLite, rebuilt via a pure `apply(state, event)` function, served over an
//! axum HTTP API with a Server-Sent Events stream for real-time updates.

pub mod alignment;
pub mod apns;
pub mod apply;
pub mod character;
pub mod character_type;
pub mod config;
pub mod effects;
pub mod error;
pub mod event_store;
pub mod events;
pub mod game_manager;
pub mod game_state;
pub mod lighting;
pub mod lock;
pub mod night_step;
pub mod player;
pub mod script;
pub mod script_name;
pub mod scripts;
pub mod sound;
pub mod status_effect;
pub mod timer_state;

pub mod app;
pub mod routes;
