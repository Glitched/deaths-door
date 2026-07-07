//! Routes for the day cycle: dawn/dusk transitions, overnight-death
//! announcements, nominations, live vote tallies, and executions.
//!
//! The intended day loop is: `POST /game/day/begin` → announce each death via
//! `POST /game/day/announce_death` → per nomination, either tally live
//! (`/game/vote/start` → `/game/vote/voters` → `/game/vote/confirm`) or post
//! the finished result to `/game/nominations` → `POST /game/day/end` (executes
//! the chopping block) → `POST /game/night/begin`.

use std::collections::HashSet;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::app::{AppError, AppJson, AppResult, AppState};
use crate::events::EventPayload;
use crate::game_state::{compute_death_cleared_effects, GameState, NominationOutcome};
use crate::lighting::LightingScene;
use crate::routes::game::{build_game_state_response, maybe_phase_auto_effects, GameStateResponse};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(begin_day))
        .routes(routes!(announce_death))
        .routes(routes!(record_nomination))
        .routes(routes!(start_vote))
        .routes(routes!(set_vote_voters))
        .routes(routes!(cancel_vote))
        .routes(routes!(confirm_vote))
        .routes(routes!(end_day))
        .routes(routes!(begin_night))
}

// --- Phase transitions ---

/// Begin the day: dawn breaks and the game enters day mode.
///
/// Deaths that happened overnight are waiting in `deaths_to_announce`;
/// announce them via `POST /game/day/announce_death`, run nominations via
/// the vote endpoints or `POST /game/nominations`, then close out with
/// `POST /game/day/end` and `POST /game/night/begin`.
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
    state.vote.clear_silent();
    let new_state = state.manager.dispatch(EventPayload::DayBegan).await?;
    maybe_phase_auto_effects(&state, &old_step, &new_state.current_night_step).await;
    Ok(Json(build_game_state_response(&state).await?))
}

/// Begin the night: dusk falls and the game enters night mode.
///
/// Clears the day's nominations, any live vote tally, and anything left on
/// the chopping block (end the day first via `POST /game/day/end` if an
/// execution should happen). A night begun after any day is not the first night.
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
    state.vote.clear_silent();
    let new_state = state.manager.dispatch(EventPayload::NightBegan).await?;
    maybe_phase_auto_effects(&state, &old_step, &new_state.current_night_step).await;
    Ok(Json(build_game_state_response(&state).await?))
}

// --- Death announcements ---

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

// --- Nominations ---

/// Check that a nominee is valid right now: daytime, exists, alive, and not
/// yet nominated today.
fn validate_nominee(game: &GameState, player_name: &str) -> AppResult<()> {
    if !game.is_day() {
        return Err(AppError::bad_request("Nominations happen during the day"));
    }
    let nominee = game
        .get_player(player_name)
        .ok_or_else(|| AppError::not_found(format!("Player not found: {player_name}")))?;
    if !nominee.is_alive {
        return Err(AppError::bad_request(format!(
            "{} is dead and cannot be nominated",
            nominee.name
        )));
    }
    if game.was_nominated_today(player_name) {
        return Err(AppError::bad_request(format!(
            "{} has already been nominated today",
            nominee.name
        )));
    }
    Ok(())
}

