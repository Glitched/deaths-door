from contextlib import AbstractAsyncContextManager

from fastapi import APIRouter, Depends
from fastapi.exceptions import HTTPException
from pydantic import BaseModel, Field

from ..character import CharacterOut
from ..game import Game
from ..game_manager import get_current_game, replace_game
from ..night_step import NightStep
from ..script import ScriptName
from ..status_effects import StatusEffectOut

router = APIRouter(prefix="/game", tags=["Game Management"])


class NewGameRequest(BaseModel):
    """Request to start a new game with a specific script."""

    script_name: str = Field(
        ...,
        description="Name of the script/edition to play",
        examples=["trouble_brewing", "bad_moon_rising", "sects_and_violets"],
    )


class NewGameResponse(BaseModel):
    """Response after creating a new game."""

    status: str = Field(..., description="Operation status", examples=["success"])
    script_name: str = Field(
        ...,
        description="Name of the script that was loaded",
        examples=["trouble_brewing"],
    )


@router.post("/new", responses={404: {"description": "Script not found"}})
async def new_game(req: NewGameRequest) -> NewGameResponse:
    """Start a new game with the specified script."""
    script_name = ScriptName.from_str(req.script_name)
    if script_name is None:
        raise HTTPException(status_code=404, detail="Script not found")

    await replace_game(Game(script_name))
    return NewGameResponse(status="success", script_name=script_name.value)


@router.get("/script/name")
async def get_game_script(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> str:
    """Return the name of the script for the current game."""
    async with game_ctx as game:
        return game.script.name.value


@router.get("/script/roles")
async def get_game_script_roles(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> list[CharacterOut]:
    """Return all available character roles in the current script."""
    async with game_ctx as game:
        return [c.to_out() for c in game.script.characters]


@router.get("/script/night/first")
async def get_game_first_night_steps(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> list[NightStep]:
    """Return the first night steps."""
    async with game_ctx as game:
        return list(game.get_first_night_steps())


@router.get("/script/night/other")
async def get_game_other_night_steps(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> list[NightStep]:
    """Return the other night steps (subsequent nights)."""
    async with game_ctx as game:
        return list(game.get_other_night_steps())


@router.get("/status_effects")
async def get_game_status_effects(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> list[StatusEffectOut]:
    """Return the status effects for the current game."""
    async with game_ctx as game:
        return game.get_status_effects()
