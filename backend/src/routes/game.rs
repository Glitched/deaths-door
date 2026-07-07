//! Routes for game management, night phases, and consolidated state.

use std::convert::Infallible;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::app::{AppError, AppJson, AppResult, AppState};
use crate::character::CharacterOut;
use crate::effects::ActiveEffect;
use crate::events::{describe_event, EventPayload};
use crate::game_state::{
    compute_death_cleared_effects, game_state_to_included_role_outs, player_state_to_out,
    ChoppingBlock, GameState, Nomination, NominationOutcome, Phase,
};
use crate::lighting::LightingScene;
use crate::night_step::NightStep;
use crate::player::PlayerOut;
use crate::routes::timer::TimerStateResponse;
use crate::script_name::ScriptName;
use crate::status_effect::StatusEffectOut;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(new_game))
        .routes(routes!(get_game_script))
        .routes(routes!(get_game_script_roles))
        .routes(routes!(get_first_night_steps))
        .routes(routes!(get_other_night_steps))
        .routes(routes!(get_night_steps))
        .routes(routes!(get_status_effects))
        .routes(routes!(get_game_state))
        .routes(routes!(get_night_phase))
        .routes(routes!(set_night_step))
        .routes(routes!(set_first_night))
        .routes(routes!(set_demon_bluffs))
        .routes(routes!(set_chopping_block))
        .routes(routes!(clear_chopping_block))
        .routes(routes!(begin_day))
        .routes(routes!(announce_death))
        .routes(routes!(record_nomination))
        .routes(routes!(end_day))
        .routes(routes!(begin_night))
        .routes(routes!(get_game_history))
        .routes(routes!(rewind_game))
        .routes(routes!(fork_game))
        .routes(routes!(load_game))
        .routes(routes!(list_games))
        .routes(routes!(stream_game_state))
}

// --- Shared state serialization ---

/// The full game-state snapshot pushed over SSE and embedded in `/game/state`.
#[derive(Serialize, ToSchema)]
pub struct GameStateBase {
    pub script_name: String,
    pub players: Vec<PlayerOut>,
    /// Current night-step bookmark, e.g. "Dusk" or "Imp".
    pub current_night_step: String,
    pub is_first_night: bool,
    pub should_reveal_roles: bool,
    pub status_effects: Vec<StatusEffectOut>,
    /// Roles in the pool not yet assigned to a player.
    pub included_roles: Vec<CharacterOut>,
    /// The Demon's bluffs (good characters shown as "not in play"), resolved to
    /// full character objects. Up to 3; empty if none set. Set via `POST /game/bluffs`.
    pub demon_bluffs: Vec<CharacterOut>,
    /// The player currently on the chopping block, if any. Set via
    /// `POST /game/chopping_block`; cleared automatically when that player dies
    /// or is removed, or when night begins.
    pub chopping_block: Option<ChoppingBlock>,
    /// Night steps for the current phase, filtered to roles in play.
    pub night_steps: Vec<NightStep>,
    pub living_player_count: usize,
    /// Votes needed to execute (>= 50% of living players).
    pub execution_threshold: usize,
    pub dead_players_with_vote: Vec<String>,
    /// Whether it is day or night — the app's day/night mode switch. Driven by
    /// `POST /game/day/begin` / `POST /game/night/begin` (and kept coherent
    /// with the night-step bookmark: "Dawn" = day).
    pub phase: Phase,
    /// How many days have begun (0 during setup and the first night).
    pub day_number: u32,
    /// Night deaths not yet announced. Announce each via
    /// `POST /game/day/announce_death` (which also fires the death scene).
    pub deaths_to_announce: Vec<String>,
    /// Nominations resolved today via `POST /game/nominations`. Cleared when
    /// night begins; players listed here can't be nominated again today.
    pub nominations_today: Vec<Nomination>,
    /// Everyone currently allowed to vote: living players plus dead players
    /// holding their one dead vote.
    pub eligible_voters: Vec<String>,
}

