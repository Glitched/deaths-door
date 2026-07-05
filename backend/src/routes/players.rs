//! Routes for managing players in the game.

use std::time::Duration;

use axum::extract::{Path, State};
use axum::Json;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::alignment::Alignment;
use crate::app::{AppError, AppJson, AppResult, AppState};
use crate::events::EventPayload;
use crate::game_state::{player_state_to_out, GameState, PlayerState};
use crate::lighting::LightingScene;
use crate::player::PlayerOut;

const ROLE_REVEAL_TIMEOUT_ATTEMPTS: u32 = 100;
const POLLING_INTERVAL_MS: u64 = 100;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(add_player))
        .routes(routes!(add_player_as_traveler))
        .routes(routes!(list_players))
        .routes(routes!(get_player_role))
        .routes(routes!(set_player_alive))
        .routes(routes!(set_player_has_used_dead_vote))
        .routes(routes!(set_player_alignment))
        .routes(routes!(swap_character))
        .routes(routes!(rename_player))
        .routes(routes!(remove_player))
        .routes(routes!(get_player_names))
        .routes(routes!(get_roles_visibility))
        .routes(routes!(set_roles_visibility))
        .routes(routes!(add_status_effect))
        .routes(routes!(remove_status_effect))
}

// --- Helpers ---

fn get_player_or_404<'a>(state: &'a GameState, name: &str) -> AppResult<&'a PlayerState> {
    state
        .get_player(name)
        .ok_or_else(|| AppError::not_found(format!("Player not found: {name}")))
}

fn to_out(state: &GameState, name: &str) -> AppResult<PlayerOut> {
    let player = get_player_or_404(state, name)?;
    let script = state
        .get_script()
        .ok_or_else(|| AppError::internal("No script loaded"))?;
    Ok(player_state_to_out(player, script))
}

/// Persistent status effects to clear when a character dies (death cleanup).
fn character_persistent_effects(character_name: &str) -> &'static [&'static str] {
    match character_name {
        "Poisoner" => &["Poisoned"],
        "Monk" => &["Safe"],
        "Butler" => &["Butler's Master"],
        _ => &[],
    }
}

/// Cascading status-effect removals when a player dies.
fn compute_death_cleared_effects(state: &GameState, player_name: &str) -> Vec<(String, String)> {
    let Some(player) = state.get_player(player_name) else {
        return Vec::new();
    };
    let effects_to_remove = character_persistent_effects(&player.character_name);
    if effects_to_remove.is_empty() {
        return Vec::new();
    }
    let mut cleared = Vec::new();
    for p in &state.players {
        for effect in effects_to_remove {
            if p.status_effects.iter().any(|e| e == effect) {
                cleared.push((p.name.clone(), effect.to_string()));
            }
        }
    }
    cleared
}

async fn push_counts(state: &AppState, game: &GameState) {
    let alive = game.living_player_count() as i64;
    let total = game.players.len() as i64;
    state.timer.push_live_activity_update(alive, total).await;
}

// --- Add player ---

#[derive(Deserialize, ToSchema)]
pub struct AddPlayerRequest {
    #[schema(example = "Alice")]
    pub name: String,
}

/// Add a player to the game with a random role from the pool.
#[utoipa::path(
    post, path = "/players/add", tag = "Players",
    request_body = AddPlayerRequest,
    responses(
        (status = 200, description = "Player added with a random role", body = PlayerOut),
        (status = 400, description = "No roles in pool"),
        (status = 409, description = "Duplicate name")
    )
)]
async fn add_player(
    State(state): State<AppState>,
    AppJson(req): AppJson<AddPlayerRequest>,
) -> AppResult<Json<PlayerOut>> {
    let game = state.manager.get_state().await?;

    if game.get_player(&req.name).is_some() {
        return Err(AppError::conflict(format!(
            "Player with name {} already exists.",
            req.name
        )));
    }
    if game.included_role_names.is_empty() {
        return Err(AppError::bad_request("No roles to assign"));
    }

    // Resolve randomness BEFORE creating the event.
    let chosen = game
        .included_role_names
        .choose(&mut rand::thread_rng())
        .cloned()
        .ok_or_else(|| AppError::bad_request("No roles to assign"))?;
    let script = game
        .get_script()
        .ok_or_else(|| AppError::internal("No script loaded"))?;
    let character = script
        .get_character(&chosen)
        .ok_or_else(|| AppError::bad_request(format!("Character not found: {chosen}")))?;

    let new_state = state
        .manager
        .dispatch(EventPayload::PlayerAdded {
            player_name: req.name.clone(),
            character_name: character.name.clone(),
            alignment: character.alignment.as_str().to_string(),
        })
        .await?;
    push_counts(&state, &new_state).await;
    Ok(Json(to_out(&new_state, &req.name)?))
}

