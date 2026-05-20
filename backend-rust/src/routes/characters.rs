//! Routes for managing character roles in the game.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::app::{AppError, AppJson, AppResult, AppState};
use crate::character::CharacterOut;
use crate::events::EventPayload;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_game_roles))
        .routes(routes!(add_role))
        .routes(routes!(add_role_multi))
        .routes(routes!(remove_role))
}

#[derive(Serialize, ToSchema)]
pub struct AddRoleResponse {
    pub status: String,
    /// Names of all roles now in the game's pool.
    pub included_roles: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AddRoleMultiResponse {
    pub status: String,
    pub added_count: usize,
}

/// List the roles currently in the game's pool.
///
/// This is the active pool players are randomly assigned from (built via
/// `/characters/add/multi`). For *all* roles defined by the script, use
/// `GET /game/script/roles` instead.
#[utoipa::path(
    get, path = "/characters/list", tag = "Characters",
    responses((status = 200, description = "Included roles", body = [CharacterOut]))
)]
async fn get_game_roles(State(state): State<AppState>) -> AppResult<Json<Vec<CharacterOut>>> {
    let game = state.manager.get_state().await?;
    Ok(Json(
        game.get_included_roles()
            .iter()
            .map(|c| c.to_out())
            .collect(),
    ))
}

#[derive(Deserialize, ToSchema)]
pub struct AddRoleRequest {
    /// Character name to add to the pool, e.g. `Imp`.
    #[schema(example = "Imp")]
    pub name: String,
}

/// Add a single role to the game's pool.
#[utoipa::path(
    post, path = "/characters/add", tag = "Characters",
    request_body = AddRoleRequest,
    responses(
        (status = 200, description = "Role added", body = AddRoleResponse),
        (status = 400, description = "Role not found in script")
    )
)]
async fn add_role(
    State(state): State<AppState>,
    AppJson(req): AppJson<AddRoleRequest>,
) -> AppResult<Json<AddRoleResponse>> {
    let game = state.manager.get_state().await?;
    let script = game
        .get_script()
        .ok_or_else(|| AppError::internal("No script loaded"))?;
    if script.get_character(&req.name).is_none() {
        return Err(AppError::bad_request(format!(
            "Role not found: {}",
            req.name
        )));
    }

    let new_state = state
        .manager
        .dispatch(EventPayload::RoleIncluded {
            name: req.name.clone(),
        })
        .await?;
    Ok(Json(AddRoleResponse {
        status: "success".to_string(),
        included_roles: new_state.included_role_names.clone(),
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct AddRoleMultiRequest {
    /// Character names to add to the pool (validated atomically).
    #[schema(example = json!(["Imp", "Baron", "Chef", "Empath", "Mayor"]))]
    pub names: Vec<String>,
}

/// Add multiple roles to the game's pool in one atomic call.
///
/// All names are validated against the script first; if any is unknown, none
/// are added. This is the usual way to build the role pool before adding players.
#[utoipa::path(
    post, path = "/characters/add/multi", tag = "Characters",
    request_body = AddRoleMultiRequest,
    responses(
        (status = 200, description = "Roles added", body = AddRoleMultiResponse),
        (status = 400, description = "One or more roles not found in script")
    )
)]
async fn add_role_multi(
    State(state): State<AppState>,
    AppJson(req): AppJson<AddRoleMultiRequest>,
) -> AppResult<Json<AddRoleMultiResponse>> {
    let game = state.manager.get_state().await?;
    let script = game
        .get_script()
        .ok_or_else(|| AppError::internal("No script loaded"))?;

    for name in &req.names {
        if script.get_character(name).is_none() {
            return Err(AppError::bad_request(format!("Role not found: {name}")));
        }
    }

    let added_count = req.names.len();
    state
        .manager
        .dispatch(EventPayload::RolesIncluded { names: req.names })
        .await?;
    Ok(Json(AddRoleMultiResponse {
        status: "success".to_string(),
        added_count,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct RemoveRoleRequest {
    /// Character name to remove from the pool (case-insensitive).
    #[schema(example = "Imp")]
    pub name: String,
}

/// Remove a role from the game's pool.
#[utoipa::path(
    post, path = "/characters/remove", tag = "Characters",
    request_body = RemoveRoleRequest,
    responses(
        (status = 200, description = "Role removed", body = AddRoleResponse),
        (status = 404, description = "Role not in the game's pool")
    )
)]
async fn remove_role(
    State(state): State<AppState>,
    AppJson(req): AppJson<RemoveRoleRequest>,
) -> AppResult<Json<AddRoleResponse>> {
    let game = state.manager.get_state().await?;
    let normalized = req.name.to_lowercase();
    let normalized = normalized.trim();
    let present = game
        .included_role_names
        .iter()
        .any(|r| r.to_lowercase().trim() == normalized);
    if !present {
        return Err(AppError::not_found(format!(
            "Role not in game: {}",
            req.name
        )));
    }

    let new_state = state
        .manager
        .dispatch(EventPayload::RoleRemoved {
            name: req.name.clone(),
        })
        .await?;
    Ok(Json(AddRoleResponse {
        status: "success".to_string(),
        included_roles: new_state.included_role_names.clone(),
    }))
}
