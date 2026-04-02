"""Routes for managing players in the game."""

import secrets

import anyio
from fastapi import APIRouter, Path
from fastapi.exceptions import HTTPException
from pydantic import BaseModel, Field

from ..alignment import Alignment
from ..events import (
    CharactersSwapped,
    DeadVoteUsedSet,
    PlayerAdded,
    PlayerAlignmentSet,
    PlayerAliveSet,
    PlayerRemoved,
    PlayerRenamed,
    RoleVisibilitySet,
    StatusEffectAdded,
    StatusEffectRemoved,
    TravelerAdded,
)
from ..game_manager import game_manager
from ..game_state import GameState, PlayerState, player_state_to_out
from ..player import PlayerOut

ROLE_REVEAL_TIMEOUT_ATTEMPTS = 100
POLLING_INTERVAL_SECONDS = 0.1

# Mapping of characters to their persistent status effects (for death cleanup)
CHARACTER_PERSISTENT_EFFECTS: dict[str, list[str]] = {
    "Poisoner": ["Poisoned"],
    "Monk": ["Safe"],
    "Butler": ["Butler's Master"],
}


def get_player_or_404(state: GameState, name: str) -> PlayerState:
    """Get player by name or raise 404 error."""
    player = state.get_player(name)
    if player is None:
        raise HTTPException(status_code=404, detail=f"Player not found: {name}")
    return player


def _to_out(state: GameState, player_name: str) -> PlayerOut:
    """Convert a player from the current state to API output."""
    player = get_player_or_404(state, player_name)
    return player_state_to_out(player, state.get_script())


def compute_death_cleared_effects(state: GameState, player_name: str) -> tuple[tuple[str, str], ...]:
    """Compute cascading status effect removals when a player dies."""
    player = state.get_player(player_name)
    if player is None:
        return ()

    effects_to_remove = CHARACTER_PERSISTENT_EFFECTS.get(player.character_name, [])
    if not effects_to_remove:
        return ()

    cleared: list[tuple[str, str]] = []
    for p in state.players:
        for effect in effects_to_remove:
            if effect in p.status_effects:
                cleared.append((p.name, effect))
    return tuple(cleared)


router = APIRouter(prefix="/players", tags=["Players"])


class AddPlayerRequest(BaseModel):
    """Request to add a player to the game."""

    name: str = Field(
        ...,
        min_length=1,
        max_length=50,
        pattern=r"^[a-zA-Z0-9\s\-_]+$",
        description="Player's display name (alphanumeric, spaces, hyphens, underscores)",
        examples=["Alice", "Bob the Builder", "Player-1"],
    )


@router.post(
    "/add",
    responses={
        400: {"description": "No roles available to assign"},
        409: {"description": "Player with this name already exists"},
    },
)
async def add_player(req: AddPlayerRequest) -> PlayerOut:
    """Add a player to the current game with a randomly assigned role."""
    state = await game_manager.get_state()

    if state.get_player(req.name) is not None:
        raise HTTPException(status_code=409, detail=f"Player with name {req.name} already exists.")
    if not state.included_role_names:
        raise HTTPException(status_code=400, detail="No roles to assign")

    # Resolve randomness BEFORE creating the event
    chosen_name = secrets.choice(state.included_role_names)
    script = state.get_script()
    character = script.get_character(chosen_name)
    if character is None:
        raise HTTPException(status_code=400, detail=f"Character not found: {chosen_name}")

    new_state = await game_manager.dispatch(
        PlayerAdded(
            player_name=req.name,
            character_name=character.name,
            alignment=character.alignment.value,
        )
    )
    return _to_out(new_state, req.name)


class AddTravelerRequest(BaseModel):
    """Request to add a player to the game as a traveler."""

    name: str = Field(
        ...,
        description="Player's display name",
        examples=["Wandering Traveler"],
    )
    traveler: str = Field(
        ...,
        description="Name of the traveler character to assign",
        examples=["Beggar", "Thief", "Gunslinger"],
    )


@router.post(
    "/add_traveler",
    responses={
        400: {"description": "Invalid traveler or validation error"},
        404: {"description": "Traveler character not found"},
        409: {"description": "Player with this name already exists"},
    },
)
async def add_player_as_traveler(req: AddTravelerRequest) -> PlayerOut:
    """Add a player to the current game as a specific traveler character."""
    state = await game_manager.get_state()

    if state.get_player(req.name) is not None:
        raise HTTPException(status_code=409, detail=f"Player with name {req.name} already exists.")

    # Check traveler is valid and unclaimed
    unclaimed = state.get_unclaimed_travelers()
    traveler = next((t for t in unclaimed if t.name == req.traveler), None)
    if traveler is None:
        raise HTTPException(status_code=404, detail=f"Traveler not found or in game: {req.traveler}")

    new_state = await game_manager.dispatch(
        TravelerAdded(
            player_name=req.name,
            traveler_name=traveler.name,
            alignment=traveler.alignment.value,
        )
    )
    return _to_out(new_state, req.name)