#[derive(Deserialize, ToSchema)]
pub struct AddTravelerRequest {
    pub name: String,
    pub traveler: String,
}

/// Add a player to the game as a specific (unclaimed) traveler.
#[utoipa::path(
    post, path = "/players/add_traveler", tag = "Players",
    request_body = AddTravelerRequest,
    responses(
        (status = 200, description = "Traveler player added", body = PlayerOut),
        (status = 404, description = "Traveler not found or already claimed"),
        (status = 409, description = "Duplicate name")
    )
)]
async fn add_player_as_traveler(
    State(state): State<AppState>,
    AppJson(req): AppJson<AddTravelerRequest>,
) -> AppResult<Json<PlayerOut>> {
    let game = state.manager.get_state().await?;

    if game.get_player(&req.name).is_some() {
        return Err(AppError::conflict(format!(
            "Player with name {} already exists.",
            req.name
        )));
    }

    let unclaimed = game.get_unclaimed_travelers();
    let traveler = unclaimed
        .into_iter()
        .find(|t| t.name == req.traveler)
        .ok_or_else(|| {
            AppError::not_found(format!("Traveler not found or in game: {}", req.traveler))
        })?;

    let new_state = state
        .manager
        .dispatch(EventPayload::TravelerAdded {
            player_name: req.name.clone(),
            traveler_name: traveler.name.clone(),
            alignment: traveler.alignment.as_str().to_string(),
        })
        .await?;
    push_counts(&state, &new_state).await;
    Ok(Json(to_out(&new_state, &req.name)?))
}

/// List all players currently in the game.
#[utoipa::path(
    get, path = "/players/list", tag = "Players",
    responses((status = 200, description = "All players", body = [PlayerOut]))
)]
async fn list_players(State(state): State<AppState>) -> AppResult<Json<Vec<PlayerOut>>> {
    let game = state.manager.get_state().await?;
    let players = match game.get_script() {
        Some(script) => game
            .players
            .iter()
            .map(|p| player_state_to_out(p, script))
            .collect(),
        None => Vec::new(),
    };
    Ok(Json(players))
}

/// Get a single player's revealed role.
///
/// Long-polls (up to ~10s) until `/players/set_visibility` enables role reveal,
/// then returns the player's role. Used by player devices to wait for the
/// storyteller to reveal roles.
#[utoipa::path(
    get, path = "/players/name/{name}", tag = "Players",
    params(("name" = String, Path, description = "Player name")),
    responses(
        (status = 200, description = "Player role (waits up to ~10s for reveal to be enabled)", body = PlayerOut),
        (status = 404, description = "Player not found"),
        (status = 408, description = "Role reveal not enabled within timeout")
    )
)]
async fn get_player_role(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> AppResult<Json<PlayerOut>> {
    let game = state.manager.get_state().await?;
    get_player_or_404(&game, &name)?;

    // Poll for role reveal.
    let mut attempts = 0;
    loop {
        let game = state.manager.get_state().await?;
        if game.should_reveal_roles {
            return Ok(Json(to_out(&game, &name)?));
        }
        attempts += 1;
        if attempts >= ROLE_REVEAL_TIMEOUT_ATTEMPTS {
            let timeout_seconds =
                ROLE_REVEAL_TIMEOUT_ATTEMPTS as f64 * (POLLING_INTERVAL_MS as f64 / 1000.0);
            return Err(AppError::timeout(format!(
                "Role reveal timed out after {timeout_seconds}s. Check if roles are set to be revealed."
            )));
        }
        tokio::time::sleep(Duration::from_millis(POLLING_INTERVAL_MS)).await;
    }
}

#[derive(Deserialize, ToSchema)]
pub struct SetPlayerAliveRequest {
    pub name: String,
    pub is_alive: bool,
}

/// Set whether a player is alive (death clears persistent status effects).
#[utoipa::path(
    post, path = "/players/set_alive", tag = "Players",
    request_body = SetPlayerAliveRequest,
    responses(
        (status = 200, description = "Player updated", body = PlayerOut),
        (status = 404, description = "Player not found")
    )
)]
async fn set_player_alive(
    State(state): State<AppState>,
    AppJson(req): AppJson<SetPlayerAliveRequest>,
) -> AppResult<Json<PlayerOut>> {
    let game = state.manager.get_state().await?;
    let player = get_player_or_404(&game, &req.name)?;
    let died = player.is_alive && !req.is_alive;

    let cleared_effects = if died {
        compute_death_cleared_effects(&game, &req.name)
    } else {
        Vec::new()
    };

    let new_state = state
        .manager
        .dispatch(EventPayload::PlayerAliveSet {
            player_name: req.name.clone(),
            is_alive: req.is_alive,
            cleared_effects,
        })
        .await?;
    push_counts(&state, &new_state).await;
    if died && state.effects.auto_effects().death {
        state.effects.trigger(LightingScene::Death, false).await;
    }
    Ok(Json(to_out(&new_state, &req.name)?))
}