pub fn game_state_base(state: &GameState) -> GameStateBase {
    let players = match state.get_script() {
        Some(script) => state
            .players
            .iter()
            .map(|p| player_state_to_out(p, script))
            .collect(),
        None => Vec::new(),
    };
    GameStateBase {
        script_name: state.script_name.clone(),
        players,
        current_night_step: state.current_night_step.clone(),
        is_first_night: state.is_first_night,
        should_reveal_roles: state.should_reveal_roles,
        status_effects: state.get_status_effects(),
        included_roles: game_state_to_included_role_outs(state),
        demon_bluffs: state
            .get_demon_bluffs()
            .iter()
            .map(|c| c.to_out())
            .collect(),
        chopping_block: state.chopping_block.clone(),
        night_steps: state.get_night_steps(),
        living_player_count: state.living_player_count(),
        execution_threshold: state.execution_threshold(),
        dead_players_with_vote: state.get_dead_players_with_vote(),
        phase: state.phase,
        day_number: state.day_number,
        deaths_to_announce: state.deaths_to_announce.clone(),
        nominations_today: state.nominations_today.clone(),
        eligible_voters: state.eligible_voters(),
    }
}

/// Full `/game/state` response: the snapshot plus ephemeral live state (timer,
/// playing scene effect).
#[derive(Serialize, ToSchema)]
pub struct GameStateResponse {
    #[serde(flatten)]
    pub base: GameStateBase,
    pub timer: TimerStateResponse,
    /// Scene effect currently playing (lights/sound), if any. The overlay runs
    /// a matching visual whenever the effect's `id` changes.
    pub active_effect: Option<ActiveEffect>,
}

pub async fn build_game_state_response(state: &AppState) -> AppResult<GameStateResponse> {
    let game = state.manager.get_state().await?;
    let timer = TimerStateResponse {
        is_running: state.timer.get_is_running().await,
        seconds: state.timer.get_seconds().await,
    };
    Ok(GameStateResponse {
        base: game_state_base(&game),
        timer,
        active_effect: state.effects.active(),
    })
}

/// Build one SSE frame: the full game state plus ephemeral live state (same
/// shape as `GET /game/state`). The timer and active effect are read live so
/// broadcasts carry the current countdown / playing scene.
async fn sse_frame(game: &GameState, app: &AppState) -> String {
    let timer = TimerStateResponse {
        is_running: app.timer.get_is_running().await,
        seconds: app.timer.get_seconds().await,
    };
    let payload = GameStateResponse {
        base: game_state_base(game),
        timer,
        active_effect: app.effects.active(),
    };
    serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
}

// --- New game ---

#[derive(Deserialize, ToSchema)]
pub struct NewGameRequest {
    /// Script/edition id, e.g. `trouble_brewing`.
    #[schema(example = "trouble_brewing")]
    pub script_name: String,
}

#[derive(Serialize, ToSchema)]
pub struct NewGameResponse {
    pub status: String,
    pub script_name: String,
}

/// Start a new game with the given script.
///
/// The new game starts with **no roles** in the pool — add them via
/// `POST /characters/add/multi` before adding players.
#[utoipa::path(
    post, path = "/game/new", tag = "Game",
    request_body = NewGameRequest,
    responses(
        (status = 200, description = "Game created", body = NewGameResponse),
        (status = 404, description = "Unknown script")
    )
)]
async fn new_game(
    State(state): State<AppState>,
    AppJson(req): AppJson<NewGameRequest>,
) -> AppResult<Json<NewGameResponse>> {
    let script_name = ScriptName::from_str(&req.script_name)
        .ok_or_else(|| AppError::not_found("Script not found"))?;
    state.manager.create_game(script_name.value()).await?;
    Ok(Json(NewGameResponse {
        status: "success".to_string(),
        script_name: script_name.value().to_string(),
    }))
}

/// The current game's script id.
#[utoipa::path(get, path = "/game/script/name", tag = "Game", responses((status = 200, body = String)))]
async fn get_game_script(State(state): State<AppState>) -> AppResult<Json<String>> {
    let game = state.manager.get_state().await?;
    Ok(Json(game.script_name))
}

