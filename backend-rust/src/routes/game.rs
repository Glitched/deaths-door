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
use crate::events::{describe_event, EventPayload};
use crate::game_state::{game_state_to_included_role_outs, player_state_to_out, GameState};
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
    /// Night steps for the current phase, filtered to roles in play.
    pub night_steps: Vec<NightStep>,
    pub living_player_count: usize,
    /// Votes needed to execute (>= 50% of living players).
    pub execution_threshold: usize,
    pub dead_players_with_vote: Vec<String>,
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
        night_steps: state.get_night_steps(),
        living_player_count: state.living_player_count(),
        execution_threshold: state.execution_threshold(),
        dead_players_with_vote: state.get_dead_players_with_vote(),
    }
}

/// Full `/game/state` response: the snapshot plus the timer.
#[derive(Serialize, ToSchema)]
pub struct GameStateResponse {
    #[serde(flatten)]
    pub base: GameStateBase,
    pub timer: TimerStateResponse,
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
    })
}

fn sse_payload(state: &GameState) -> String {
    serde_json::to_string(&game_state_base(state)).unwrap_or_else(|_| "{}".to_string())
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
    let new_state = state
        .manager
        .dispatch(EventPayload::NightStepSet { step: req.step })
        .await?;
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
    let new_state = state
        .manager
        .dispatch(EventPayload::FirstNightSet {
            is_first_night: req.is_first_night,
        })
        .await?;
    Ok(Json(NightPhaseResponse {
        current_night_step: new_state.current_night_step,
        is_first_night: new_state.is_first_night,
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
    let initial = state.manager.get_state().await.ok();
    let initial_event = initial.map(|s| Ok(Event::default().data(sse_payload(&s))));

    let updates = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(s) => Some(Ok(Event::default().data(sse_payload(&s)))),
        Err(_) => None,
    });

    let stream = tokio_stream::iter(initial_event).chain(updates);
    Sse::new(stream).keep_alive(KeepAlive::default())
}
