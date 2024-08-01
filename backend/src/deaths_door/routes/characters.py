from fastapi import APIRouter
from pydantic import BaseModel

from ..game import Game

router = APIRouter(prefix="/characters")

# Initialize a sample game state for debugging purposes
game = Game.get_sample_game()

should_reveal_roles = True


@router.get("/list")
async def get_game_roles():
    """List the names of roles present in the current game."""
    global game
    return game.included_roles


class AddRoleRequest(BaseModel):
    """Request to add a role to the game."""

    name: str


@router.post("/add")
async def add_role(req: AddRoleRequest):
    """Add a role to the current game."""
    global game

    game.include_role(req.name)


class AddRoleMultiRequest(BaseModel):
    """Request to add multiple roles to the game."""

    names: list[str]


@router.post("/add/multi")
async def add_role_multi(req: AddRoleMultiRequest):
    """Add multiple roles to the current game."""
    global game

    for name in req.names:
        game.include_role(name)


class RemoveRoleRequest(BaseModel):
    """Request to remove a role from the game."""

    name: str


@router.post("/remove")
async def remove_role(req: RemoveRoleRequest):
    """Remove a role from the current game."""
    global game

    game.remove_role(req.name)
