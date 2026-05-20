//! Routes for the countdown timer and Live Activity push tokens.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::app::{AppError, AppJson, AppResult, AppState};
use crate::config;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(fetch_timer))
        .routes(routes!(set_timer))
        .routes(routes!(add_seconds))
        .routes(routes!(start_timer))
        .routes(routes!(stop_timer))
        .routes(routes!(register_push_token))
}

#[derive(Serialize, ToSchema)]
pub struct TimerStateResponse {
    pub is_running: bool,
    pub seconds: i64,
}

#[derive(Serialize, ToSchema)]
pub struct TimerOperationResponse {
    pub status: String,
    pub is_running: bool,
    pub seconds: i64,
}

fn validate_timer_seconds(seconds: i64, allow_negative: bool) -> AppResult<()> {
    let min = if allow_negative {
        -config::TIMER_MAX_SECONDS
    } else {
        0
    };
    let max = config::TIMER_MAX_SECONDS;
    if seconds < min || seconds > max {
        return Err(AppError::bad_request(format!(
            "Timer seconds must be between {min} and {max}"
        )));
    }
    Ok(())
}

/// Fetch the current timer state.
#[utoipa::path(
    get, path = "/timer/fetch", tag = "Timer",
    responses((status = 200, description = "Current timer state", body = TimerStateResponse))
)]
async fn fetch_timer(State(state): State<AppState>) -> Json<TimerStateResponse> {
    Json(TimerStateResponse {
        is_running: state.timer.get_is_running().await,
        seconds: state.timer.get_seconds().await,
    })
}

/// Set the timer to a specific number of seconds and start it.
#[utoipa::path(
    get, path = "/timer/set/{seconds}", tag = "Timer",
    params(("seconds" = i64, Path, description = "Seconds to set, 0..=3600")),
    responses(
        (status = 200, description = "Timer set", body = TimerOperationResponse),
        (status = 400, description = "out of range")
    )
)]
async fn set_timer(
    State(state): State<AppState>,
    Path(seconds): Path<i64>,
) -> AppResult<Json<TimerOperationResponse>> {
    validate_timer_seconds(seconds, false)?;
    state.timer.set_seconds(seconds).await;
    state.timer.set_is_running(true).await;
    Ok(Json(TimerOperationResponse {
        status: "success".to_string(),
        is_running: true,
        seconds,
    }))
}

/// Add (or subtract) seconds from the timer.
#[utoipa::path(
    get, path = "/timer/add/{seconds}", tag = "Timer",
    params(("seconds" = i64, Path, description = "Seconds to add (may be negative)")),
    responses(
        (status = 200, description = "Timer adjusted", body = TimerOperationResponse),
        (status = 400, description = "out of range")
    )
)]
async fn add_seconds(
    State(state): State<AppState>,
    Path(seconds): Path<i64>,
) -> AppResult<Json<TimerOperationResponse>> {
    validate_timer_seconds(seconds, true)?;
    state.timer.add_seconds(seconds).await;
    Ok(Json(TimerOperationResponse {
        status: "success".to_string(),
        is_running: state.timer.get_is_running().await,
        seconds: state.timer.get_seconds().await,
    }))
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct StartQuery {
    pub seconds: Option<i64>,
}

/// Start the timer, optionally setting it to a number of seconds first.
#[utoipa::path(
    get, path = "/timer/start", tag = "Timer",
    params(StartQuery),
    responses(
        (status = 200, description = "Timer started", body = TimerOperationResponse),
        (status = 400, description = "out of range")
    )
)]
async fn start_timer(
    State(state): State<AppState>,
    Query(query): Query<StartQuery>,
) -> AppResult<Json<TimerOperationResponse>> {
    if let Some(seconds) = query.seconds {
        validate_timer_seconds(seconds, false)?;
        state.timer.set_seconds(seconds).await;
    }
    state.timer.set_is_running(true).await;
    Ok(Json(TimerOperationResponse {
        status: "success".to_string(),
        is_running: true,
        seconds: state.timer.get_seconds().await,
    }))
}

/// Stop the timer.
#[utoipa::path(
    get, path = "/timer/stop", tag = "Timer",
    responses((status = 200, description = "Timer stopped", body = TimerOperationResponse))
)]
async fn stop_timer(State(state): State<AppState>) -> Json<TimerOperationResponse> {
    state.timer.set_is_running(false).await;
    Json(TimerOperationResponse {
        status: "success".to_string(),
        is_running: false,
        seconds: state.timer.get_seconds().await,
    })
}

#[derive(Deserialize, ToSchema)]
pub struct PushTokenRequest {
    pub push_token: String,
}

/// Register a Live Activity push token.
#[utoipa::path(
    post, path = "/timer/push_token", tag = "Timer",
    request_body = PushTokenRequest,
    responses((status = 200, description = "Token registered"))
)]
async fn register_push_token(
    State(state): State<AppState>,
    AppJson(req): AppJson<PushTokenRequest>,
) -> Json<serde_json::Value> {
    state.timer.apns().register_token(req.push_token);
    Json(json!({ "status": "registered" }))
}
