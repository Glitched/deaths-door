from contextlib import AbstractAsyncContextManager

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel, Field

from ..character import Character, CharacterOut
from ..game import Game
from ..game_manager import get_current_game


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
    added_count: int = Field(
        ..., description="Number of roles successfully added", examples=[7]
    )

router = APIRouter(prefix="/characters", tags=["Characters"])


@router.get("/list")
async def get_game_roles(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> list[CharacterOut]:
    """List the names of roles present in the current game."""
    async with game_ctx as game:
        return [c.to_out() for c in game.included_roles]


class AddRoleRequest(BaseModel):
    """Request to add a role to the game."""

    name: str = Field(
        ...,
        description="Name of the character/role to add to the game",
        examples=["Imp", "Chef", "Empath", "Baron"],
    )


@router.post("/add", responses={400: {"description": "Role not found or already included"}})
async def add_role(
    req: AddRoleRequest, game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)
) -> AddRoleResponse:
    """Add a role to the current game's included roles."""
    async with game_ctx as game:
        try:
            game.include_role(req.name)
            return AddRoleResponse(
                status="success",
                included_roles=[r.name for r in game.included_roles],
            )
        except ValueError as e:
            raise HTTPException(status_code=400, detail=str(e)) from e


class AddRoleMultiRequest(BaseModel):
    """Request to add multiple roles to the game."""

    names: list[str] = Field(
        ...,
        description="List of character names to add to the game",
        examples=[["Imp", "Chef", "Butler", "Baron", "Librarian", "Empath", "Mayor"]],
    )


@router.post("/add/multi", responses={400: {"description": "One or more roles not found"}})
async def add_role_multi(
    req: AddRoleMultiRequest, game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)
) -> AddRoleMultiResponse:
    """Add multiple roles to the current game (atomic operation)."""
    async with game_ctx as game:
        # Validate all names first (atomic operation)
        characters: list[Character] = []
        for name in req.names:
            character = game.script.get_character(name)
            if character is None:
                raise HTTPException(status_code=400, detail=f"Role not found: {name}")
            characters.append(character)

        # All valid - now add them
        for character in characters:
            game.included_roles.append(character)

        return AddRoleMultiResponse(status="success", added_count=len(characters))


class RemoveRoleRequest(BaseModel):
    """Request to remove a role from the game."""

    name: str = Field(
        ...,
        description="Name of the character/role to remove from the game",
        examples=["Imp"],
    )


@router.post("/remove", responses={404: {"description": "Role not found in included roles"}})
async def remove_role(
    req: RemoveRoleRequest, game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)
) -> AddRoleResponse:
    """Remove a role from the current game's included roles."""
    async with game_ctx as game:
        try:
            game.remove_role(req.name)
            return AddRoleResponse(
                status="success",
                included_roles=[r.name for r in game.included_roles],
            )
        except ValueError as e:
            raise HTTPException(status_code=404, detail=str(e)) from e