/// All character roles defined by the current script (the full catalog, not the
/// game's pool — for that, use `GET /characters/list`).
#[utoipa::path(get, path = "/game/script/roles", tag = "Game", responses((status = 200, body = [CharacterOut])))]
async fn get_game_script_roles(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<CharacterOut>>> {
    let game = state.manager.get_state().await?;
    let roles = match game.get_script() {
        Some(script) => script.characters.iter().map(|c| c.to_out()).collect(),
        None => Vec::new(),
    };
    Ok(Json(roles))
}

/// First-night steps for the current game (filtered to roles in play).
#[utoipa::path(get, path = "/game/script/night/first", tag = "Game", responses((status = 200, body = [NightStep])))]
async fn get_first_night_steps(State(state): State<AppState>) -> AppResult<Json<Vec<NightStep>>> {
    let mut game = state.manager.get_state().await?;
    game.is_first_night = true;
    Ok(Json(game.get_night_steps()))
}

/// Subsequent-night steps for the current game (filtered to roles in play).
#[utoipa::path(get, path = "/game/script/night/other", tag = "Game", responses((status = 200, body = [NightStep])))]
async fn get_other_night_steps(State(state): State<AppState>) -> AppResult<Json<Vec<NightStep>>> {
    let mut game = state.manager.get_state().await?;
    game.is_first_night = false;
    Ok(Json(game.get_night_steps()))
}

/// Night steps for the current phase (first vs. other, based on game state).
#[utoipa::path(get, path = "/game/script/night/steps", tag = "Game", responses((status = 200, body = [NightStep])))]
async fn get_night_steps(State(state): State<AppState>) -> AppResult<Json<Vec<NightStep>>> {
    let game = state.manager.get_state().await?;
    Ok(Json(game.get_night_steps()))
}

/// All status effects currently introduced by characters in the game.
#[utoipa::path(get, path = "/game/status_effects", tag = "Game", responses((status = 200, body = [StatusEffectOut])))]
async fn get_status_effects(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<StatusEffectOut>>> {
    let game = state.manager.get_state().await?;
    Ok(Json(game.get_status_effects()))
}

/// The complete game state in one request (players, night phase, vote info,
/// status effects, and timer). For live updates, prefer the SSE `/game/stream`.
#[utoipa::path(get, path = "/game/state", tag = "Game", responses((status = 200, body = GameStateResponse)))]
async fn get_game_state(State(state): State<AppState>) -> AppResult<Json<GameStateResponse>> {
    Ok(Json(build_game_state_response(&state).await?))
}

// --- Night phase ---

/// Fire the goodnight/morning auto-effects when the night-step bookmark
/// crosses a day/night boundary. Day is when the bookmark sits on "Dawn"
/// (mirroring the chopping-block auto-clear rule); moving between two night
/// steps re-fires nothing.
async fn maybe_phase_auto_effects(state: &AppState, old_step: &str, new_step: &str) {
    let auto = state.effects.auto_effects();
    let was_day = old_step == "Dawn";
    let is_day = new_step == "Dawn";
    if was_day && !is_day && auto.goodnight {
        state.effects.trigger(LightingScene::Goodnight, false).await;
    } else if !was_day && is_day && auto.morning {
        state.effects.trigger(LightingScene::Morning, false).await;
    }
}

#[derive(Serialize, ToSchema)]
pub struct NightPhaseResponse {
    pub current_night_step: String,
    pub is_first_night: bool,
}

/// The current night phase (step bookmark + first-night flag).
#[utoipa::path(get, path = "/game/night/phase", tag = "Game", responses((status = 200, body = NightPhaseResponse)))]
async fn get_night_phase(State(state): State<AppState>) -> AppResult<Json<NightPhaseResponse>> {
    let game = state.manager.get_state().await?;
    Ok(Json(NightPhaseResponse {
        current_night_step: game.current_night_step,
        is_first_night: game.is_first_night,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct SetNightStepRequest {
    /// Night-step name to bookmark, e.g. "Poisoner".
    #[schema(example = "Poisoner")]
    pub step: String,
}

/// Set the current night-step bookmark.
#[utoipa::path(
    post, path = "/game/night/phase/step", tag = "Game",
    request_body = SetNightStepRequest,
    responses((status = 200, body = NightPhaseResponse))
)]
async fn set_night_step(
    State(state): State<AppState>,
    AppJson(req): AppJson<SetNightStepRequest>,
) -> AppResult<Json<NightPhaseResponse>> {
    let old_step = state.manager.get_state().await?.current_night_step;
    let new_state = state
        .manager
        .dispatch(EventPayload::NightStepSet { step: req.step })
        .await?;
    maybe_phase_auto_effects(&state, &old_step, &new_state.current_night_step).await;
    Ok(Json(NightPhaseResponse {
        current_night_step: new_state.current_night_step,
        is_first_night: new_state.is_first_night,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct SetFirstNightRequest {
    pub is_first_night: bool,
}

/// Toggle the first-night flag (resets the step bookmark to "Dusk").
#[utoipa::path(
    post, path = "/game/night/phase/first_night", tag = "Game",
    request_body = SetFirstNightRequest,
    responses((status = 200, body = NightPhaseResponse))
)]
async fn set_first_night(
    State(state): State<AppState>,
    AppJson(req): AppJson<SetFirstNightRequest>,
) -> AppResult<Json<NightPhaseResponse>> {
    let old_step = state.manager.get_state().await?.current_night_step;
    let new_state = state
        .manager
        .dispatch(EventPayload::FirstNightSet {
            is_first_night: req.is_first_night,
        })
        .await?;
    // Toggling the flag resets the bookmark to Dusk, which can cross into night.
    maybe_phase_auto_effects(&state, &old_step, &new_state.current_night_step).await;
    Ok(Json(NightPhaseResponse {
        current_night_step: new_state.current_night_step,
        is_first_night: new_state.is_first_night,
    }))
}

// --- Demon bluffs ---

#[derive(Deserialize, ToSchema)]
pub struct SetBluffsRequest {
    /// Up to 3 character names to record as the Demon's bluffs (good characters
    /// shown as "not in play"). Names are matched case-insensitively against the
    /// current script. Pass an empty list to clear the bluffs.
    #[schema(example = json!(["Mayor", "Slayer", "Empath"]))]
    pub bluffs: Vec<String>,
}

/// Record the Demon's bluffs (the up-to-3 characters shown as "not in play").
///
/// Each name must be a real character in the current script (matched
/// case-insensitively and stored canonically); unknown roles 404 and more than
/// 3 bluffs 400. The bluffs are persisted as an event and surface, resolved to
/// full character objects, in every state snapshot — including the SSE stream.
#[utoipa::path(
    post, path = "/game/bluffs", tag = "Game",
    request_body = SetBluffsRequest,
    responses(
        (status = 200, description = "Bluffs set; full game state returned", body = GameStateResponse),
        (status = 400, description = "More than 3 bluffs given"),
        (status = 404, description = "A named character is not in the current script")
    )
)]
async fn set_demon_bluffs(
    State(state): State<AppState>,
    AppJson(req): AppJson<SetBluffsRequest>,
) -> AppResult<Json<GameStateResponse>> {
    if req.bluffs.len() > 3 {
        return Err(AppError::bad_request(format!(
            "At most 3 demon bluffs allowed, got {}",
            req.bluffs.len()
        )));
    }
    // Resolve each name against the current script and store the canonical
    // character name; reject unknown roles so typos surface as a 404.
    let game = state.manager.get_state().await?;
    let script = game
        .get_script()
        .ok_or_else(|| AppError::not_found("No script loaded for the current game"))?;
    let mut canonical = Vec::with_capacity(req.bluffs.len());
    for name in &req.bluffs {
        let character = script
            .get_character(name)
            .ok_or_else(|| AppError::not_found(format!("Role '{name}' not found in script")))?;
        canonical.push(character.name.clone());
    }
    state
        .manager
        .dispatch(EventPayload::DemonBluffsSet { bluffs: canonical })
        .await?;
    Ok(Json(build_game_state_response(&state).await?))
}

// --- Chopping block ---

#[derive(Deserialize, ToSchema)]
pub struct SetChoppingBlockRequest {
    /// Name of the player to put on the chopping block. Must be a living player
    /// in the current game.
    #[schema(example = "Alice")]
    pub player_name: String,
    /// Vote count that put the player on the block. Optional — record it if you
    /// want it shown on the overlay.
    #[schema(example = 4)]
    pub votes: Option<u32>,
}

/// Put a player on the chopping block (up for execution).
///
/// Replaces any player already on the block. The block clears automatically
/// when the player on it dies or is removed, or when night begins; it can also
/// be cleared explicitly via `POST /game/chopping_block/clear`.
#[utoipa::path(
    post, path = "/game/chopping_block", tag = "Game",
    request_body = SetChoppingBlockRequest,
    responses(
        (status = 200, description = "Chopping block set; full game state returned", body = GameStateResponse),
        (status = 400, description = "Player is dead"),
        (status = 404, description = "Player not found")
    )
)]
async fn set_chopping_block(
    State(state): State<AppState>,
    AppJson(req): AppJson<SetChoppingBlockRequest>,
) -> AppResult<Json<GameStateResponse>> {
    let game = state.manager.get_state().await?;
    let player = game
        .get_player(&req.player_name)
        .ok_or_else(|| AppError::not_found(format!("Player not found: {}", req.player_name)))?;
    if !player.is_alive {
        return Err(AppError::bad_request(format!(
            "{} is dead and cannot be executed",
            player.name
        )));
    }
    state
        .manager
        .dispatch(EventPayload::ChoppingBlockSet {
            player_name: player.name.clone(),
            votes: req.votes,
        })
        .await?;
    if state.effects.auto_effects().nomination {
        state.effects.trigger(LightingScene::Drama, false).await;
    }
    Ok(Json(build_game_state_response(&state).await?))
}

/// Clear the chopping block (e.g. a later nomination tied the vote count).
///
/// Safe to call when nothing is on the block — no event is recorded.
#[utoipa::path(
    post, path = "/game/chopping_block/clear", tag = "Game",
    responses((status = 200, description = "Chopping block cleared; full game state returned", body = GameStateResponse))
)]
async fn clear_chopping_block(State(state): State<AppState>) -> AppResult<Json<GameStateResponse>> {
    let game = state.manager.get_state().await?;
    if game.chopping_block.is_some() {
        state
            .manager
            .dispatch(EventPayload::ChoppingBlockCleared)
            .await?;
    }
    Ok(Json(build_game_state_response(&state).await?))
}

