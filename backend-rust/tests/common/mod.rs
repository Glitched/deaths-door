//! Shared helpers for API integration tests.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use deaths_door::app::{build_router, AppState};
use deaths_door::event_store::EventStore;

/// Build a fresh app backed by an in-memory event store.
pub fn test_app() -> Router {
    let state = AppState::with_store(EventStore::in_memory().expect("in-memory store"));
    build_router(state)
}

/// Send a request and return the status plus parsed JSON body (Null if empty).
pub async fn request(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(serde_json::to_vec(&v).unwrap())
        }
        None => Body::empty(),
    };
    let req = builder.body(body).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

pub async fn get(app: &Router, uri: &str) -> (StatusCode, Value) {
    request(app, "GET", uri, None).await
}

pub async fn post(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    request(app, "POST", uri, Some(body)).await
}

/// Create a new Trouble Brewing game.
pub async fn new_game(app: &Router) {
    let (status, _) = post(
        app,
        "/game/new",
        serde_json::json!({ "script_name": "trouble_brewing" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

/// Add a single role to the pool, then add a player who will deterministically
/// receive it (since it's the only role available).
pub async fn add_player_with_role(app: &Router, player: &str, role: &str) -> Value {
    let (s1, _) = post(app, "/characters/add", serde_json::json!({ "name": role })).await;
    assert_eq!(s1, StatusCode::OK, "adding role {role}");
    let (s2, body) = post(app, "/players/add", serde_json::json!({ "name": player })).await;
    assert_eq!(s2, StatusCode::OK, "adding player {player}");
    body
}
