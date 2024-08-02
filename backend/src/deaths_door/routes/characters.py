from fastapi import APIRouter, Depends
from pydantic import BaseModel

from ..game import Game
from ..game_manager import get_current_game

router = APIRouter(prefix="/characters")


@router.get("/list")
async def get_game_roles(game: Game = Depends(get_current_game)):
    """List the names of roles present in the current game."""
    return game.included_roles


class AddRoleRequest(BaseModel):
    """Request to add a role to the game."""

    name: str


@router.post("/add")
async def add_role(req: AddRoleRequest, game: Game = Depends(get_current_game)):
    """Add a role to the current game."""
    game.include_role(req.name)


class AddRoleMultiRequest(BaseModel):
    """Request to add multiple roles to the game."""

    names: list[str]


@router.post("/add/multi")
async def add_role_multi(
    req: AddRoleMultiRequest, game: Game = Depends(get_current_game)
):
    """Add multiple roles to the current game."""
    for name in req.names:
        game.include_role(name)


class RemoveRoleRequest(BaseModel):
    """Request to remove a role from the game."""

    name: str


@router.post("/remove")
async def remove_role(req: RemoveRoleRequest, game: Game = Depends(get_current_game)):
    """Remove a role from the current game."""
    game.remove_role(req.name)
