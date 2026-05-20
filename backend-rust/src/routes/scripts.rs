//! Routes for browsing scripts/editions and their character lists.

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::app::{AppError, AppResult, AppState};
use crate::character::CharacterOut;
use crate::script_name::ScriptName;
use crate::scripts::get_script_by_name;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(read_scripts))
        .routes(routes!(read_roles))
        .routes(routes!(read_travelers))
        .routes(routes!(read_role))
}

#[derive(Serialize, ToSchema)]
pub struct ScriptListResponse {
    pub scripts: BTreeMap<String, String>,
}

/// List all available scripts/editions.
#[utoipa::path(
    get, path = "/scripts/list", tag = "Scripts",
    responses((status = 200, description = "Available scripts", body = ScriptListResponse))
)]
async fn read_scripts() -> Json<ScriptListResponse> {
    let scripts = ScriptName::ALL
        .iter()
        .map(|s| (s.value().to_string(), s.display().to_string()))
        .collect();
    Json(ScriptListResponse { scripts })
}

/// List all roles defined by a script.
#[utoipa::path(
    get, path = "/scripts/{script_name}/role", tag = "Scripts",
    params(("script_name" = String, Path, description = "Script id, e.g. trouble_brewing")),
    responses(
        (status = 200, description = "Script roles", body = [CharacterOut]),
        (status = 404, description = "script not found")
    )
)]
async fn read_roles(
    State(_state): State<AppState>,
    Path(script_name): Path<String>,
) -> AppResult<Json<Vec<CharacterOut>>> {
    let script =
        get_script_by_name(&script_name).ok_or_else(|| AppError::not_found("Script not found"))?;
    Ok(Json(script.characters.iter().map(|c| c.to_out()).collect()))
}

/// List all travelers defined by a script.
#[utoipa::path(
    get, path = "/scripts/{script_name}/travelers", tag = "Scripts",
    params(("script_name" = String, Path, description = "Script id")),
    responses(
        (status = 200, description = "Script travelers", body = [CharacterOut]),
        (status = 404, description = "script not found")
    )
)]
async fn read_travelers(
    State(_state): State<AppState>,
    Path(script_name): Path<String>,
) -> AppResult<Json<Vec<CharacterOut>>> {
    let script =
        get_script_by_name(&script_name).ok_or_else(|| AppError::not_found("Script not found"))?;
    Ok(Json(script.travelers.iter().map(|c| c.to_out()).collect()))
}

/// Look up a single role by name within a script.
#[utoipa::path(
    get, path = "/scripts/{script_name}/role/{name}", tag = "Scripts",
    params(
        ("script_name" = String, Path, description = "Script id"),
        ("name" = String, Path, description = "Character name")
    ),
    responses(
        (status = 200, description = "Role", body = CharacterOut),
        (status = 404, description = "script not found")
    )
)]
async fn read_role(
    State(_state): State<AppState>,
    Path((script_name, name)): Path<(String, String)>,
) -> AppResult<Json<CharacterOut>> {
    let script =
        get_script_by_name(&script_name).ok_or_else(|| AppError::not_found("Script not found"))?;
    let character = script
        .get_character(&name)
        .ok_or_else(|| AppError::not_found("Role not found"))?;
    Ok(Json(character.to_out()))
}