// --- Day/night cycle and nominations ---

/// Begin the day: dawn breaks and the game enters day mode.
///
/// Deaths that happened overnight are waiting in `deaths_to_announce`;
/// announce them via `POST /game/day/announce_death`, run nominations via
/// `POST /game/nominations`, then close out with `POST /game/day/end` and
/// `POST /game/night/begin`.
#[utoipa::path(
    post, path = "/game/day/begin", tag = "Game",
    responses(
        (status = 200, description = "Day begun; full game state returned", body = GameStateResponse),
        (status = 400, description = "It is already daytime")
    )
)]
async fn begin_day(State(state): State<AppState>) -> AppResult<Json<GameStateResponse>> {
    let game = state.manager.get_state().await?;
    if game.is_day() {
        return Err(AppError::bad_request("It is already daytime"));
    }
    let old_step = game.current_night_step.clone();
    let new_state = state.manager.dispatch(EventPayload::DayBegan).await?;
    maybe_phase_auto_effects(&state, &old_step, &new_state.current_night_step).await;
    Ok(Json(build_game_state_response(&state).await?))
}

/// Begin the night: dusk falls and the game enters night mode.
///
/// Clears the day's nominations and anything left on the chopping block (end
/// the day first via `POST /game/day/end` if an execution should happen).
/// A night begun after any day is not the first night.
#[utoipa::path(
    post, path = "/game/night/begin", tag = "Game",
    responses(
        (status = 200, description = "Night begun; full game state returned", body = GameStateResponse),
        (status = 400, description = "It is already night")
    )
)]
async fn begin_night(State(state): State<AppState>) -> AppResult<Json<GameStateResponse>> {
    let game = state.manager.get_state().await?;
    if !game.is_day() {
        return Err(AppError::bad_request("It is already night"));
    }
    let old_step = game.current_night_step.clone();
    let new_state = state.manager.dispatch(EventPayload::NightBegan).await?;
    maybe_phase_auto_effects(&state, &old_step, &new_state.current_night_step).await;
    Ok(Json(build_game_state_response(&state).await?))
}

