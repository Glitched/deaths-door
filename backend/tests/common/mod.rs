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

/// GET that must succeed; returns just the body.
pub async fn get_ok(app: &Router, uri: &str) -> Value {
    let (status, body) = get(app, uri).await;
    assert_eq!(status, StatusCode::OK, "GET {uri} failed: {body}");
    body
}

/// POST that must succeed; returns just the body.
pub async fn post_ok(app: &Router, uri: &str, body: Value) -> Value {
    let (status, resp) = post(app, uri, body).await;
    assert_eq!(status, StatusCode::OK, "POST {uri} failed: {resp}");
    resp
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

/// Add several roles to the game's pool in one atomic call.
pub async fn add_roles(app: &Router, names: &[&str]) {
    post_ok(
        app,
        "/characters/add/multi",
        serde_json::json!({ "names": names }),
    )
    .await;
}

/// Create a Trouble Brewing game with the given (player, role) pairs. Each pair
/// is added one at a time so role assignment is deterministic.
pub async fn game_with_players(app: &Router, players: &[(&str, &str)]) {
    new_game(app).await;
    for (player, role) in players {
        add_player_with_role(app, player, role).await;
    }
}

/// Find a player by name in a `/players/list`-shaped JSON array.
pub fn find_player<'a>(players: &'a Value, name: &str) -> &'a Value {
    players
        .as_array()
        .expect("player list should be a JSON array")
        .iter()
        .find(|p| p["name"] == name)
        .unwrap_or_else(|| panic!("player {name} not found in {players}"))
}
