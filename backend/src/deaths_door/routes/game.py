from contextlib import AbstractAsyncContextManager

from fastapi import APIRouter, Depends
from fastapi.exceptions import HTTPException
from pydantic import BaseModel

from ..game import Game
from ..game_manager import get_current_game, replace_game
from ..script import ScriptName

router = APIRouter(prefix="/game")


class NewGameRequest(BaseModel):
    """Request to add a player to the game."""

    script_name: str


@router.post("/new")
async def new_game(req: NewGameRequest):
    """Start a new game."""
    script_name = ScriptName.from_str(req.script_name)
    if script_name is None:
        raise HTTPException(status_code=404, detail="Script not found")

    await replace_game(Game(script_name))


@router.get("/script/name")
async def get_game_script(game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)):
    """Return the name of the script for the current game."""
    async with game_ctx as game:
        return game.script.name.value


@router.get("/script/roles")
async def get_game_script_roles(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
):
    """Return the name of the script for the current game."""
    async with game_ctx as game:
        return [c.to_out() for c in game.script.characters]


@router.get("/script/night/first")
async def get_game_first_night_steps(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
):
    """Return the first night steps."""
    async with game_ctx as game:
        return list(game.get_first_night_steps())


@router.get("/status_effects")
async def get_game_status_effects(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
):
    """Return the status effects for the current game."""
    async with game_ctx as game:
        return game.get_status_effects()