#[derive(Deserialize, ToSchema)]
pub struct AnnounceDeathRequest {
    /// A player from `deaths_to_announce`.
    #[schema(example = "Alice")]
    pub player_name: String,
}

/// Announce an overnight death: checks the player off `deaths_to_announce`
/// and fires the death scene (sound + lights + overlay).
#[utoipa::path(
    post, path = "/game/day/announce_death", tag = "Game",
    request_body = AnnounceDeathRequest,
    responses(
        (status = 200, description = "Death announced; full game state returned", body = GameStateResponse),
        (status = 400, description = "It is not daytime"),
        (status = 404, description = "Player is not awaiting announcement")
    )
)]
async fn announce_death(
    State(state): State<AppState>,
    AppJson(req): AppJson<AnnounceDeathRequest>,
) -> AppResult<Json<GameStateResponse>> {
    let game = state.manager.get_state().await?;
    if !game.is_day() {
        return Err(AppError::bad_request("Deaths are announced during the day"));
    }
    if !game.deaths_to_announce.contains(&req.player_name) {
        return Err(AppError::not_found(format!(
            "{} is not awaiting announcement",
            req.player_name
        )));
    }
    state
        .manager
        .dispatch(EventPayload::DeathAnnounced {
            player_name: req.player_name.clone(),
        })
        .await?;
    // The announcement is the dramatic beat the night death skipped.
    state.effects.trigger(LightingScene::Death, false).await;
    Ok(Json(build_game_state_response(&state).await?))
}