@router.get("/list")
async def list_players() -> list[PlayerOut]:
    """List the players in the current game."""
    state = await game_manager.get_state()
    script = state.get_script()
    return [player_state_to_out(p, script) for p in state.players]


@router.get(
    "/name/{name}",
    responses={
        404: {"description": "Player not found"},
        408: {"description": "Role reveal timed out - roles not set to be revealed"},
    },
)
async def get_player_role(
    name: str = Path(..., description="Player's name", examples=["Alice"]),
) -> PlayerOut:
    """Get the role of a player in the current game (waits for role reveal if needed)."""
    # Check player exists
    state = await game_manager.get_state()
    get_player_or_404(state, name)

    # Poll for role reveal
    polling_attempts = 0
    should_reveal = False
    while not should_reveal and polling_attempts < ROLE_REVEAL_TIMEOUT_ATTEMPTS:
        state = await game_manager.get_state()
        should_reveal = state.should_reveal_roles
        if not should_reveal:
            await anyio.sleep(POLLING_INTERVAL_SECONDS)
            polling_attempts += 1

    if polling_attempts >= ROLE_REVEAL_TIMEOUT_ATTEMPTS:
        timeout_seconds = ROLE_REVEAL_TIMEOUT_ATTEMPTS * POLLING_INTERVAL_SECONDS
        raise HTTPException(
            status_code=408,
            detail=f"Role reveal timed out after {timeout_seconds}s. Check if roles are set to be revealed.",
        )

    state = await game_manager.get_state()
    return _to_out(state, name)


class SetPlayerAliveRequest(BaseModel):
    """Request to set the alive status of a player in the game."""

    name: str = Field(..., description="Player's name", examples=["Alice"])
    is_alive: bool = Field(
        ...,
        description="Whether the player is alive (true) or dead (false)",
        examples=[False],
    )


@router.post("/set_alive", responses={404: {"description": "Player not found"}})
async def set_player_alive(req: SetPlayerAliveRequest) -> PlayerOut:
    """Set the alive status of a player (automatically clears status effects on death)."""
    state = await game_manager.get_state()
    player = get_player_or_404(state, req.name)

    # Compute cascading effects if the player is dying
    cleared_effects: tuple[tuple[str, str], ...] = ()
    if player.is_alive and not req.is_alive:
        cleared_effects = compute_death_cleared_effects(state, req.name)

    new_state = await game_manager.dispatch(
        PlayerAliveSet(
            player_name=req.name,
            is_alive=req.is_alive,
            cleared_effects=cleared_effects,
        )
    )
    return _to_out(new_state, req.name)


class SetPlayerHasUsedDeadVoteRequest(BaseModel):
    """Request to set whether a dead player has used their single vote."""

    name: str = Field(..., description="Player's name", examples=["Alice"])
    has_used_dead_vote: bool = Field(
        ...,
        description="Whether the dead player has used their one vote",
        examples=[True],
    )


@router.post("/set_has_used_dead_vote", responses={404: {"description": "Player not found"}})
async def set_player_has_used_dead_vote(req: SetPlayerHasUsedDeadVoteRequest) -> PlayerOut:
    """Mark whether a dead player has used their single vote."""
    state = await game_manager.get_state()
    get_player_or_404(state, req.name)

    new_state = await game_manager.dispatch(
        DeadVoteUsedSet(player_name=req.name, has_used_dead_vote=req.has_used_dead_vote)
    )
    return _to_out(new_state, req.name)


class SetPlayerAlignmentRequest(BaseModel):
    """Request to set the alignment of a player in the game."""

    name: str = Field(..., description="Player's name", examples=["Alice"])
    alignment: Alignment = Field(
        ...,
        description="Player's alignment (good, evil, or unknown for travelers)",
        examples=["evil"],
    )


@router.post("/set_alignment", responses={404: {"description": "Player not found"}})
async def set_player_alignment(req: SetPlayerAlignmentRequest) -> PlayerOut:
    """Set the alignment of a player (good, evil, or unknown for travelers)."""
    state = await game_manager.get_state()
    get_player_or_404(state, req.name)

    new_state = await game_manager.dispatch(PlayerAlignmentSet(player_name=req.name, alignment=req.alignment.value))
    return _to_out(new_state, req.name)


class SwapCharacterRequest(BaseModel):
    """Request to swap the characters of two players in the game."""

    name1: str = Field(..., description="First player's name", examples=["Alice"])
    name2: str = Field(..., description="Second player's name", examples=["Bob"])


