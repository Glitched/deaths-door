"""Routes for managing character roles in the game."""

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, Field

from ..character import CharacterOut
from ..events import RoleIncluded, RoleRemoved, RolesIncluded
from ..game_manager import game_manager


class AddRoleResponse(BaseModel):
    """Response after adding a role to the game."""

    status: str = Field(..., description="Operation status", examples=["success"])
    included_roles: list[str] = Field(
        ...,
        description="Names of all roles now included in the game",
        examples=[["Imp", "Chef", "Butler", "Baron"]],
    )


class AddRoleMultiResponse(BaseModel):
    """Response after adding multiple roles to the game."""

    status: str = Field(..., description="Operation status", examples=["success"])
    added_count: int = Field(..., description="Number of roles successfully added", examples=[7])


router = APIRouter(prefix="/characters", tags=["Characters"])


@router.get("/list")
async def get_game_roles() -> list[CharacterOut]:
    """
    List roles that have been added to the current game via /characters/add/multi.

    This returns the active role pool that players will be randomly assigned from.
    To see ALL possible roles in the script, use GET /game/script/roles instead.
    """
    state = await game_manager.get_state()
    return [c.to_out() for c in state.get_included_roles()]


class AddRoleRequest(BaseModel):
    """Request to add a role to the game."""

    name: str = Field(
        ...,
        description="Name of the character/role to add to the game",
        examples=["Imp", "Chef", "Empath", "Baron"],
    )


@router.post("/add", responses={400: {"description": "Role not found or already included"}})
async def add_role(req: AddRoleRequest) -> AddRoleResponse:
    """Add a role to the current game's included roles."""
    state = await game_manager.get_state()
    script = state.get_script()
    if script.get_character(req.name) is None:
        raise HTTPException(status_code=400, detail=f"Role not found: {req.name}")

    new_state = await game_manager.dispatch(RoleIncluded(name=req.name))
    return AddRoleResponse(
        status="success",
        included_roles=list(new_state.included_role_names),
    )


class AddRoleMultiRequest(BaseModel):
    """Request to add multiple roles to the game."""

    names: list[str] = Field(
        ...,
        description="List of character names to add to the game",
        examples=[["Imp", "Chef", "Butler", "Baron", "Librarian", "Empath", "Mayor"]],
    )


@router.post("/add/multi", responses={400: {"description": "One or more roles not found"}})
async def add_role_multi(req: AddRoleMultiRequest) -> AddRoleMultiResponse:
    """Add multiple roles to the current game (atomic operation)."""
    state = await game_manager.get_state()
    script = state.get_script()

    # Validate all names first
    for name in req.names:
        if script.get_character(name) is None:
            raise HTTPException(status_code=400, detail=f"Role not found: {name}")

    await game_manager.dispatch(RolesIncluded(names=tuple(req.names)))
    return AddRoleMultiResponse(status="success", added_count=len(req.names))


class RemoveRoleRequest(BaseModel):
    """Request to remove a role from the game."""

    name: str = Field(
        ...,
        description="Name of the character/role to remove from the game",
        examples=["Imp"],
    )


@router.post("/remove", responses={404: {"description": "Role not found in included roles"}})
async def remove_role(req: RemoveRoleRequest) -> AddRoleResponse:
    """Remove a role from the current game's included roles."""
    state = await game_manager.get_state()
    normalized = req.name.lower().strip()
    if not any(r.lower().strip() == normalized for r in state.included_role_names):
        raise HTTPException(status_code=404, detail=f"Role not in game: {req.name}")

    new_state = await game_manager.dispatch(RoleRemoved(name=req.name))
    return AddRoleResponse(
        status="success",
        included_roles=list(new_state.included_role_names),
    )