#[derive(Deserialize, ToSchema)]
pub struct NominationRequest {
    /// The nominated player. Must be alive and not yet nominated today.
    #[schema(example = "Alice")]
    pub player_name: String,
    /// Every player who voted. Each must be in `eligible_voters`; dead voters
    /// automatically spend their one dead vote.
    #[schema(example = json!(["Bob", "Carol"]))]
    pub voters: Vec<String>,
    /// Counted vote total, if it differs from the number of voters (e.g. a
    /// Bureaucrat's vote counts three times). Defaults to `voters.len()`.
    pub votes: Option<u32>,
}

#[derive(Serialize, ToSchema)]
pub struct NominationResponse {
    /// What the vote did to the chopping block.
    pub outcome: NominationOutcome,
    #[serde(flatten)]
    pub state: GameStateResponse,
}

/// Confirm a nomination's vote and resolve the chopping block.
///
/// Meeting the execution threshold AND beating the current block's votes puts
/// the nominee on the block; exactly tying the current block empties it (a
/// tie means no one is executed); fewer votes change nothing. Dead voters'
/// dead votes are spent automatically, and the nomination is recorded in
/// `nominations_today` (each player can be nominated once per day).
#[utoipa::path(
    post, path = "/game/nominations", tag = "Game",
    request_body = NominationRequest,
    responses(
        (status = 200, description = "Vote resolved; outcome plus full game state", body = NominationResponse),
        (status = 400, description = "Not daytime, nominee dead or already nominated, or a voter is ineligible/duplicated"),
        (status = 404, description = "Nominee or voter not found")
    )
)]
async fn record_nomination(
    State(state): State<AppState>,
    AppJson(req): AppJson<NominationRequest>,
) -> AppResult<Json<NominationResponse>> {
    let game = state.manager.get_state().await?;
    if !game.is_day() {
        return Err(AppError::bad_request("Nominations happen during the day"));
    }
    let nominee = game
        .get_player(&req.player_name)
        .ok_or_else(|| AppError::not_found(format!("Player not found: {}", req.player_name)))?;
    if !nominee.is_alive {
        return Err(AppError::bad_request(format!(
            "{} is dead and cannot be nominated",
            nominee.name
        )));
    }
    if game.was_nominated_today(&req.player_name) {
        return Err(AppError::bad_request(format!(
            "{} has already been nominated today",
            nominee.name
        )));
    }
    let eligible = game.eligible_voters();
    let mut seen = std::collections::HashSet::new();
    for voter in &req.voters {
        game.get_player(voter)
            .ok_or_else(|| AppError::not_found(format!("Voter not found: {voter}")))?;
        if !seen.insert(voter.as_str()) {
            return Err(AppError::bad_request(format!("Duplicate voter: {voter}")));
        }
        if !eligible.contains(voter) {
            return Err(AppError::bad_request(format!(
                "{voter} has no vote (dead vote already spent)"
            )));
        }
    }
    let votes = req.votes.unwrap_or(req.voters.len() as u32);

    let new_state = state
        .manager
        .dispatch(EventPayload::NominationRecorded {
            player_name: req.player_name.clone(),
            voters: req.voters.clone(),
            votes,
        })
        .await?;
    let outcome = new_state
        .nominations_today
        .last()
        .map(|n| n.outcome)
        .unwrap_or(NominationOutcome::BlockUnchanged);
    if outcome == NominationOutcome::OnTheBlock && state.effects.auto_effects().nomination {
        state.effects.trigger(LightingScene::Drama, false).await;
    }
    Ok(Json(NominationResponse {
        outcome,
        state: build_game_state_response(&state).await?,
    }))
}

