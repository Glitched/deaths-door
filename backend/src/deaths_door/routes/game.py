"""Routes for game management, night phases, and consolidated state."""

from uuid import UUID

from fastapi import APIRouter
from fastapi.exceptions import HTTPException
from pydantic import BaseModel, Field

from ..character import CharacterOut
from ..events import EventType, FirstNightSet, NightStepSet, describe_event
from ..game_manager import game_manager
from ..game_state import game_state_to_included_role_outs, player_state_to_out
from ..night_step import NightStep
from ..player import PlayerOut
from ..script_name import ScriptName
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

    await game_manager.create_game(script_name.value)
    return NewGameResponse(status="success", script_name=script_name.value)


@router.get("/script/name")
async def get_game_script() -> str:
    """Return the name of the script for the current game."""
    state = await game_manager.get_state()
    return state.script_name


@router.get("/script/roles")
async def get_game_script_roles() -> list[CharacterOut]:
    """
    Return all possible character roles available in the current script.

    This returns the complete set of roles defined by the script (e.g., all Trouble Brewing roles).
    To see which roles have been added to the current game, use GET /characters/list instead.
    """
    state = await game_manager.get_state()
    script = state.get_script()
    return [c.to_out() for c in script.characters]


@router.get("/script/night/first")
async def get_game_first_night_steps() -> list[NightStep]:
    """Return the first night steps."""
    state = await game_manager.get_state()
    # Temporarily set first night to get those steps
    first_night_state = state.model_copy(update={"is_first_night": True})
    return first_night_state.get_night_steps()


@router.get("/script/night/other")
async def get_game_other_night_steps() -> list[NightStep]:
    """Return the other night steps (subsequent nights)."""
    state = await game_manager.get_state()
    other_night_state = state.model_copy(update={"is_first_night": False})
    return other_night_state.get_night_steps()


@router.get("/status_effects")
async def get_game_status_effects() -> list[StatusEffectOut]:
    """Return the status effects for the current game."""
    state = await game_manager.get_state()
    return state.get_status_effects()


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
    living_player_count: int = Field(
        ...,
        description="Number of living players",
    )
    execution_threshold: int = Field(
        ...,
        description="Number of votes needed to execute (≥50% of living players)",
    )
    dead_players_with_vote: list[str] = Field(
        ...,
        description="Names of dead players who haven't used their dead vote",
    )
    timer: timer_routes.TimerStateResponse = Field(
        ...,
        description="Current timer state (is_running and seconds remaining)",
    )


