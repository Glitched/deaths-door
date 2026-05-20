//! Routes for playing sound effects.

use axum::extract::Path;
use axum::Json;
use serde::Serialize;
use serde_json::{Map, Value};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::app::{AppError, AppResult, AppState};
use crate::sound::{sounds_by_category, SoundFx, SoundName};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(play_sound))
        .routes(routes!(list_sounds))
}

#[derive(Serialize, ToSchema)]
pub struct PlaySoundResponse {
    pub status: String,
    pub sound: String,
}

/// Play a sound effect by name.
#[utoipa::path(
    get, path = "/sounds/play/{name}", tag = "Sounds",
    params(("name" = String, Path, description = "Sound name, e.g. death/rooster/drumroll")),
    responses(
        (status = 200, description = "Sound played", body = PlaySoundResponse),
        (status = 404, description = "sound not found"),
        (status = 500, description = "playback failed")
    )
)]
async fn play_sound(Path(name): Path<String>) -> AppResult<Json<PlaySoundResponse>> {
    let sound_name =
        SoundName::from_str(&name).ok_or_else(|| AppError::not_found("Sound not found"))?;
    // `play` blocks until playback starts (or fails to), so run it off the async
    // worker. A decode/device/file failure now surfaces as a 500 with the reason.
    tokio::task::spawn_blocking(move || SoundFx::new().play(sound_name))
        .await
        .map_err(|e| AppError::internal(format!("sound task failed: {e}")))?
        .map_err(|e| AppError::internal(format!("Failed to play '{name}': {e}")))?;
    Ok(Json(PlaySoundResponse {
        status: "success".to_string(),
        sound: name,
    }))
}

/// List all available sounds grouped by category.
#[utoipa::path(
    get, path = "/sounds/list", tag = "Sounds",
    responses((status = 200, description = "Available sounds grouped by category"))
)]
async fn list_sounds() -> Json<Value> {
    // Preserve the Python category ordering.
    let mut map = Map::new();
    for (category, sound_list) in sounds_by_category() {
        let values: Vec<Value> = sound_list
            .iter()
            .map(|s| Value::String(s.value().to_string()))
            .collect();
        map.insert(category.to_string(), Value::Array(values));
    }
    Json(Value::Object(map))
}
