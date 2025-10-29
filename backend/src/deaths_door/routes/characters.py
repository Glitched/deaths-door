from contextlib import AbstractAsyncContextManager

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from ..character import Character
from ..game import Game
from ..game_manager import get_current_game

router = APIRouter(prefix="/characters")


@router.get("/list")
async def get_game_roles(game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)):
    """List the names of roles present in the current game."""
    async with game_ctx as game:
        return game.included_roles


class AddRoleRequest(BaseModel):
    """Request to add a role to the game."""

    name: str


@router.post("/add")
async def add_role(
    req: AddRoleRequest, game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)
):
    """Add a role to the current game."""
    async with game_ctx as game:
        try:
            game.include_role(req.name)
            return {
                "status": "success",
                "included_roles": [r.name for r in game.included_roles],
            }
        except ValueError as e:
            raise HTTPException(status_code=400, detail=str(e)) from e


class AddRoleMultiRequest(BaseModel):
    """Request to add multiple roles to the game."""

    names: list[str]


@router.post("/add/multi")
async def add_role_multi(
    req: AddRoleMultiRequest, game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)
):
    """Add multiple roles to the current game."""
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

        return {"status": "success", "added_count": len(characters)}


class RemoveRoleRequest(BaseModel):
    """Request to remove a role from the game."""

    name: str


@router.post("/remove")
async def remove_role(
    req: RemoveRoleRequest, game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)
):
    """Remove a role from the current game."""
    async with game_ctx as game:
        try:
            game.remove_role(req.name)
            return {
                "status": "success",
                "included_roles": [r.name for r in game.included_roles],
            }
        except ValueError as e:
            raise HTTPException(status_code=404, detail=str(e)) from e