@router.get("/state")
async def get_game_state() -> GameStateResponse:
    """Get the complete game state in a single request."""
    state = await game_manager.get_state()
    script = state.get_script()

    # Get timer state
    timer_is_running = await timer_routes.state.get_is_running()
    timer_seconds = await timer_routes.state.get_seconds()

    return GameStateResponse(
        script_name=state.script_name,
        players=[player_state_to_out(p, script) for p in state.players],
        current_night_step=state.current_night_step,
        is_first_night=state.is_first_night,
        should_reveal_roles=state.should_reveal_roles,
        status_effects=state.get_status_effects(),
        included_roles=game_state_to_included_role_outs(state),
        night_steps=state.get_night_steps(),
        living_player_count=state.living_player_count,
        execution_threshold=state.execution_threshold,
        dead_players_with_vote=state.get_dead_players_with_vote(),
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
async def get_night_phase() -> NightPhaseResponse:
    """Get the current night phase information."""
    state = await game_manager.get_state()
    return NightPhaseResponse(
        current_night_step=state.current_night_step,
        is_first_night=state.is_first_night,
    )


class SetNightStepRequest(BaseModel):
    """Request to set the current night step."""

    step: str = Field(
        ...,
        description="Name of the night step to set as current",
        examples=["Dusk", "Poisoner", "Imp", "Dawn"],
    )


@router.post("/night/phase/step")
async def set_night_step(req: SetNightStepRequest) -> NightPhaseResponse:
    """Set the current night step."""
    new_state = await game_manager.dispatch(NightStepSet(step=req.step))
    return NightPhaseResponse(
        current_night_step=new_state.current_night_step,
        is_first_night=new_state.is_first_night,
    )


class SetFirstNightRequest(BaseModel):
    """Request to set whether it's the first night."""

    is_first_night: bool = Field(
        ...,
        description="Whether this is the first night",
        examples=[True, False],
    )


@router.post("/night/phase/first_night")
async def set_first_night(req: SetFirstNightRequest) -> NightPhaseResponse:
    """Set whether this is the first night (automatically resets to Dusk)."""
    new_state = await game_manager.dispatch(FirstNightSet(is_first_night=req.is_first_night))
    return NightPhaseResponse(
        current_night_step=new_state.current_night_step,
        is_first_night=new_state.is_first_night,
    )


@router.get("/script/night/steps")
async def get_night_steps() -> list[NightStep]:
    """Return the night steps for the current night (first or other based on game state)."""
    state = await game_manager.get_state()
    return state.get_night_steps()


# --- Event sourcing endpoints ---


class EventOut(BaseModel):
    """A game event for the history endpoint."""

    version: int = Field(..., description="Version after this event (1-indexed)")
    description: str = Field(..., description="Human-readable description of the event")
    event_type: EventType = Field(..., description="Type of event")
    timestamp: str = Field(..., description="ISO 8601 timestamp")
    payload: dict[str, object] = Field(..., description="Event payload data")


class HistoryResponse(BaseModel):
    """Response containing the event history for the current game."""

    game_id: str = Field(..., description="Current game ID")
    version: int = Field(..., description="Current version (number of events applied)")
    events: list[EventOut] = Field(..., description="All events in chronological order")


@router.get("/history")
async def get_game_history() -> HistoryResponse:
    """Get the full event history for the current game."""
    state = await game_manager.get_state()
    events = await game_manager.get_history()
    return HistoryResponse(
        game_id=str(state.game_id),
        version=state.version,
        events=[
            EventOut(
                version=e.sequence + 1,
                description=describe_event(e.payload),
                event_type=e.payload.type,
                timestamp=e.timestamp.isoformat(),
                payload=e.payload.model_dump(exclude={"type"}),
            )
            for e in events
        ],
    )


class RewindRequest(BaseModel):
    """Request to rewind the game to a specific version."""

    to_version: int = Field(..., description="Version to rewind to (1-based, inclusive)", ge=1)


@router.post("/rewind", responses={400: {"description": "Invalid version"}})
async def rewind_game(req: RewindRequest) -> GameStateResponse:
    """Rewind the current game to a previous version, deleting subsequent events."""
    try:
        await game_manager.rewind(req.to_version)
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e)) from e

    return await get_game_state()


class ForkRequest(BaseModel):
    """Request to fork the game from a specific version."""

    from_version: int = Field(..., description="Version to fork from (1-based, inclusive)", ge=1)


class ForkResponse(BaseModel):
    """Response after forking a game."""

    new_game_id: str = Field(..., description="ID of the newly forked game")
    version: int = Field(..., description="Version of the forked game")


@router.post("/fork", responses={400: {"description": "Invalid version"}})
async def fork_game(req: ForkRequest) -> ForkResponse:
    """Fork the current game from a specific version, creating a new game branch."""
    try:
        new_state = await game_manager.fork(req.from_version)
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e)) from e

    return ForkResponse(new_game_id=str(new_state.game_id), version=new_state.version)


class LoadGameRequest(BaseModel):
    """Request to load a game by ID."""

    game_id: str = Field(..., description="UUID of the game to load")


@router.post("/load", responses={400: {"description": "Game not found"}})
async def load_game(req: LoadGameRequest) -> GameStateResponse:
    """Load a previously saved game by its ID."""
    try:
        game_id = UUID(req.game_id)
    except ValueError as e:
        raise HTTPException(status_code=400, detail=f"Invalid game ID: {req.game_id}") from e

    try:
        await game_manager.load_game(game_id)
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e)) from e

    return await get_game_state()


class GameListResponse(BaseModel):
    """Response listing all saved games."""

    game_ids: list[str] = Field(..., description="List of all game IDs")


@router.get("/list")
async def list_games() -> GameListResponse:
    """List all saved game IDs."""
    ids = await game_manager.list_games()
    return GameListResponse(game_ids=[str(gid) for gid in ids])
