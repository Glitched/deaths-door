//! API routes for DMX lighting control.

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::app::{AppError, AppJson, AppResult, AppState};
use crate::lighting::LightingScene;
use crate::sound::{SoundFx, SoundName};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_status))
        .routes(routes!(list_scenes))
        .routes(routes!(trigger_scene))
        .routes(routes!(trigger_integrated_scene))
        .routes(routes!(set_channel))
        .routes(routes!(set_position))
        .routes(routes!(blackout))
        .routes(routes!(save_player_position))
        .routes(routes!(get_all_positions))
        .routes(routes!(spotlight_player))
}

#[derive(Serialize, ToSchema)]
pub struct OperationResponse {
    pub status: String,
    pub message: String,
}

fn ok(message: impl Into<String>) -> Json<OperationResponse> {
    Json(OperationResponse {
        status: "success".to_string(),
        message: message.into(),
    })
}

#[derive(Serialize, ToSchema)]
pub struct SceneListResponse {
    pub scenes: Vec<String>,
}

#[utoipa::path(
    get, path = "/lights/status", tag = "Lighting",
    responses((status = 200, description = "DMX connection status and detected devices"))
)]
async fn get_status(State(state): State<AppState>) -> Json<Value> {
    let status = state.lighting.status();
    let port_info = status
        .serial_port
        .map(|port| json!({ "port": port, "is_open": true }))
        .unwrap_or(Value::Null);
    Json(json!({
        "connected": status.connected,
        "serial_port": port_info,
        "fixtures": {
            "light1": status.has_light1,
            "light2": status.has_light2,
            "fog": status.has_fog,
        },
        "calibrated_positions": status.calibrated_positions,
    }))
}

#[utoipa::path(
    get, path = "/lights/scenes/list", tag = "Lighting",
    responses((status = 200, description = "Available scene names", body = SceneListResponse))
)]
async fn list_scenes(State(state): State<AppState>) -> Json<SceneListResponse> {
    Json(SceneListResponse {
        scenes: state.lighting.list_scenes(),
    })
}

#[utoipa::path(
    post, path = "/lights/scene/{name}", tag = "Lighting",
    params(("name" = String, Path, description = "Scene name, e.g. death/drama/blackout")),
    responses(
        (status = 200, description = "Scene triggered", body = OperationResponse),
        (status = 404, description = "Scene not found")
    )
)]
async fn trigger_scene(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> AppResult<Json<OperationResponse>> {
    if LightingScene::from_str(&name).is_none() {
        return Err(AppError::not_found(format!("Scene '{name}' not found")));
    }
    state.lighting.trigger_scene(&name);
    Ok(ok(format!("Scene '{name}' triggered successfully")))
}

/// Trigger a lighting scene plus its matching sound effect if one exists.
#[utoipa::path(
    post, path = "/lights/scene/integrated/{name}", tag = "Lighting",
    params(("name" = String, Path, description = "Scene name")),
    responses(
        (status = 200, description = "Integrated scene triggered", body = OperationResponse),
        (status = 404, description = "Scene not found")
    )
)]
async fn trigger_integrated_scene(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> AppResult<Json<OperationResponse>> {
    if LightingScene::from_str(&name).is_none() {
        return Err(AppError::not_found(format!("Scene '{name}' not found")));
    }
    state.lighting.trigger_scene(&name);
    if let Some(sound) = SoundName::from_str(&name) {
        // Best-effort (the scene is lighting-primary); off the async worker.
        tokio::task::spawn_blocking(move || {
            let _ = SoundFx::new().play(sound);
        });
    }
    Ok(ok(format!(
        "Integrated scene '{name}' triggered successfully"
    )))
}

#[derive(Deserialize, ToSchema)]
pub struct ChannelSetRequest {
    pub value: i64,
}

#[utoipa::path(
    post, path = "/lights/fixture/{fixture_id}/channel/{channel}", tag = "Lighting",
    params(
        ("fixture_id" = i64, Path, description = "1=Light1, 2=Light2, 3=Fog"),
        ("channel" = i64, Path, description = "DMX channel 1-11")
    ),
    request_body = ChannelSetRequest,
    responses((status = 200, description = "Channel set", body = OperationResponse))
)]
async fn set_channel(
    State(state): State<AppState>,
    Path((fixture_id, channel)): Path<(i64, i64)>,
    AppJson(req): AppJson<ChannelSetRequest>,
) -> AppResult<Json<OperationResponse>> {
    state.lighting.set_channel(fixture_id, channel, req.value);
    Ok(ok(format!(
        "Fixture {fixture_id} channel {channel} set to {}",
        req.value
    )))
}