#[derive(Serialize, ToSchema)]
pub struct EndDayResponse {
    /// The player who was executed (whoever was on the chopping block), if any.
    pub executed: Option<String>,
    #[serde(flatten)]
    pub state: GameStateResponse,
}

/// End the day: whoever is on the chopping block is executed.
///
/// Safe to call with an empty block (nobody is executed). The game stays in
/// day mode for last words — press `POST /game/night/begin` to move on.
#[utoipa::path(
    post, path = "/game/day/end", tag = "Game",
    responses(
        (status = 200, description = "Day ended; executed player (if any) plus full game state", body = EndDayResponse),
        (status = 400, description = "It is not daytime")
    )
)]
async fn end_day(State(state): State<AppState>) -> AppResult<Json<EndDayResponse>> {
    let game = state.manager.get_state().await?;
    if !game.is_day() {
        return Err(AppError::bad_request("The day has not begun"));
    }
    let executed = game.chopping_block.as_ref().map(|b| b.player_name.clone());
    if let Some(name) = &executed {
        let cleared_effects = compute_death_cleared_effects(&game, name);
        let new_state = state
            .manager
            .dispatch(EventPayload::PlayerExecuted {
                player_name: name.clone(),
                cleared_effects,
            })
            .await?;
        state
            .timer
            .push_live_activity_update(
                new_state.living_player_count() as i64,
                new_state.players.len() as i64,
            )
            .await;
        // An execution is a public daytime death.
        if state.effects.auto_effects().death {
            state.effects.trigger(LightingScene::Death, false).await;
        }
    }
    Ok(Json(EndDayResponse {
        executed,
        state: build_game_state_response(&state).await?,
    }))
}

// --- Event sourcing endpoints ---

#[derive(Serialize, ToSchema)]
pub struct EventOut {
    /// 1-indexed version produced by this event (use with rewind/fork).
    pub version: i64,
    pub description: String,
    pub event_type: String,
    pub timestamp: String,
    /// Event-specific payload (fields vary by `event_type`).
    #[schema(value_type = Object)]
    pub payload: serde_json::Value,
}

#[derive(Serialize, ToSchema)]
pub struct HistoryResponse {
    pub game_id: String,
    pub version: i64,
    pub events: Vec<EventOut>,
}