/// Check that every voter exists, is eligible, and appears only once.
fn validate_voters(game: &GameState, voters: &[String]) -> AppResult<()> {
    let eligible = game.eligible_voters();
    let mut seen = HashSet::new();
    for voter in voters {
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
    Ok(())
}

/// Validate and record a confirmed nomination, returning its outcome. Shared
/// by `POST /game/nominations` and `POST /game/vote/confirm` (the latter
/// already played the drama sting at `vote/start`, so it skips it here).
async fn resolve_nomination(
    state: &AppState,
    player_name: &str,
    voters: &[String],
    votes: Option<u32>,
    fire_drama: bool,
) -> AppResult<NominationOutcome> {
    let game = state.manager.get_state().await?;
    validate_nominee(&game, player_name)?;
    validate_voters(&game, voters)?;
    let votes = votes.unwrap_or(voters.len() as u32);

    let new_state = state
        .manager
        .dispatch(EventPayload::NominationRecorded {
            player_name: player_name.to_string(),
            voters: voters.to_vec(),
            votes,
        })
        .await?;
    let outcome = new_state
        .nominations_today
        .last()
        .map(|n| n.outcome)
        .unwrap_or(NominationOutcome::BlockUnchanged);
    if fire_drama
        && outcome == NominationOutcome::OnTheBlock
        && state.effects.auto_effects().nomination
    {
        state.effects.trigger(LightingScene::Drama, false).await;
    }
    Ok(outcome)
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

/// Confirm a nomination's vote and resolve the chopping block in one call.
///
/// For a live tally shown on the overlay, use `POST /game/vote/start` +
/// `/game/vote/voters` + `/game/vote/confirm` instead. Meeting the execution
/// threshold AND beating the current block's votes puts the nominee on the
/// block; exactly tying the current block empties it (a tie means no one is
/// executed); fewer votes change nothing. Dead voters' dead votes are spent
/// automatically, and the nomination lands in `nominations_today` (each
/// player can be nominated once per day).
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
    let outcome =
        resolve_nomination(&state, &req.player_name, &req.voters, req.votes, true).await?;
    // A live tally for the same nominee is superseded by the direct result.
    if state
        .vote
        .current()
        .is_some_and(|v| v.player_name == req.player_name)
    {
        state.vote.clear_silent();
    }
    Ok(Json(NominationResponse {
        outcome,
        state: build_game_state_response(&state).await?,
    }))
}

// --- Live vote tally ---

#[derive(Deserialize, ToSchema)]
pub struct StartVoteRequest {
    /// The nominated player. Must be alive and not yet nominated today.
    #[schema(example = "Alice")]
    pub player_name: String,
}

/// Start a live vote tally for a nominee.
///
/// The tally appears as `vote_in_progress` in `/game/state` and every SSE
/// frame, so the overlay can show the count climbing as hands go around the
/// circle. Replaces any tally already open. Fires the drama scene if the
/// nomination auto-effect is on (the confirm won't re-fire it).
#[utoipa::path(
    post, path = "/game/vote/start", tag = "Game",
    request_body = StartVoteRequest,
    responses(
        (status = 200, description = "Tally started; full game state returned", body = GameStateResponse),
        (status = 400, description = "Not daytime, nominee dead, or already nominated today"),
        (status = 404, description = "Nominee not found")
    )
)]
async fn start_vote(
    State(state): State<AppState>,
    AppJson(req): AppJson<StartVoteRequest>,
) -> AppResult<Json<GameStateResponse>> {
    let game = state.manager.get_state().await?;
    validate_nominee(&game, &req.player_name)?;
    state.vote.start(req.player_name).await;
    // The nomination announcement is the dramatic moment.
    if state.effects.auto_effects().nomination {
        state.effects.trigger(LightingScene::Drama, false).await;
    }
    Ok(Json(build_game_state_response(&state).await?))
}

#[derive(Deserialize, ToSchema)]
pub struct SetVotersRequest {
    /// The complete list of voters selected so far (send the full selection on
    /// every change — this replaces, not appends).
    #[schema(example = json!(["Bob", "Carol"]))]
    pub voters: Vec<String>,
}

/// Update the live tally's voter selection (replaces the whole list).
#[utoipa::path(
    post, path = "/game/vote/voters", tag = "Game",
    request_body = SetVotersRequest,
    responses(
        (status = 200, description = "Tally updated; full game state returned", body = GameStateResponse),
        (status = 400, description = "No vote in progress, or a voter is ineligible/duplicated"),
        (status = 404, description = "A voter was not found")
    )
)]
async fn set_vote_voters(
    State(state): State<AppState>,
    AppJson(req): AppJson<SetVotersRequest>,
) -> AppResult<Json<GameStateResponse>> {
    let game = state.manager.get_state().await?;
    validate_voters(&game, &req.voters)?;
    if state.vote.set_voters(req.voters).await.is_none() {
        return Err(AppError::bad_request("No vote in progress"));
    }
    Ok(Json(build_game_state_response(&state).await?))
}

/// Abandon the live tally without recording anything.
#[utoipa::path(
    post, path = "/game/vote/cancel", tag = "Game",
    responses((status = 200, description = "Tally cleared (safe when none is open)", body = GameStateResponse))
)]
async fn cancel_vote(State(state): State<AppState>) -> AppResult<Json<GameStateResponse>> {
    state.vote.cancel().await;
    Ok(Json(build_game_state_response(&state).await?))
}

#[derive(Deserialize, ToSchema)]
pub struct ConfirmVoteRequest {
    /// Counted vote total, if it differs from the number of selected voters
    /// (e.g. a Bureaucrat's vote counts three times).
    pub votes: Option<u32>,
}

/// Confirm the live tally: records the nomination and resolves the chopping
/// block exactly like `POST /game/nominations`, then clears the tally.
#[utoipa::path(
    post, path = "/game/vote/confirm", tag = "Game",
    request_body = ConfirmVoteRequest,
    responses(
        (status = 200, description = "Vote resolved; outcome plus full game state", body = NominationResponse),
        (status = 400, description = "No vote in progress, or the tally is no longer valid")
    )
)]
async fn confirm_vote(
    State(state): State<AppState>,
    AppJson(req): AppJson<ConfirmVoteRequest>,
) -> AppResult<Json<NominationResponse>> {
    let session = state
        .vote
        .clear_silent()
        .ok_or_else(|| AppError::bad_request("No vote in progress"))?;
    // Drama already played at vote/start; don't re-fire on the block change.
    match resolve_nomination(
        &state,
        &session.player_name,
        &session.voters,
        req.votes,
        false,
    )
    .await
    {
        Ok(outcome) => Ok(Json(NominationResponse {
            outcome,
            state: build_game_state_response(&state).await?,
        })),
        Err(err) => {
            // Leave the tally on screen so the storyteller can fix and retry.
            state.vote.restore(session);
            Err(err)
        }
    }
}

// --- End of day ---

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
    state.vote.clear_silent();
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