#[derive(Deserialize, ToSchema)]
pub struct PositionSetRequest {
    pub pan: i64,
    pub tilt: i64,
    #[serde(default = "default_true")]
    pub fine: bool,
}

fn default_true() -> bool {
    true
}

#[utoipa::path(
    post, path = "/lights/fixture/{fixture_id}/position", tag = "Lighting",
    params(("fixture_id" = i64, Path, description = "1 or 2")),
    request_body = PositionSetRequest,
    responses((status = 200, description = "Fixture positioned", body = OperationResponse))
)]
async fn set_position(
    State(state): State<AppState>,
    Path(fixture_id): Path<i64>,
    AppJson(req): AppJson<PositionSetRequest>,
) -> AppResult<Json<OperationResponse>> {
    state
        .lighting
        .set_position(fixture_id, req.pan, req.tilt, req.fine);
    Ok(ok(format!(
        "Fixture {fixture_id} positioned to pan={}, tilt={}",
        req.pan, req.tilt
    )))
}

#[utoipa::path(
    post, path = "/lights/blackout", tag = "Lighting",
    responses((status = 200, description = "All lights blacked out", body = OperationResponse))
)]
async fn blackout(State(state): State<AppState>) -> AppResult<Json<OperationResponse>> {
    state.lighting.blackout();
    Ok(ok("All lights blacked out"))
}

#[derive(Deserialize, ToSchema)]
pub struct CalibrationSaveRequest {
    pub pan: i64,
    pub tilt: i64,
}

#[utoipa::path(
    post, path = "/lights/calibrate/player/{player_num}/save", tag = "Lighting",
    params(("player_num" = i64, Path, description = "Player/chair number")),
    request_body = CalibrationSaveRequest,
    responses((status = 200, description = "Position saved", body = OperationResponse))
)]
async fn save_player_position(
    State(state): State<AppState>,
    Path(player_num): Path<i64>,
    AppJson(req): AppJson<CalibrationSaveRequest>,
) -> AppResult<Json<OperationResponse>> {
    state
        .lighting
        .save_player_position(player_num, req.pan, req.tilt);
    Ok(ok(format!("Position saved for player {player_num}")))
}

#[derive(Serialize, ToSchema)]
pub struct PositionsListResponse {
    pub positions: BTreeMap<i64, BTreeMap<String, i64>>,
}

#[utoipa::path(
    get, path = "/lights/calibrate/positions", tag = "Lighting",
    responses((status = 200, description = "All calibrated positions", body = PositionsListResponse))
)]
async fn get_all_positions(State(state): State<AppState>) -> Json<PositionsListResponse> {
    let positions = state
        .lighting
        .get_all_positions()
        .into_iter()
        .map(|(num, pos)| {
            let mut m = BTreeMap::new();
            m.insert("player_num".to_string(), pos.player_num);
            m.insert("pan".to_string(), pos.pan);
            m.insert("tilt".to_string(), pos.tilt);
            (num, m)
        })
        .collect();
    Json(PositionsListResponse { positions })
}

#[derive(Deserialize, ToSchema)]
pub struct SpotlightRequest {
    #[serde(default = "default_brightness")]
    pub brightness: i64,
    #[serde(default = "default_fixture")]
    pub fixture_id: i64,
}

fn default_brightness() -> i64 {
    255
}
fn default_fixture() -> i64 {
    1
}

#[utoipa::path(
    post, path = "/lights/spotlight/player/{player_num}", tag = "Lighting",
    params(("player_num" = i64, Path, description = "Player number")),
    request_body = SpotlightRequest,
    responses(
        (status = 200, description = "Player spotlighted", body = OperationResponse),
        (status = 404, description = "No calibrated position for that player")
    )
)]
async fn spotlight_player(
    State(state): State<AppState>,
    Path(player_num): Path<i64>,
    AppJson(req): AppJson<SpotlightRequest>,
) -> AppResult<Json<OperationResponse>> {
    if !state.lighting.has_position(player_num) {
        return Err(AppError::not_found(format!(
            "No calibrated position found for player {player_num}. Please calibrate the position first."
        )));
    }
    state
        .lighting
        .spotlight_player(player_num, req.brightness, req.fixture_id);
    Ok(ok(format!(
        "Spotlighting player {player_num} with fixture {}",
        req.fixture_id
    )))
}