#[derive(Deserialize, ToSchema)]
pub struct SetDeadVoteRequest {
    pub name: String,
    pub has_used_dead_vote: bool,
}

/// Set whether a dead player has used their one dead vote.
#[utoipa::path(
    post, path = "/players/set_has_used_dead_vote", tag = "Players",
    request_body = SetDeadVoteRequest,
    responses(
        (status = 200, description = "Player updated", body = PlayerOut),
        (status = 404, description = "Player not found")
    )
)]
async fn set_player_has_used_dead_vote(
    State(state): State<AppState>,
    AppJson(req): AppJson<SetDeadVoteRequest>,
) -> AppResult<Json<PlayerOut>> {
    let game = state.manager.get_state().await?;
    get_player_or_404(&game, &req.name)?;

    let new_state = state
        .manager
        .dispatch(EventPayload::DeadVoteUsedSet {
            player_name: req.name.clone(),
            has_used_dead_vote: req.has_used_dead_vote,
        })
        .await?;
    Ok(Json(to_out(&new_state, &req.name)?))
}

#[derive(Deserialize, ToSchema)]
pub struct SetAlignmentRequest {
    pub name: String,
    pub alignment: Alignment,
}

/// Set a player's alignment (good/evil).
#[utoipa::path(
    post, path = "/players/set_alignment", tag = "Players",
    request_body = SetAlignmentRequest,
    responses(
        (status = 200, description = "Player updated", body = PlayerOut),
        (status = 404, description = "Player not found")
    )
)]
async fn set_player_alignment(
    State(state): State<AppState>,
    AppJson(req): AppJson<SetAlignmentRequest>,
) -> AppResult<Json<PlayerOut>> {
    let game = state.manager.get_state().await?;
    get_player_or_404(&game, &req.name)?;

    let new_state = state
        .manager
        .dispatch(EventPayload::PlayerAlignmentSet {
            player_name: req.name.clone(),
            alignment: req.alignment.as_str().to_string(),
        })
        .await?;
    Ok(Json(to_out(&new_state, &req.name)?))
}

#[derive(Deserialize, ToSchema)]
pub struct SwapCharacterRequest {
    pub name1: String,
    pub name2: String,
}

#[derive(Serialize, ToSchema)]
pub struct SwapCharacterResponse {
    pub status: String,
    pub player1: PlayerOut,
    pub player2: PlayerOut,
}

/// Swap the characters of two players.
#[utoipa::path(
    post, path = "/players/swap_character", tag = "Players",
    request_body = SwapCharacterRequest,
    responses(
        (status = 200, description = "Characters swapped", body = SwapCharacterResponse),
        (status = 404, description = "One or both players not found")
    )
)]
async fn swap_character(
    State(state): State<AppState>,
    AppJson(req): AppJson<SwapCharacterRequest>,
) -> AppResult<Json<SwapCharacterResponse>> {
    let game = state.manager.get_state().await?;
    get_player_or_404(&game, &req.name1)?;
    get_player_or_404(&game, &req.name2)?;

    let new_state = state
        .manager
        .dispatch(EventPayload::CharactersSwapped {
            name1: req.name1.clone(),
            name2: req.name2.clone(),
        })
        .await?;
    Ok(Json(SwapCharacterResponse {
        status: "success".to_string(),
        player1: to_out(&new_state, &req.name1)?,
        player2: to_out(&new_state, &req.name2)?,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct RenamePlayerRequest {
    pub name: String,
    pub new_name: String,
}

/// Rename a player.
#[utoipa::path(
    post, path = "/players/rename", tag = "Players",
    request_body = RenamePlayerRequest,
    responses(
        (status = 200, description = "Player renamed", body = PlayerOut),
        (status = 404, description = "Player not found")
    )
)]
async fn rename_player(
    State(state): State<AppState>,
    AppJson(req): AppJson<RenamePlayerRequest>,
) -> AppResult<Json<PlayerOut>> {
    let game = state.manager.get_state().await?;
    get_player_or_404(&game, &req.name)?;

    let new_state = state
        .manager
        .dispatch(EventPayload::PlayerRenamed {
            old_name: req.name.clone(),
            new_name: req.new_name.clone(),
        })
        .await?;
    Ok(Json(to_out(&new_state, &req.new_name)?))
}

#[derive(Deserialize, ToSchema)]
pub struct RemovePlayerRequest {
    pub name: String,
}

#[derive(Serialize, ToSchema)]
pub struct RemovePlayerResponse {
    pub status: String,
    pub remaining_players: Vec<String>,
}

/// Remove a player from the game.
#[utoipa::path(
    post, path = "/players/remove", tag = "Players",
    request_body = RemovePlayerRequest,
    responses(
        (status = 200, description = "Player removed", body = RemovePlayerResponse),
        (status = 404, description = "Player not found")
    )
)]
async fn remove_player(
    State(state): State<AppState>,
    AppJson(req): AppJson<RemovePlayerRequest>,
) -> AppResult<Json<RemovePlayerResponse>> {
    let game = state.manager.get_state().await?;
    get_player_or_404(&game, &req.name)?;

    let new_state = state
        .manager
        .dispatch(EventPayload::PlayerRemoved {
            player_name: req.name.clone(),
        })
        .await?;
    push_counts(&state, &new_state).await;
    Ok(Json(RemovePlayerResponse {
        status: "success".to_string(),
        remaining_players: new_state.players.iter().map(|p| p.name.clone()).collect(),
    }))
}