/// Full event log for the current game, each with a human-readable description
/// and the version it produced. Use those versions with rewind/fork.
#[utoipa::path(get, path = "/game/history", tag = "Game", responses((status = 200, body = HistoryResponse)))]
async fn get_game_history(State(state): State<AppState>) -> AppResult<Json<HistoryResponse>> {
    let game = state.manager.get_state().await?;
    let events = state.manager.get_history().await?;
    let event_outs = events
        .into_iter()
        .map(|e| {
            // Full payload minus the "type" discriminator (matches Python).
            let mut payload = serde_json::to_value(&e.payload).unwrap_or(serde_json::Value::Null);
            if let Some(obj) = payload.as_object_mut() {
                obj.remove("type");
            }
            EventOut {
                version: e.sequence + 1,
                description: describe_event(&e.payload),
                event_type: e.payload.event_type().to_string(),
                timestamp: e.timestamp.to_rfc3339(),
                payload,
            }
        })
        .collect();
    Ok(Json(HistoryResponse {
        game_id: game.game_id.to_string(),
        version: game.version,
        events: event_outs,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct RewindRequest {
    /// Version to rewind to (1-based, inclusive).
    #[schema(example = 5)]
    pub to_version: i64,
}

/// Rewind the current game to a previous version (destructive — deletes later
/// events). Use `POST /game/fork` first to preserve the original timeline.
#[utoipa::path(
    post, path = "/game/rewind", tag = "Game",
    request_body = RewindRequest,
    responses(
        (status = 200, description = "Rewound state", body = GameStateResponse),
        (status = 400, description = "Version out of range (must be 1..=current)")
    )
)]
async fn rewind_game(
    State(state): State<AppState>,
    AppJson(req): AppJson<RewindRequest>,
) -> AppResult<Json<GameStateResponse>> {
    state.manager.rewind(req.to_version).await?;
    Ok(Json(build_game_state_response(&state).await?))
}

#[derive(Deserialize, ToSchema)]
pub struct ForkRequest {
    /// Version to fork from (1-based, inclusive).
    #[schema(example = 3)]
    pub from_version: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ForkResponse {
    pub new_game_id: String,
    pub version: i64,
}

/// Fork the current game from a version into a new independent game (the
/// original is untouched). The forked game becomes active.
#[utoipa::path(
    post, path = "/game/fork", tag = "Game",
    request_body = ForkRequest,
    responses(
        (status = 200, description = "Forked game", body = ForkResponse),
        (status = 400, description = "Version out of range")
    )
)]
async fn fork_game(
    State(state): State<AppState>,
    AppJson(req): AppJson<ForkRequest>,
) -> AppResult<Json<ForkResponse>> {
    let new_state = state.manager.fork(req.from_version).await?;
    Ok(Json(ForkResponse {
        new_game_id: new_state.game_id.to_string(),
        version: new_state.version,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct LoadGameRequest {
    /// UUID of the game to load (from `GET /game/list`).
    pub game_id: String,
}

/// Load a previously saved game by id; it becomes the active game.
#[utoipa::path(
    post, path = "/game/load", tag = "Game",
    request_body = LoadGameRequest,
    responses(
        (status = 200, description = "Loaded state", body = GameStateResponse),
        (status = 400, description = "Invalid or unknown game id")
    )
)]
async fn load_game(
    State(state): State<AppState>,
    AppJson(req): AppJson<LoadGameRequest>,
) -> AppResult<Json<GameStateResponse>> {
    let game_id = Uuid::parse_str(&req.game_id)
        .map_err(|_| AppError::bad_request(format!("Invalid game ID: {}", req.game_id)))?;
    state.manager.load_game(game_id).await?;
    Ok(Json(build_game_state_response(&state).await?))
}

#[derive(Serialize, ToSchema)]
pub struct GameListResponse {
    pub game_ids: Vec<String>,
}

/// List all saved game ids. Use with `POST /game/load` to switch games.
#[utoipa::path(get, path = "/game/list", tag = "Game", responses((status = 200, body = GameListResponse)))]
async fn list_games(State(state): State<AppState>) -> AppResult<Json<GameListResponse>> {
    let ids = state.manager.list_games().await?;
    Ok(Json(GameListResponse {
        game_ids: ids.iter().map(|g| g.to_string()).collect(),
    }))
}

/// Stream game-state changes via Server-Sent Events.
///
/// Emits the full game state (the `GameStateResponse` minus `timer`) as a JSON
/// object in each `data:` frame — once immediately on connect, then again after
/// every mutation. Prefer this over polling `GET /game/state`.
#[utoipa::path(
    get, path = "/game/stream", tag = "Game",
    responses((
        status = 200,
        description = "SSE stream; each event's data is a game-state JSON object",
        content_type = "text/event-stream",
        body = GameStateBase
    ))
)]
async fn stream_game_state(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.manager.subscribe();

    // Full state (with timer) immediately on connect.
    let initial_event = match state.manager.get_state().await.ok() {
        Some(ref game) => Some(Ok(Event::default().data(sse_frame(game, &state).await))),
        None => None,
    };

    // Then a frame on every game mutation and every timer change.
    let app = state.clone();
    let updates = BroadcastStream::new(rx)
        .filter_map(|res| res.ok())
        .then(move |game| {
            let app = app.clone();
            async move { Ok(Event::default().data(sse_frame(&game, &app).await)) }
        });

    let stream = tokio_stream::iter(initial_event).chain(updates);
    Sse::new(stream).keep_alive(KeepAlive::default())
}
