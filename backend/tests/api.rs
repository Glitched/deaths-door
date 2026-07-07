//! HTTP API integration tests, exercised through the real axum router.

mod common;

use axum::http::StatusCode;
use common::{
    add_player_with_role, add_roles, find_player, game_with_players, get, get_ok, get_text,
    new_game, post, post_ok, test_app,
};
use serde_json::{json, Value};

/// Extract the `name` field from each element of a JSON array.
fn names_of(v: &Value) -> Vec<&str> {
    v.as_array()
        .expect("expected a JSON array")
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect()
}

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
async fn set_demon_bluffs_appears_in_state() {
    let app = test_app();
    new_game(&app).await;

    // Case-insensitive input is canonicalized to the script's character names.
    let (status, body) = post(
        &app,
        "/game/bluffs",
        json!({ "bluffs": ["mayor", "Slayer", "empath"] }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = body["demon_bluffs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Mayor", "Slayer", "Empath"]);

    // Bluffs are resolved to full character objects in the state snapshot.
    let (status, body) = get(&app, "/game/state").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["demon_bluffs"][0]["name"], "Mayor");
    assert!(body["demon_bluffs"][0]["description"]
        .as_str()
        .is_some_and(|d| !d.is_empty()));

    // An empty list clears them.
    let (status, body) = post(&app, "/game/bluffs", json!({ "bluffs": [] })).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["demon_bluffs"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn set_demon_bluffs_validates_input() {
    let app = test_app();
    new_game(&app).await;

    // More than 3 -> 400.
    let (status, _) = post(
        &app,
        "/game/bluffs",
        json!({ "bluffs": ["Mayor", "Slayer", "Empath", "Chef"] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Unknown role -> 404.
    let (status, _) = post(&app, "/game/bluffs", json!({ "bluffs": ["Nonexistent"] })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn chopping_block_set_clear_and_validation() {
    let app = test_app();
    new_game(&app).await;
    add_player_with_role(&app, "Alice", "Imp").await;
    add_player_with_role(&app, "Bob", "Chef").await;

    // Nothing on the block initially.
    let (_, body) = get(&app, "/game/state").await;
    assert_eq!(body["chopping_block"], json!(null));

    // Unknown player -> 404.
    let (status, _) = post(
        &app,
        "/game/chopping_block",
        json!({ "player_name": "Nobody" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Set with votes; appears in the returned (and fetched) state.
    let (status, body) = post(
        &app,
        "/game/chopping_block",
        json!({ "player_name": "Alice", "votes": 4 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["chopping_block"]["player_name"], "Alice");
    assert_eq!(body["chopping_block"]["votes"], 4);

    // Votes are optional; a new nomination replaces the block.
    let (status, body) = post(
        &app,
        "/game/chopping_block",
        json!({ "player_name": "Bob" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["chopping_block"]["player_name"], "Bob");
    assert_eq!(body["chopping_block"]["votes"], json!(null));

    // Explicit clear; clearing again is safe.
    let (status, body) = post(&app, "/game/chopping_block/clear", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["chopping_block"], json!(null));
    let (status, _) = post(&app, "/game/chopping_block/clear", json!({})).await;
    assert_eq!(status, StatusCode::OK);

    // A dead player can't be put on the block.
    post(
        &app,
        "/players/set_alive",
        json!({ "name": "Bob", "is_alive": false }),
    )
    .await;
    let (status, _) = post(
        &app,
        "/game/chopping_block",
        json!({ "player_name": "Bob" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn chopping_block_auto_clears_on_death_and_night() {
    let app = test_app();
    new_game(&app).await;
    add_player_with_role(&app, "Alice", "Imp").await;

    // Killing the player on the block clears it.
    post(
        &app,
        "/game/chopping_block",
        json!({ "player_name": "Alice", "votes": 3 }),
    )
    .await;
    let (status, _) = post(
        &app,
        "/players/set_alive",
        json!({ "name": "Alice", "is_alive": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = get(&app, "/game/state").await;
    assert_eq!(body["chopping_block"], json!(null));

    // Night falling clears the block.
    post(
        &app,
        "/players/set_alive",
        json!({ "name": "Alice", "is_alive": true }),
    )
    .await;
    post(
        &app,
        "/game/chopping_block",
        json!({ "player_name": "Alice" }),
    )
    .await;
    let (status, _) = post(&app, "/game/night/phase/step", json!({ "step": "Dusk" })).await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = get(&app, "/game/state").await;
    assert_eq!(body["chopping_block"], json!(null));
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
async fn script_browsing_endpoints() {
    let app = test_app();

    let roles = get_ok(&app, "/scripts/trouble_brewing/role").await;
    assert_eq!(roles.as_array().unwrap().len(), 22);

    let body = get_ok(&app, "/scripts/trouble_brewing/role/Imp").await;
    assert_eq!(body["name"], "Imp");
    assert_eq!(body["category"], "demon");
    assert_eq!(body["alignment"], "evil");
    assert_eq!(body["icon_path"], "imp.png");

    let travelers = get_ok(&app, "/scripts/trouble_brewing/travelers").await;
    assert_eq!(travelers.as_array().unwrap().len(), 5);

    // Unknown script or role -> 404.
    let (status, _) = get(&app, "/scripts/unknown/role").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = get(&app, "/scripts/trouble_brewing/role/NotARole").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
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
    game_with_players(&app, &[("Pat", "Poisoner"), ("Cara", "Chef")]).await;

    post_ok(
        &app,
        "/players/add_status_effect",
        json!({ "name": "Cara", "status_effect": "Poisoned" }),
    )
    .await;
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Pat", "is_alive": false }),
    )
    .await;

    let players = get_ok(&app, "/players/list").await;
    let cara = find_player(&players, "Cara");
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
    // Day/night cycle fields (games open on the first night).
    assert_eq!(body["phase"], "night");
    assert_eq!(body["day_number"], 0);
    assert_eq!(body["deaths_to_announce"], json!([]));
    assert_eq!(body["nominations_today"], json!([]));
    assert_eq!(body["eligible_voters"], json!(["Alice"]));
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

    // Out-of-range versions -> 400 for both rewind and fork.
    for body in [json!({ "to_version": 999 }), json!({ "to_version": 0 })] {
        let (status, _) = post(&app, "/game/rewind", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    for body in [json!({ "from_version": 999 }), json!({ "from_version": 0 })] {
        let (status, _) = post(&app, "/game/fork", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
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

    // Adding seconds adjusts the countdown; a large negative delta clamps at 0.
    let body = get_ok(&app, "/timer/add/60").await;
    assert_eq!(body["seconds"], 360);
    let body = get_ok(&app, "/timer/add/-3600").await;
    assert_eq!(body["seconds"], 0);

    // Start can set the countdown in the same call.
    let body = get_ok(&app, "/timer/start?seconds=120").await;
    assert_eq!(body["is_running"], true);
    assert_eq!(body["seconds"], 120);

    // Out-of-range -> 400.
    let (status, _) = get(&app, "/timer/set/99999").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = get(&app, "/timer/add/-99999").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Push-token registration is accepted (APNS degrades gracefully sans keys).
    let body = post_ok(&app, "/timer/push_token", json!({ "push_token": "abc" })).await;
    assert_eq!(body["status"], "registered");
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
    let scenes: Vec<&str> = body["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    for expected in ["death", "drama", "goodnight", "morning", "blackout"] {
        assert!(scenes.contains(&expected), "missing scene {expected}");
    }

    let (status, _) = post(&app, "/lights/scene/notascene", json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = post(&app, "/lights/blackout", json!({})).await;
    assert_eq!(status, StatusCode::OK);

    // Spotlight without a calibrated position -> 404 (player 999 is never calibrated).
    let (status, _) = post(&app, "/lights/spotlight/player/999", json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn scene_effect_lifecycle() {
    let app = test_app();
    new_game(&app).await;

    // Trigger the death scene silently (no audio during tests). Its effect
    // length follows the death.wav audio (~1.5s), not the 3s default.
    let (status, body) = post(&app, "/lights/scene/death?silent=true", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "success");
    assert_eq!(body["effect"]["scene"], "death");
    assert_eq!(body["sound"], json!(null));
    let duration = body["effect"]["duration_ms"].as_u64().unwrap();
    assert!(
        (1000..3000).contains(&duration),
        "death effect should follow its ~1.5s audio, got {duration}ms"
    );

    // While playing, the effect is visible in the game state snapshot.
    let (_, state) = get(&app, "/game/state").await;
    assert_eq!(state["active_effect"]["scene"], "death");

    // A new trigger supersedes the old one (id increments, scene swaps).
    let first_id = body["effect"]["id"].as_u64().unwrap();
    let (_, body2) = post(&app, "/lights/scene/morning?silent=true", json!({})).await;
    assert!(body2["effect"]["id"].as_u64().unwrap() > first_id);
    let (_, state) = get(&app, "/game/state").await;
    assert_eq!(state["active_effect"]["scene"], "morning");

    // Blackout is instant: zero duration, no lingering active effect.
    let (status, body) = post(&app, "/lights/scene/blackout", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["effect"]["duration_ms"], 0);
    let (_, state) = get(&app, "/game/state").await;
    assert_eq!(state["active_effect"], json!(null));

    // The deprecated integrated alias still works. (Fog has no paired sound,
    // so this stays silent during test runs and gets the default duration.)
    let (status, body) = post(&app, "/lights/scene/integrated/fog", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["effect"]["scene"], "fog");
    assert_eq!(body["sound"], json!(null));
    assert_eq!(body["effect"]["duration_ms"], 3000);
}

#[tokio::test]
async fn auto_effects_toggle_individually_and_fire_on_game_events() {
    // Auto-triggers play real sounds; mute the server so test runs stay quiet.
    std::env::set_var("DEATHS_DOOR_MUTE", "1");

    let app = test_app();
    game_with_players(&app, &[("Alice", "Imp"), ("Bob", "Chef")]).await;

    // Everything defaults off...
    let body = get_ok(&app, "/lights/auto_effects").await;
    assert_eq!(
        body,
        json!({ "death": false, "nomination": false, "goodnight": false, "morning": false })
    );

    // ...so a death fires nothing.
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Bob", "is_alive": false }),
    )
    .await;
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["active_effect"], json!(null));
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Bob", "is_alive": true }),
    )
    .await;

    // Toggles are individual; omitted fields keep their value.
    let body = post_ok(&app, "/lights/auto_effects", json!({ "death": true })).await;
    assert_eq!(body["death"], true);
    assert_eq!(body["nomination"], false);
    let body = post_ok(
        &app,
        "/lights/auto_effects",
        json!({ "nomination": true, "morning": true, "goodnight": true }),
    )
    .await;
    assert_eq!(body["death"], true);

    // Even with the death toggle on, night deaths stay silent (sneaky).
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Bob", "is_alive": false }),
    )
    .await;
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["active_effect"], json!(null));

    // Day breaks -> morning fires.
    post_ok(&app, "/game/day/begin", json!({})).await;
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["active_effect"]["scene"], "morning");

    // A daytime death fires the death scene (revival never does).
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Bob", "is_alive": true }),
    )
    .await;
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Bob", "is_alive": false }),
    )
    .await;
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["active_effect"]["scene"], "death");

    // A nomination fires drama.
    post_ok(
        &app,
        "/game/chopping_block",
        json!({ "player_name": "Alice" }),
    )
    .await;
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["active_effect"]["scene"], "drama");

    // Night falls -> goodnight.
    post_ok(&app, "/game/night/begin", json!({})).await;
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["active_effect"]["scene"], "goodnight");
    let goodnight_id = state["active_effect"]["id"].as_u64().unwrap();

    // Moving between two night steps re-fires nothing (same effect id).
    post_ok(&app, "/game/night/phase/step", json!({ "step": "Imp" })).await;
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["active_effect"]["id"].as_u64().unwrap(), goodnight_id);

    // Raw step bookmarks still cross phases: Dawn -> morning, and the
    // first-night toggle (resets to Dusk) -> goodnight again.
    post_ok(&app, "/game/night/phase/step", json!({ "step": "Dawn" })).await;
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["active_effect"]["scene"], "morning");
    post_ok(
        &app,
        "/game/night/phase/first_night",
        json!({ "is_first_night": false }),
    )
    .await;
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["active_effect"]["scene"], "goodnight");
    assert!(state["active_effect"]["id"].as_u64().unwrap() > goodnight_id);
}

#[tokio::test]
async fn day_cycle_announces_overnight_deaths() {
    // Announcements always fire the death scene; keep the test run quiet.
    std::env::set_var("DEATHS_DOOR_MUTE", "1");

    let app = test_app();
    game_with_players(&app, &[("Alice", "Imp"), ("Bob", "Chef")]).await;

    // A night kill queues for announcement instead of being public.
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Bob", "is_alive": false }),
    )
    .await;
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["deaths_to_announce"], json!(["Bob"]));

    // Announcing is a daytime action.
    let (status, _) = post(
        &app,
        "/game/day/announce_death",
        json!({ "player_name": "Bob" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Dawn breaks; the queue carries over. Beginning the day twice is a 400.
    let state = post_ok(&app, "/game/day/begin", json!({})).await;
    assert_eq!(state["phase"], "day");
    assert_eq!(state["day_number"], 1);
    assert_eq!(state["deaths_to_announce"], json!(["Bob"]));
    let (status, _) = post(&app, "/game/day/begin", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Only queued players can be announced.
    for wrong in ["Ghost", "Alice"] {
        let (status, _) = post(
            &app,
            "/game/day/announce_death",
            json!({ "player_name": wrong }),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "announcing {wrong}");
    }

    // Announcing checks Bob off and fires the death scene.
    let state = post_ok(
        &app,
        "/game/day/announce_death",
        json!({ "player_name": "Bob" }),
    )
    .await;
    assert_eq!(state["deaths_to_announce"], json!([]));
    assert_eq!(state["active_effect"]["scene"], "death");
    let (status, _) = post(
        &app,
        "/game/day/announce_death",
        json!({ "player_name": "Bob" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Night falls; a night after a day is not the first night.
    let state = post_ok(&app, "/game/night/begin", json!({})).await;
    assert_eq!(state["phase"], "night");
    assert_eq!(state["is_first_night"], false);
    let (status, _) = post(&app, "/game/night/begin", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn nomination_votes_resolve_the_chopping_block() {
    let app = test_app();
    game_with_players(
        &app,
        &[
            ("Alice", "Imp"),
            ("Bob", "Chef"),
            ("Carol", "Empath"),
            ("Dave", "Mayor"),
            ("Erin", "Monk"),
            ("Frank", "Slayer"),
        ],
    )
    .await;

    // Nominations are a daytime action.
    let (status, _) = post(
        &app,
        "/game/nominations",
        json!({ "player_name": "Alice", "voters": ["Bob"] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Dave dies overnight (he keeps a dead vote), then day breaks:
    // 5 living -> threshold 3.
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Dave", "is_alive": false }),
    )
    .await;
    let state = post_ok(&app, "/game/day/begin", json!({})).await;
    assert_eq!(state["execution_threshold"], 3);
    assert_eq!(
        state["eligible_voters"],
        json!(["Alice", "Bob", "Carol", "Dave", "Erin", "Frank"])
    );

    // Below the threshold: nothing changes.
    let body = post_ok(
        &app,
        "/game/nominations",
        json!({ "player_name": "Alice", "voters": ["Bob", "Carol"] }),
    )
    .await;
    assert_eq!(body["outcome"], "block_unchanged");
    assert_eq!(body["chopping_block"], json!(null));

    // Meeting it takes the empty block; dead Dave's vote is spent by voting.
    let body = post_ok(
        &app,
        "/game/nominations",
        json!({ "player_name": "Bob", "voters": ["Alice", "Dave", "Erin"] }),
    )
    .await;
    assert_eq!(body["outcome"], "on_the_block");
    assert_eq!(body["chopping_block"]["player_name"], "Bob");
    assert_eq!(body["chopping_block"]["votes"], 3);
    assert_eq!(
        body["eligible_voters"],
        json!(["Alice", "Bob", "Carol", "Erin", "Frank"])
    );

    // A spent dead vote can't vote again.
    let (status, body) = post(
        &app,
        "/game/nominations",
        json!({ "player_name": "Carol", "voters": ["Dave"] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // Tying the block empties it: no one is executed on a tie.
    let body = post_ok(
        &app,
        "/game/nominations",
        json!({ "player_name": "Carol", "voters": ["Alice", "Erin", "Frank"] }),
    )
    .await;
    assert_eq!(body["outcome"], "tie_block_emptied");
    assert_eq!(body["chopping_block"], json!(null));

    // Beating retakes the block.
    let body = post_ok(
        &app,
        "/game/nominations",
        json!({ "player_name": "Erin", "voters": ["Alice", "Bob", "Carol", "Frank"] }),
    )
    .await;
    assert_eq!(body["outcome"], "on_the_block");
    assert_eq!(body["chopping_block"]["player_name"], "Erin");

    // Validation: repeat nominee, dead nominee, unknowns, duplicate voter.
    let (status, _) = post(
        &app,
        "/game/nominations",
        json!({ "player_name": "Bob", "voters": ["Alice"] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST); // already nominated today
    let (status, _) = post(
        &app,
        "/game/nominations",
        json!({ "player_name": "Dave", "voters": ["Alice"] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST); // dead nominee
    let (status, _) = post(
        &app,
        "/game/nominations",
        json!({ "player_name": "Ghost", "voters": ["Alice"] }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = post(
        &app,
        "/game/nominations",
        json!({ "player_name": "Frank", "voters": ["Ghost"] }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = post(
        &app,
        "/game/nominations",
        json!({ "player_name": "Frank", "voters": ["Alice", "Alice"] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A vote total can outweigh the voter count (e.g. a Bureaucrat's vote).
    let body = post_ok(
        &app,
        "/game/nominations",
        json!({ "player_name": "Frank", "voters": ["Alice", "Bob"], "votes": 5 }),
    )
    .await;
    assert_eq!(body["outcome"], "on_the_block");
    assert_eq!(body["chopping_block"]["player_name"], "Frank");
    assert_eq!(body["chopping_block"]["votes"], 5);
    assert_eq!(body["nominations_today"].as_array().unwrap().len(), 5);

    // Ending the day executes whoever is on the block — publicly, so nothing
    // queues for announcement. Ending again is a safe no-op.
    let body = post_ok(&app, "/game/day/end", json!({})).await;
    assert_eq!(body["executed"], "Frank");
    assert_eq!(body["chopping_block"], json!(null));
    // Only Dave's unannounced night death is queued — the execution isn't.
    assert_eq!(body["deaths_to_announce"], json!(["Dave"]));
    let frank = find_player(&get_ok(&app, "/players/list").await, "Frank").clone();
    assert_eq!(frank["is_alive"], false);
    let body = post_ok(&app, "/game/day/end", json!({})).await;
    assert_eq!(body["executed"], json!(null));

    // Night wipes the day's nomination record.
    let state = post_ok(&app, "/game/night/begin", json!({})).await;
    assert_eq!(state["nominations_today"], json!([]));
}

#[tokio::test]
async fn live_vote_tally_lifecycle() {
    let app = test_app();
    game_with_players(
        &app,
        &[("Alice", "Imp"), ("Bob", "Chef"), ("Carol", "Empath")],
    )
    .await;

    // Tallies are a daytime thing, and need a session to update.
    let (status, _) = post(&app, "/game/vote/start", json!({ "player_name": "Alice" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    post_ok(&app, "/game/day/begin", json!({})).await;
    let (status, _) = post(&app, "/game/vote/voters", json!({ "voters": ["Bob"] })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = post(&app, "/game/vote/start", json!({ "player_name": "Ghost" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Start a tally; it appears in the state (and thus every SSE frame).
    let state = post_ok(&app, "/game/vote/start", json!({ "player_name": "Alice" })).await;
    assert_eq!(state["vote_in_progress"]["player_name"], "Alice");
    assert_eq!(state["vote_in_progress"]["voters"], json!([]));

    // Bad voter lists are rejected and leave the tally alone.
    let (status, _) = post(
        &app,
        "/game/vote/voters",
        json!({ "voters": ["Bob", "Ghost"] }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = post(
        &app,
        "/game/vote/voters",
        json!({ "voters": ["Bob", "Bob"] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["vote_in_progress"]["player_name"], "Alice");

    // Each update replaces the whole selection (deselects included).
    post_ok(
        &app,
        "/game/vote/voters",
        json!({ "voters": ["Bob", "Carol"] }),
    )
    .await;
    let state = post_ok(&app, "/game/vote/voters", json!({ "voters": ["Bob"] })).await;
    assert_eq!(state["vote_in_progress"]["voters"], json!(["Bob"]));

    // Cancel abandons the tally without recording anything.
    let state = post_ok(&app, "/game/vote/cancel", json!({})).await;
    assert_eq!(state["vote_in_progress"], json!(null));
    assert_eq!(state["nominations_today"], json!([]));

    // Confirm records the nomination exactly like POST /game/nominations.
    post_ok(&app, "/game/vote/start", json!({ "player_name": "Alice" })).await;
    post_ok(
        &app,
        "/game/vote/voters",
        json!({ "voters": ["Bob", "Carol"] }),
    )
    .await;
    let body = post_ok(&app, "/game/vote/confirm", json!({})).await;
    assert_eq!(body["outcome"], "on_the_block");
    assert_eq!(body["chopping_block"]["player_name"], "Alice");
    assert_eq!(body["chopping_block"]["votes"], 2);
    assert_eq!(body["vote_in_progress"], json!(null));
    let (status, _) = post(&app, "/game/vote/confirm", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A confirm that fails validation keeps the tally up for fixing.
    post_ok(&app, "/game/vote/start", json!({ "player_name": "Carol" })).await;
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Carol", "is_alive": false }),
    )
    .await;
    let (status, _) = post(&app, "/game/vote/confirm", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST); // nominee died mid-tally
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["vote_in_progress"]["player_name"], "Carol");

    // Night falling drops any open tally.
    post_ok(&app, "/game/night/begin", json!({})).await;
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["vote_in_progress"], json!(null));
}

#[tokio::test]
async fn game_over_hint_and_recorded_winner() {
    let app = test_app();
    game_with_players(
        &app,
        &[("Alice", "Imp"), ("Bob", "Chef"), ("Carol", "Empath")],
    )
    .await;

    // Three living players and a living demon: nothing to report.
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["game_over_hint"], json!(null));
    assert_eq!(state["winner"], json!(null));

    // The demon dying suggests good won.
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Alice", "is_alive": false }),
    )
    .await;
    let state = get_ok(&app, "/game/state").await;
    let hint = state["game_over_hint"].as_str().unwrap();
    assert!(hint.contains("demon is dead"), "{hint}");

    // Two players left with the demon standing suggests evil won.
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Alice", "is_alive": true }),
    )
    .await;
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Bob", "is_alive": false }),
    )
    .await;
    let state = get_ok(&app, "/game/state").await;
    let hint = state["game_over_hint"].as_str().unwrap();
    assert!(hint.contains("evil may have won"), "{hint}");

    // The storyteller makes the call; unknown isn't a team.
    let (status, _) = post(&app, "/game/end", json!({ "winner": "unknown" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let state = post_ok(&app, "/game/end", json!({ "winner": "evil" })).await;
    assert_eq!(state["winner"], "evil");
    assert_eq!(state["game_over_hint"], json!(null));
    let (status, _) = post(&app, "/game/end", json!({ "winner": "good" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn journal_reads_like_a_story() {
    // Announcements fire the death scene; keep the test run quiet.
    std::env::set_var("DEATHS_DOOR_MUTE", "1");

    let app = test_app();
    game_with_players(&app, &[("Alice", "Imp"), ("Bob", "Chef")]).await;

    // A tiny game: Bob dies overnight, is announced at dawn, Alice gets
    // nominated and executed, evil wins.
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Bob", "is_alive": false }),
    )
    .await;
    post_ok(&app, "/game/day/begin", json!({})).await;
    post_ok(
        &app,
        "/game/day/announce_death",
        json!({ "player_name": "Bob" }),
    )
    .await;
    post_ok(
        &app,
        "/game/nominations",
        json!({ "player_name": "Alice", "voters": ["Alice"] }),
    )
    .await;
    post_ok(&app, "/game/day/end", json!({})).await;
    post_ok(&app, "/game/end", json!({ "winner": "evil" })).await;

    let (status, journal) = get_text(&app, "/game/journal").await;
    assert_eq!(status, StatusCode::OK);
    for expected in [
        "# Blood on the Clocktower: Trouble Brewing",
        "## Setup & first night",
        "- Alice joined as Imp",
        "- Bob died",
        "## Day 1",
        "- Bob's death was announced",
        "- Alice was nominated: 1 vote (Alice)",
        "- Alice was executed",
        "**Evil wins!**",
    ] {
        assert!(
            journal.contains(expected),
            "journal missing {expected:?}:\n{journal}"
        );
    }
}

#[tokio::test]
async fn scene_sound_override_sets_effect_length() {
    let app = test_app();
    new_game(&app).await;

    // The Wilhelm scream (~2.6s) replaces the death sting (~1.5s), and the
    // effect length follows it. Silent so test runs stay quiet.
    let body = post_ok(
        &app,
        "/lights/scene/death?silent=true&sound=wilhelm",
        json!({}),
    )
    .await;
    assert_eq!(body["effect"]["scene"], "death");
    let duration = body["effect"]["duration_ms"].as_u64().unwrap();
    assert!(
        (2000..3500).contains(&duration),
        "expected a wilhelm-length effect, got {duration}ms"
    );

    // Unknown sound -> 404.
    let (status, _) = post(
        &app,
        "/lights/scene/death?silent=true&sound=nope",
        json!({}),
    )
    .await;
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

#[tokio::test]
async fn openapi_spec_is_served() {
    let app = test_app();
    let body = get_ok(&app, "/openapi.json").await;
    assert!(body["openapi"].as_str().unwrap().starts_with("3."));
    assert!(body["info"]["title"]
        .as_str()
        .unwrap()
        .contains("Death's Door"));

    let paths = body["paths"].as_object().unwrap();
    for path in [
        "/game/state",
        "/game/stream",
        "/players/add",
        "/characters/add/multi",
        "/lights/scene/{name}",
        "/timer/set/{seconds}",
    ] {
        assert!(paths.contains_key(path), "spec is missing {path}");
    }
}

#[tokio::test]
async fn malformed_request_bodies_use_the_error_shape() {
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = test_app();
    new_game(&app).await;

    // Wrong field type -> 422 with a {"detail": ...} body.
    let (status, body) = post(&app, "/players/add", json!({ "name": 42 })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["detail"].is_string(), "expected detail, got {body}");

    // Invalid JSON syntax -> 400 with the same shape.
    let req = Request::builder()
        .method("POST")
        .uri("/players/add")
        .header("content-type", "application/json")
        .body(Body::from("{not json"))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["detail"].is_string(), "expected detail, got {body}");
}

#[tokio::test]
async fn night_step_endpoints_filter_by_phase() {
    let app = test_app();
    game_with_players(&app, &[("Pat", "Poisoner")]).await;

    // Info steps only exist on the first night; the Poisoner acts on both.
    let first = get_ok(&app, "/game/script/night/first").await;
    let first_names = names_of(&first);
    assert!(first_names.contains(&"Minion Info"));
    assert!(first_names.contains(&"Poisoner"));

    let other = get_ok(&app, "/game/script/night/other").await;
    let other_names = names_of(&other);
    assert!(!other_names.contains(&"Minion Info"));
    assert!(other_names.contains(&"Poisoner"));

    // /game/script/night/steps follows the current phase flag.
    let phase = get_ok(&app, "/game/night/phase").await;
    assert_eq!(phase["is_first_night"], true);
    let steps = get_ok(&app, "/game/script/night/steps").await;
    assert_eq!(names_of(&steps), first_names);

    post_ok(
        &app,
        "/game/night/phase/first_night",
        json!({ "is_first_night": false }),
    )
    .await;
    let steps = get_ok(&app, "/game/script/night/steps").await;
    assert_eq!(names_of(&steps), other_names);
}

#[tokio::test]
async fn script_roles_catalog_is_distinct_from_role_pool() {
    let app = test_app();
    new_game(&app).await;
    add_roles(&app, &["Imp", "Chef"]).await;

    // The catalog lists everything the script defines; the pool only what was added.
    let catalog = get_ok(&app, "/game/script/roles").await;
    assert_eq!(catalog.as_array().unwrap().len(), 22);
    let pool = get_ok(&app, "/characters/list").await;
    assert_eq!(names_of(&pool), ["Imp", "Chef"]);
}

#[tokio::test]
async fn status_effects_catalog_follows_characters_in_play() {
    let app = test_app();
    game_with_players(&app, &[("Pat", "Poisoner"), ("Mo", "Monk")]).await;

    let effects = get_ok(&app, "/game/status_effects").await;
    let pairs: Vec<(&str, &str)> = effects
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["name"].as_str().unwrap(),
                e["character_name"].as_str().unwrap(),
            )
        })
        .collect();
    // Sorted by character name: Monk before Poisoner.
    assert_eq!(pairs, [("Safe", "Monk"), ("Poisoned", "Poisoner")]);
}

#[tokio::test]
async fn dead_vote_flow_updates_state() {
    let app = test_app();
    game_with_players(&app, &[("Alice", "Imp"), ("Bob", "Chef")]).await;

    let names = get_ok(&app, "/players/names").await;
    assert_eq!(names, json!(["Alice", "Bob"]));

    // A fresh death grants a dead vote...
    post_ok(
        &app,
        "/players/set_alive",
        json!({ "name": "Bob", "is_alive": false }),
    )
    .await;
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["dead_players_with_vote"], json!(["Bob"]));

    // ...which disappears once used.
    let body = post_ok(
        &app,
        "/players/set_has_used_dead_vote",
        json!({ "name": "Bob", "has_used_dead_vote": true }),
    )
    .await;
    assert_eq!(body["has_used_dead_vote"], true);
    let state = get_ok(&app, "/game/state").await;
    assert_eq!(state["dead_players_with_vote"], json!([]));

    let (status, _) = post(
        &app,
        "/players/set_has_used_dead_vote",
        json!({ "name": "Ghost", "has_used_dead_vote": true }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn remove_status_effect_is_safe_when_absent() {
    let app = test_app();
    game_with_players(&app, &[("Alice", "Imp")]).await;

    // Removing an effect the player doesn't have succeeds without error.
    let body = post_ok(
        &app,
        "/players/remove_status_effect",
        json!({ "name": "Alice", "status_effect": "Poisoned" }),
    )
    .await;
    assert_eq!(body["status_effects"], json!([]));

    // Add-then-remove round-trips.
    post_ok(
        &app,
        "/players/add_status_effect",
        json!({ "name": "Alice", "status_effect": "Poisoned" }),
    )
    .await;
    let body = post_ok(
        &app,
        "/players/remove_status_effect",
        json!({ "name": "Alice", "status_effect": "Poisoned" }),
    )
    .await;
    assert_eq!(body["status_effects"], json!([]));

    let (status, _) = post(
        &app,
        "/players/remove_status_effect",
        json!({ "name": "Ghost", "status_effect": "Poisoned" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn traveler_duplicate_name_and_claimed_traveler_are_rejected() {
    let app = test_app();
    new_game(&app).await;
    post_ok(
        &app,
        "/players/add_traveler",
        json!({ "name": "Wanderer", "traveler": "Beggar" }),
    )
    .await;

    // Duplicate player name -> 409, even for travelers.
    let (status, _) = post(
        &app,
        "/players/add_traveler",
        json!({ "name": "Wanderer", "traveler": "Thief" }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // A traveler already in play can't be claimed again.
    let (status, _) = post(
        &app,
        "/players/add_traveler",
        json!({ "name": "Drifter", "traveler": "Beggar" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn load_game_switches_between_saved_games() {
    let app = test_app();
    game_with_players(&app, &[("Alice", "Imp")]).await;
    let hist = get_ok(&app, "/game/history").await;
    let first_id = hist["game_id"].as_str().unwrap().to_string();

    // A second game becomes active with its own players.
    game_with_players(&app, &[("Bob", "Chef")]).await;
    let names = get_ok(&app, "/players/names").await;
    assert_eq!(names, json!(["Bob"]));

    // Loading the first game restores its state.
    let body = post_ok(&app, "/game/load", json!({ "game_id": first_id })).await;
    assert_eq!(body["players"][0]["name"], "Alice");

    // Malformed and unknown ids -> 400.
    let (status, _) = post(&app, "/game/load", json!({ "game_id": "not-a-uuid" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let unknown = uuid::Uuid::new_v4().to_string();
    let (status, _) = post(&app, "/game/load", json!({ "game_id": unknown })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn role_reveal_long_poll_unblocks_when_visibility_enabled() {
    let app = test_app();
    game_with_players(&app, &[("Alice", "Imp")]).await;

    // Start the long-poll before roles are revealed; it should wait.
    let poller = {
        let app = app.clone();
        tokio::spawn(async move { get(&app, "/players/name/Alice").await })
    };
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    post_ok(
        &app,
        "/players/set_visibility",
        json!({ "should_reveal_roles": true }),
    )
    .await;

    let (status, body) = poller.await.unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["character"]["name"], "Imp");
}

#[tokio::test]
async fn calibrate_then_spotlight_flow() {
    // Redirect calibration persistence away from the repo's assets directory.
    let file = std::env::temp_dir().join(format!("dd_api_positions_{}.json", std::process::id()));
    std::env::set_var("LIGHTING_POSITIONS_PATH", &file);

    let app = test_app();
    post_ok(
        &app,
        "/lights/calibrate/player/7/save",
        json!({ "pan": 120, "tilt": 80 }),
    )
    .await;

    let body = get_ok(&app, "/lights/calibrate/positions").await;
    assert_eq!(body["positions"]["7"]["pan"], 120);
    assert_eq!(body["positions"]["7"]["tilt"], 80);

    // With a saved position, spotlighting succeeds instead of 404ing.
    post_ok(&app, "/lights/spotlight/player/7", json!({})).await;

    std::env::remove_var("LIGHTING_POSITIONS_PATH");
    let _ = std::fs::remove_file(&file);
}

#[tokio::test]
async fn remove_role_is_case_insensitive_and_removes_one_copy() {
    let app = test_app();
    new_game(&app).await;
    post_ok(&app, "/characters/add", json!({ "name": "Imp" })).await;
    post_ok(&app, "/characters/add", json!({ "name": "Imp" })).await;

    let body = post_ok(&app, "/characters/remove", json!({ "name": "imp" })).await;
    assert_eq!(body["included_roles"], json!(["Imp"]));
    let body = post_ok(&app, "/characters/remove", json!({ "name": "IMP" })).await;
    assert_eq!(body["included_roles"], json!([]));

    let (status, _) = post(&app, "/characters/remove", json!({ "name": "Imp" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