/// List the names of all players in the game.
#[utoipa::path(
    get, path = "/players/names", tag = "Players",
    responses((status = 200, description = "Player names", body = [String]))
)]
async fn get_player_names(State(state): State<AppState>) -> AppResult<Json<Vec<String>>> {
    let game = state.manager.get_state().await?;
    Ok(Json(game.players.iter().map(|p| p.name.clone()).collect()))
}

/// Get whether roles are currently revealed to players.
#[utoipa::path(
    get, path = "/players/visibility", tag = "Players",
    responses((status = 200, description = "Whether roles are revealed", body = bool))
)]
async fn get_roles_visibility(State(state): State<AppState>) -> AppResult<Json<bool>> {
    let game = state.manager.get_state().await?;
    Ok(Json(game.should_reveal_roles))
}

#[derive(Deserialize, ToSchema)]
pub struct SetVisibilityRequest {
    pub should_reveal_roles: bool,
}

/// Set whether roles are revealed to players (unblocks `/players/name/{name}`).
#[utoipa::path(
    post, path = "/players/set_visibility", tag = "Players",
    request_body = SetVisibilityRequest,
    responses((status = 200, description = "New visibility state", body = bool))
)]
async fn set_roles_visibility(
    State(state): State<AppState>,
    AppJson(req): AppJson<SetVisibilityRequest>,
) -> AppResult<Json<bool>> {
    let new_state = state
        .manager
        .dispatch(EventPayload::RoleVisibilitySet {
            should_reveal_roles: req.should_reveal_roles,
        })
        .await?;
    Ok(Json(new_state.should_reveal_roles))
}

#[derive(Deserialize, ToSchema)]
pub struct StatusEffectRequest {
    pub name: String,
    pub status_effect: String,
}

/// Add a status effect to a player.
#[utoipa::path(
    post, path = "/players/add_status_effect", tag = "Players",
    request_body = StatusEffectRequest,
    responses(
        (status = 200, description = "Player updated", body = PlayerOut),
        (status = 404, description = "Player not found")
    )
)]
async fn add_status_effect(
    State(state): State<AppState>,
    AppJson(req): AppJson<StatusEffectRequest>,
) -> AppResult<Json<PlayerOut>> {
    let game = state.manager.get_state().await?;
    get_player_or_404(&game, &req.name)?;

    let new_state = state
        .manager
        .dispatch(EventPayload::StatusEffectAdded {
            player_name: req.name.clone(),
            effect: req.status_effect.clone(),
        })
        .await?;
    Ok(Json(to_out(&new_state, &req.name)?))
}

/// Remove a status effect from a player (safe if effect is absent).
#[utoipa::path(
    post, path = "/players/remove_status_effect", tag = "Players",
    request_body = StatusEffectRequest,
    responses(
        (status = 200, description = "Player updated", body = PlayerOut),
        (status = 404, description = "Player not found")
    )
)]
async fn remove_status_effect(
    State(state): State<AppState>,
    AppJson(req): AppJson<StatusEffectRequest>,
) -> AppResult<Json<PlayerOut>> {
    let game = state.manager.get_state().await?;
    get_player_or_404(&game, &req.name)?;

    let new_state = state
        .manager
        .dispatch(EventPayload::StatusEffectRemoved {
            player_name: req.name.clone(),
            effect: req.status_effect.clone(),
        })
        .await?;
    Ok(Json(to_out(&new_state, &req.name)?))
}