class SwapCharacterResponse(BaseModel):
    """Response after swapping characters between two players."""

    status: str = Field(..., description="Operation status", examples=["success"])
    player1: PlayerOut = Field(..., description="First player with their new character")
    player2: PlayerOut = Field(..., description="Second player with their new character")


@router.post("/swap_character", responses={404: {"description": "One or both players not found"}})
async def swap_character(req: SwapCharacterRequest) -> SwapCharacterResponse:
    """Swap the characters between two players in the current game."""
    state = await game_manager.get_state()
    get_player_or_404(state, req.name1)
    get_player_or_404(state, req.name2)

    new_state = await game_manager.dispatch(CharactersSwapped(name1=req.name1, name2=req.name2))
    return SwapCharacterResponse(
        status="success",
        player1=_to_out(new_state, req.name1),
        player2=_to_out(new_state, req.name2),
    )


class RenamePlayerRequest(BaseModel):
    """Request to rename a player in the game."""

    name: str = Field(..., description="Current player name", examples=["Alice"])
    new_name: str = Field(..., description="New player name", examples=["Alice Johnson"])


@router.post("/rename", responses={404: {"description": "Player not found"}})
async def rename_player(req: RenamePlayerRequest) -> PlayerOut:
    """Rename a player in the current game."""
    state = await game_manager.get_state()
    get_player_or_404(state, req.name)

    new_state = await game_manager.dispatch(PlayerRenamed(old_name=req.name, new_name=req.new_name))
    return _to_out(new_state, req.new_name)


class RemovePlayerRequest(BaseModel):
    """Request to remove a player from the game."""

    name: str = Field(..., description="Player's name to remove", examples=["Alice"])


class RemovePlayerResponse(BaseModel):
    """Response after removing a player from the game."""

    status: str = Field(..., description="Operation status", examples=["success"])
    remaining_players: list[str] = Field(
        ..., description="Names of players still in the game", examples=[["Bob", "Charlie"]]
    )


@router.post("/remove", responses={404: {"description": "Player not found"}})
async def remove_player(req: RemovePlayerRequest) -> RemovePlayerResponse:
    """Remove a player from the current game."""
    state = await game_manager.get_state()
    get_player_or_404(state, req.name)

    new_state = await game_manager.dispatch(PlayerRemoved(player_name=req.name))
    return RemovePlayerResponse(
        status="success",
        remaining_players=[p.name for p in new_state.players],
    )


@router.get("/names")
async def get_game_players_names() -> list[str]:
    """Return the names of the players in the current game."""
    state = await game_manager.get_state()
    return [p.name for p in state.players]


@router.get("/visibility")
async def get_roles_visibility() -> bool:
    """Get the visibility of the roles for the current game."""
    state = await game_manager.get_state()
    return state.should_reveal_roles


class SetRolesVisibilityRequest(BaseModel):
    """Request to set the visibility of the roles for the current game."""

    should_reveal_roles: bool = Field(
        ...,
        description="Whether to reveal player roles to the storyteller immediately",
        examples=[True],
    )


@router.post("/set_visibility")
async def set_roles_visibility(req: SetRolesVisibilityRequest) -> bool:
    """Set the visibility of the roles for the current game."""
    new_state = await game_manager.dispatch(RoleVisibilitySet(should_reveal_roles=req.should_reveal_roles))
    return new_state.should_reveal_roles


class AddStatusEffectRequest(BaseModel):
    """Request to add a status effect to a player in the game."""

    name: str = Field(..., description="Player's name", examples=["Alice"])
    status_effect: str = Field(
        ...,
        description="Status effect to add (e.g., Poisoned, Safe, Drunk)",
        examples=["Poisoned", "Safe", "Drunk"],
    )


@router.post("/add_status_effect", responses={404: {"description": "Player not found"}})
async def add_status_effect(req: AddStatusEffectRequest) -> PlayerOut:
    """Add a status effect to a player (e.g., Poisoned, Safe, Drunk)."""
    state = await game_manager.get_state()
    get_player_or_404(state, req.name)

    new_state = await game_manager.dispatch(StatusEffectAdded(player_name=req.name, effect=req.status_effect))
    return _to_out(new_state, req.name)


class RemoveStatusEffectRequest(BaseModel):
    """Request to remove a status effect from a player in the game."""

    name: str = Field(..., description="Player's name", examples=["Alice"])
    status_effect: str = Field(
        ...,
        description="Status effect to remove",
        examples=["Poisoned", "Safe"],
    )


@router.post("/remove_status_effect", responses={404: {"description": "Player not found"}})
async def remove_status_effect(req: RemoveStatusEffectRequest) -> PlayerOut:
    """Remove a status effect from a player."""
    state = await game_manager.get_state()
    get_player_or_404(state, req.name)

    new_state = await game_manager.dispatch(StatusEffectRemoved(player_name=req.name, effect=req.status_effect))
    return _to_out(new_state, req.name)
