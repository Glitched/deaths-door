//! Application wiring: shared state, error type, and the axum router.

use std::sync::Arc;

use axum::extract::{FromRequest, Request};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::error::{GameError, StoreError};
use crate::event_store::EventStore;
use crate::game_manager::GameManager;
use crate::lighting::LightingManager;
use crate::routes;
use crate::timer_state::TimerState;

/// Shared application state, injected into handlers via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<GameManager>,
    pub timer: Arc<TimerState>,
    pub lighting: Arc<LightingManager>,
}

impl AppState {
    /// Build state backed by a SQLite database at `db_path`.
    pub fn new(db_path: &str) -> Result<Self, StoreError> {
        let store = EventStore::open(db_path)?;
        Ok(Self::with_store(store))
    }

    /// Build state backed by an arbitrary [`EventStore`] (e.g. in-memory tests).
    pub fn with_store(store: EventStore) -> Self {
        let manager = Arc::new(GameManager::new(store));
        let timer = Arc::new(TimerState::new(Arc::clone(&manager)));
        timer.spawn_ticker();
        let lighting = Arc::new(LightingManager::new());
        AppState {
            manager,
            timer,
            lighting,
        }
    }
}

/// An error that maps to an HTTP status and a FastAPI-style `{"detail": ...}` body.
#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub detail: String,
}

impl AppError {
    pub fn new(status: StatusCode, detail: impl Into<String>) -> Self {
        AppError {
            status,
            detail: detail.into(),
        }
    }
    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, detail)
    }
    pub fn not_found(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, detail)
    }
    pub fn conflict(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, detail)
    }
    pub fn timeout(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::REQUEST_TIMEOUT, detail)
    }
    pub fn internal(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, detail)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "detail": self.detail }))).into_response()
    }
}

/// Map domain errors to HTTP responses, so handlers can use `?` directly.
impl From<GameError> for AppError {
    fn from(err: GameError) -> Self {
        match err {
            // Validation failures the caller can fix.
            GameError::InvalidVersion(_) | GameError::GameNotFound(_) => {
                AppError::bad_request(err.to_string())
            }
            // Internal invariants / storage problems.
            GameError::NoActiveGame | GameError::EmptyReplay | GameError::Storage(_) => {
                AppError::internal(err.to_string())
            }
        }
    }
}

impl From<StoreError> for AppError {
    fn from(err: StoreError) -> Self {
        AppError::internal(err.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// A `Json` request extractor whose rejection renders as our `{"detail": ...}`
/// error shape, so malformed or ill-typed request bodies return the same error
/// format as everything else.
pub struct AppJson<T>(pub T);

impl<T, S> FromRequest<S> for AppJson<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(AppJson(value)),
            Err(rejection) => Err(AppError::new(rejection.status(), rejection.body_text())),
        }
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "version": "0.0.1" }))
}

/// OpenAPI document for the Death's Door API. The human-readable usage guide
/// lives in the `info.description` below (served at `/openapi.json`).
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Death's Door — Blood on the Clocktower API",
        version = "1.0.0",
        description = r#"
A Blood on the Clocktower game-management API.

## Setup workflow (important)

A new game starts with an **empty role pool**. The usual sequence is:

1. `POST /game/new` with a `script_name` (e.g. `trouble_brewing`).
2. `POST /characters/add/multi` to build the role pool.
3. `POST /players/add` for each player — each is assigned a **random** role from the pool, which is then removed from the pool. Use `POST /players/add_traveler` to assign a specific traveler instead.
4. `POST /players/set_visibility` with `{ "should_reveal_roles": true }` when roles should become visible. Until then, `GET /players/name/{name}` long-polls (~10s) and returns 408 if reveal is never enabled.

## Real-time updates

`GET /game/stream` is a Server-Sent Events stream: each `data:` frame is the full game-state JSON (the `/game/state` response minus `timer`), sent on connect and after every mutation. REST endpoints remain available for polling.

## Event sourcing

Every mutation is an event. `GET /game/history` returns the log with the version each event produced; `POST /game/rewind` (destructive) and `POST /game/fork` (non-destructive) operate on those versions.

## Errors

All errors share the shape `{ "detail": "<message>" }`. Common statuses: 400 (validation), 404 (not found), 408 (role-reveal timeout), 409 (duplicate player), 422 (malformed or ill-typed request body).
"#
    ),
    tags(
        (name = "Game", description = "Create/manage games, night phases, history, and the SSE stream."),
        (name = "Players", description = "Add/remove players, set status, swap roles, control role visibility."),
        (name = "Characters", description = "Manage the role pool players are drawn from."),
        (name = "Scripts", description = "Browse scripts/editions and their characters and travelers."),
        (name = "Timer", description = "Countdown timer for discussion phases plus APNS push-token registration."),
        (name = "Sounds", description = "Play sound effects."),
        (name = "Lighting", description = "DMX lighting scenes, fixtures, and player spotlights.")
    )
)]
struct ApiDoc;

/// Build the full application router.
pub fn build_router(state: AppState) -> Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::sounds::router())
        .merge(routes::scripts::router())
        .merge(routes::game::router())
        .merge(routes::timer::router())
        .merge(routes::players::router())
        .merge(routes::characters::router())
        .merge(routes::lights::router())
        .split_for_parts();

    let router = router
        .route("/health", get(health))
        .route(
            "/openapi.json",
            get(move || {
                let spec = api.clone();
                async move { Json(spec) }
            }),
        )
        .route_service("/reveal", ServeFile::new("static/role.html"))
        // SSE inspector: raw game-state dump for debugging.
        .route_service("/debug", ServeFile::new("static/debug.html"))
        .nest_service("/static", ServeDir::new("static"))
        // Serve the built frontend (if present) as a fallback so API routes win.
        .fallback_service(ServeDir::new("static/app"))
        .with_state(state);

    router
        // A panic in a handler becomes a 500 and the server stays up.
        .layer(CatchPanicLayer::new())
        // Log method/path/status/latency for every request.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(CorsLayer::permissive())
}
