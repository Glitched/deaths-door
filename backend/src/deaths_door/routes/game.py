from contextlib import AbstractAsyncContextManager

from fastapi import APIRouter, Depends
from fastapi.exceptions import HTTPException
from pydantic import BaseModel, Field

from ..character import CharacterOut
from ..game import Game
from ..game_manager import get_current_game, replace_game
from ..night_step import NightStep
from ..player import PlayerOut
from ..script import ScriptName
from ..status_effects import StatusEffectOut
from . import timer as timer_routes

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
    """Return all possible character roles available in the current script.

    This returns the complete set of roles defined by the script (e.g., all Trouble Brewing roles).
    To see which roles have been added to the current game, use GET /characters/list instead.
    """
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


class GameStateResponse(BaseModel):
    """Complete game state including all relevant information."""

    script_name: str = Field(
        ...,
        description="Name of the current script/edition",
        examples=["trouble_brewing"],
    )
    players: list[PlayerOut] = Field(
        ...,
        description="All players in the game with their characters and status",
    )
    current_night_step: str = Field(
        ...,
        description="Current night step bookmark",
        examples=["Dusk", "Poisoner", "Imp"],
    )
    is_first_night: bool = Field(
        ...,
        description="Whether this is the first night or a subsequent night",
    )
    should_reveal_roles: bool = Field(
        ...,
        description="Whether roles should be revealed to players",
    )
    status_effects: list[StatusEffectOut] = Field(
        ...,
        description="All active status effects in the game",
    )
    included_roles: list[CharacterOut] = Field(
        ...,
        description="Roles that have been included but not yet assigned to players",
    )
    night_steps: list[NightStep] = Field(
        ...,
        description="Night steps for the current night (filtered based on is_first_night)",
    )
    timer: timer_routes.TimerStateResponse = Field(
        ...,
        description="Current timer state (is_running and seconds remaining)",
    )


@router.get("/state")
async def get_game_state(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> GameStateResponse:
    """Get the complete game state in a single request."""
    # Get game state first
    async with game_ctx as game:
        # Get appropriate night steps based on is_first_night
        if game.is_first_night:
            night_steps = list(game.get_first_night_steps())
        else:
            night_steps = list(game.get_other_night_steps())

        # Capture all game state while holding the lock
        game_state = {
            "script_name": game.script.name.value,
            "players": [player.to_out() for player in game.players],
            "current_night_step": game.current_night_step,
            "is_first_night": game.is_first_night,
            "should_reveal_roles": game.should_reveal_roles,
            "status_effects": game.get_status_effects(),
            "included_roles": [role.to_out() for role in game.included_roles],
            "night_steps": night_steps,
        }

    # Get timer state AFTER releasing game lock to avoid lock ordering issues
    timer_is_running = await timer_routes.state.get_is_running()
    timer_seconds = await timer_routes.state.get_seconds()

    return GameStateResponse(
        **game_state,
        timer=timer_routes.TimerStateResponse(
            is_running=timer_is_running,
            seconds=timer_seconds,
        ),
    )


class NightPhaseResponse(BaseModel):
    """Response containing current night phase information."""

    current_night_step: str = Field(
        ...,
        description="Name of the current night step (e.g., 'Dusk', 'Poisoner')",
        examples=["Dusk", "Poisoner", "Fortune Teller"],
    )
    is_first_night: bool = Field(
        ...,
        description="Whether this is the first night or a subsequent night",
        examples=[True, False],
    )


@router.get("/night/phase")
async def get_night_phase(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> NightPhaseResponse:
    """Get the current night phase information."""
    async with game_ctx as game:
        return NightPhaseResponse(
            current_night_step=game.current_night_step,
            is_first_night=game.is_first_night,
        )


class SetNightStepRequest(BaseModel):
    """Request to set the current night step."""

    step: str = Field(
        ...,
        description="Name of the night step to set as current",
        examples=["Dusk", "Poisoner", "Imp", "Dawn"],
    )


@router.post("/night/phase/step")
async def set_night_step(
    req: SetNightStepRequest,
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> NightPhaseResponse:
    """Set the current night step."""
    async with game_ctx as game:
        game.current_night_step = req.step
        return NightPhaseResponse(
            current_night_step=game.current_night_step,
            is_first_night=game.is_first_night,
        )


class SetFirstNightRequest(BaseModel):
    """Request to set whether it's the first night."""

    is_first_night: bool = Field(
        ...,
        description="Whether this is the first night",
        examples=[True, False],
    )


@router.post("/night/phase/first_night")
async def set_first_night(
    req: SetFirstNightRequest,
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> NightPhaseResponse:
    """Set whether this is the first night (automatically resets to Dusk)."""
    async with game_ctx as game:
        game.is_first_night = req.is_first_night
        game.current_night_step = "Dusk"
        return NightPhaseResponse(
            current_night_step=game.current_night_step,
            is_first_night=game.is_first_night,
        )


@router.get("/script/night/steps")
async def get_night_steps(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> list[NightStep]:
    """Return the night steps for the current night (first or other based on game state)."""
    async with game_ctx as game:
        if game.is_first_night:
            return list(game.get_first_night_steps())
        else:
            return list(game.get_other_night_steps())
