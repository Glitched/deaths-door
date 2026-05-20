//! HTTP API integration tests, exercised through the real axum router.

mod common;

use axum::http::StatusCode;
use common::{add_player_with_role, get, new_game, post, test_app};
use serde_json::json;

#[tokio::test]
async fn health_ok() {
    let app = test_app();
    let (status, body) = get(&app, "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn new_game_then_script_name() {
    let app = test_app();
    new_game(&app).await;
    let (status, body) = get(&app, "/game/script/name").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!("trouble_brewing"));
}

#[tokio::test]
async fn new_game_unknown_script_is_404() {
    let app = test_app();
    let (status, body) = post(&app, "/game/new", json!({ "script_name": "nope" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["detail"], "Script not found");
}

#[tokio::test]
async fn scripts_list_contains_trouble_brewing() {
    let app = test_app();
    let (status, body) = get(&app, "/scripts/list").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["scripts"]["trouble_brewing"], "Trouble Brewing");
    assert_eq!(body["scripts"]["bad_moon_rising"], "Bad Moon Rising");
}

#[tokio::test]
async fn script_roles_has_22_characters() {
    let app = test_app();
    let (status, body) = get(&app, "/scripts/trouble_brewing/role").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 22);

    let (status, _) = get(&app, "/scripts/unknown/role").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn script_role_and_travelers_lookup() {
    let app = test_app();
    let (status, body) = get(&app, "/scripts/trouble_brewing/role/Imp").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Imp");
    assert_eq!(body["category"], "demon");
    assert_eq!(body["alignment"], "evil");
    assert_eq!(body["icon_path"], "imp.png");

    let (status, body) = get(&app, "/scripts/trouble_brewing/travelers").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn characters_add_list_remove() {
    let app = test_app();
    new_game(&app).await;

    let (status, body) = post(
        &app,
        "/characters/add/multi",
        json!({ "names": ["Imp", "Chef", "Baron"] }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["added_count"], 3);

    let (status, body) = get(&app, "/characters/list").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 3);

    let (status, _) = post(&app, "/characters/remove", json!({ "name": "Chef" })).await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = get(&app, "/characters/list").await;
    assert_eq!(body.as_array().unwrap().len(), 2);

    // Unknown role can't be added.
    let (status, _) = post(&app, "/characters/add", json!({ "name": "NotARole" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn add_player_deterministic_and_duplicate_and_empty_pool() {
    let app = test_app();
    new_game(&app).await;

    // No roles yet -> 400.
    let (status, _) = post(&app, "/players/add", json!({ "name": "Alice" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let body = add_player_with_role(&app, "Alice", "Imp").await;
    assert_eq!(body["name"], "Alice");
    assert_eq!(body["character"]["name"], "Imp");
    assert_eq!(body["alignment"], "evil");
    assert_eq!(body["is_alive"], true);

    // Duplicate name -> 409.
    post(&app, "/characters/add", json!({ "name": "Chef" })).await;
    let (status, _) = post(&app, "/players/add", json!({ "name": "Alice" })).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn add_traveler_valid_and_invalid() {
    let app = test_app();
    new_game(&app).await;

    let (status, body) = post(
        &app,
        "/players/add_traveler",
        json!({ "name": "Wanderer", "traveler": "Beggar" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["character"]["name"], "Beggar");

    let (status, _) = post(
        &app,
        "/players/add_traveler",
        json!({ "name": "Other", "traveler": "NotATraveler" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn killing_poisoner_clears_poisoned_effect() {
    let app = test_app();
    new_game(&app).await;
    add_player_with_role(&app, "Pat", "Poisoner").await;
    add_player_with_role(&app, "Cara", "Chef").await;

    post(
        &app,
        "/players/add_status_effect",
        json!({ "name": "Cara", "status_effect": "Poisoned" }),
    )
    .await;

    let (status, _) = post(
        &app,
        "/players/set_alive",
        json!({ "name": "Pat", "is_alive": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, players) = get(&app, "/players/list").await;
    let cara = players
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "Cara")
        .unwrap();
    assert_eq!(cara["status_effects"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn visibility_and_role_reveal() {
    let app = test_app();
    new_game(&app).await;
    add_player_with_role(&app, "Alice", "Imp").await;

    let (_, vis) = get(&app, "/players/visibility").await;
    assert_eq!(vis, json!(false));

    let (status, vis) = post(
        &app,
        "/players/set_visibility",
        json!({ "should_reveal_roles": true }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(vis, json!(true));

    // With visibility on, the reveal endpoint returns immediately.
    let (status, body) = get(&app, "/players/name/Alice").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["character"]["name"], "Imp");

    // Unknown player -> 404.
    let (status, _) = get(&app, "/players/name/Nobody").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rename_swap_alignment_remove() {
    let app = test_app();
    new_game(&app).await;
    add_player_with_role(&app, "Alice", "Imp").await;
    add_player_with_role(&app, "Bob", "Chef").await;

    // Rename
    let (status, body) = post(
        &app,
        "/players/rename",
        json!({ "name": "Alice", "new_name": "Alice2" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Alice2");

    // Swap
    let (status, body) = post(
        &app,
        "/players/swap_character",
        json!({ "name1": "Alice2", "name2": "Bob" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["player1"]["character"]["name"], "Chef");
    assert_eq!(body["player2"]["character"]["name"], "Imp");

    // Set alignment
    let (status, body) = post(
        &app,
        "/players/set_alignment",
        json!({ "name": "Bob", "alignment": "good" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["alignment"], "good");

    // Remove
    let (status, body) = post(&app, "/players/remove", json!({ "name": "Bob" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["remaining_players"], json!(["Alice2"]));

    // Removing a missing player -> 404.
    let (status, _) = post(&app, "/players/remove", json!({ "name": "Ghost" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn game_state_shape() {
    let app = test_app();
    new_game(&app).await;
    add_player_with_role(&app, "Alice", "Imp").await;

    let (status, body) = get(&app, "/game/state").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["script_name"], "trouble_brewing");
    assert_eq!(body["players"].as_array().unwrap().len(), 1);
    assert_eq!(body["living_player_count"], 1);
    assert_eq!(body["execution_threshold"], 1);
    assert!(body["timer"]["is_running"].is_boolean());
    assert!(body["night_steps"].is_array());
}

#[tokio::test]
async fn history_rewind_fork_load_list() {
    let app = test_app();
    new_game(&app).await;
    add_player_with_role(&app, "Alice", "Imp").await;
    add_player_with_role(&app, "Bob", "Chef").await;

    let (status, hist) = get(&app, "/game/history").await;
    assert_eq!(status, StatusCode::OK);
    let version = hist["version"].as_i64().unwrap();
    assert!(version >= 5);
    assert!(hist["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["description"] == "Alice joined as Imp"));

    // Fork from version 2 into a new game.
    let (status, fork) = post(&app, "/game/fork", json!({ "from_version": 2 })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fork["version"], 2);

    // List shows at least the original + the fork.
    let (_, list) = get(&app, "/game/list").await;
    assert!(list["game_ids"].as_array().unwrap().len() >= 2);

    // Rewind the (now active, forked) game to version 1.
    let (status, _) = post(&app, "/game/rewind", json!({ "to_version": 1 })).await;
    assert_eq!(status, StatusCode::OK);

    // Invalid version -> 400.
    let (status, _) = post(&app, "/game/rewind", json!({ "to_version": 999 })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn night_phase_controls() {
    let app = test_app();
    new_game(&app).await;

    let (status, body) = post(
        &app,
        "/game/night/phase/step",
        json!({ "step": "Poisoner" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["current_night_step"], "Poisoner");

    // Toggling first night resets the step to Dusk.
    let (status, body) = post(
        &app,
        "/game/night/phase/first_night",
        json!({ "is_first_night": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["is_first_night"], false);
    assert_eq!(body["current_night_step"], "Dusk");
}

#[tokio::test]
async fn timer_endpoints() {
    let app = test_app();

    let (status, body) = get(&app, "/timer/set/300").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["seconds"], 300);
    assert_eq!(body["is_running"], true);

    let (_, body) = get(&app, "/timer/fetch").await;
    assert_eq!(body["seconds"], 300);

    let (status, body) = get(&app, "/timer/stop").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["is_running"], false);

    // Out-of-range -> 400.
    let (status, _) = get(&app, "/timer/set/99999").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn sounds_and_lights_basics() {
    let app = test_app();

    let (status, body) = get(&app, "/sounds/list").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["death"].is_array());

    let (status, _) = get(&app, "/sounds/play/notasound").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Lighting degrades gracefully with no hardware.
    let (status, body) = get(&app, "/lights/status").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["connected"], false);

    let (status, body) = get(&app, "/lights/scenes/list").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["scenes"].as_array().unwrap().len(), 8);

    let (status, _) = post(&app, "/lights/scene/notascene", json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = post(&app, "/lights/blackout", json!({})).await;
    assert_eq!(status, StatusCode::OK);

    // Spotlight without a calibrated position -> 404 (player 999 is never calibrated).
    let (status, _) = post(&app, "/lights/spotlight/player/999", json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn sse_stream_emits_initial_state() {
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = test_app();
    new_game(&app).await;
    add_player_with_role(&app, "Alice", "Imp").await;

    let req = Request::builder()
        .uri("/game/stream")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        content_type.contains("text/event-stream"),
        "got {content_type}"
    );

    // Read frames until the first complete `data:` event (bounded by a timeout
    // so the never-ending stream can't hang the test).
    let mut body = resp.into_body();
    let mut buf = String::new();
    let read = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(Ok(frame)) = body.frame().await {
            if let Some(data) = frame.data_ref() {
                buf.push_str(&String::from_utf8_lossy(data));
                if buf.contains("data:") && buf.contains("\n\n") {
                    break;
                }
            }
        }
    })
    .await;
    assert!(read.is_ok(), "timed out waiting for an SSE frame");

    // The frame payload is the game-state JSON.
    let start = buf.find('{').expect("no JSON in SSE frame");
    let end = buf.rfind('}').expect("no JSON in SSE frame");
    let payload: serde_json::Value = serde_json::from_str(&buf[start..=end]).unwrap();
    assert_eq!(payload["script_name"], "trouble_brewing");
    assert_eq!(payload["players"][0]["character"]["name"], "Imp");
}
