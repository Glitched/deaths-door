import asyncio
from contextlib import AbstractAsyncContextManager

from fastapi import APIRouter, Depends, Path
from fastapi.exceptions import HTTPException
from pydantic import BaseModel, Field

from ..alignment import Alignment
from ..game import Game
from ..game_manager import get_current_game
from ..player import Player, PlayerOut


class RemovePlayerResponse(BaseModel):
    """Response after removing a player from the game."""

    status: str = Field(..., description="Operation status", examples=["success"])
    remaining_players: list[str] = Field(
        ..., description="Names of players still in the game", examples=[["Bob", "Charlie"]]
    )


def get_player_or_404(game: Game, name: str) -> Player:
    """Get player by name or raise 404 error."""
    player = game.get_player_by_name(name)
    if player is None:
        raise HTTPException(status_code=404, detail=f"Player not found: {name}")
    return player


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
async def add_player(
    req: AddPlayerRequest,
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> PlayerOut:
    """Add a player to the current game with a randomly assigned role."""
    async with game_ctx as game:
        try:
            existing_player = game.get_player_by_name(req.name)
            if existing_player:
                raise HTTPException(
                    status_code=409,
                    detail=f"Player with name {req.name} already exists.",
                )
            player = game.add_player_with_random_role(req.name)
        except ValueError as e:
            # No roles to assign, role not found, etc. - these are 400 Bad Request
            raise HTTPException(status_code=400, detail=str(e)) from e

        return player.to_out()


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
async def add_player_as_traveler(
    req: AddTravelerRequest, game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)
) -> PlayerOut:
    """Add a player to the current game as a specific traveler character."""
    async with game_ctx as game:
        try:
            # Check if the player already exists
            existing_player = game.get_player_by_name(req.name)
            if existing_player:
                raise HTTPException(
                    status_code=409,
                    detail=f"Player with name {req.name} already exists.",
                )
            player = game.add_player_as_traveler(req.name, req.traveler)
        except ValueError as e:
            # Traveler not found - this is 404 Not Found
            if "not found" in str(e).lower():
                raise HTTPException(status_code=404, detail=str(e)) from e
            # Other validation errors are 400 Bad Request
            raise HTTPException(status_code=400, detail=str(e)) from e

        return player.to_out()


@router.get("/list")
async def list_players(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> list[PlayerOut]:
    """List the players in the current game."""
    async with game_ctx as game:
        return [player.to_out() for player in game.players]


@router.get(
    "/name/{name}",
    responses={
        404: {"description": "Player not found"},
        408: {"description": "Role reveal timed out - roles not set to be revealed"},
    },
)
async def get_player_role(
    name: str = Path(..., description="Player's name", examples=["Alice"]),
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> PlayerOut:
    """Get the role of a player in the current game (waits for role reveal if needed)."""
    async with game_ctx as game:
        # Check if player exists first
        player = get_player_or_404(game, name)

        ROLE_REVEAL_TIMEOUT_ATTEMPTS = 100
        POLLING_INTERVAL_SECONDS = 0.1

        polling_attempts = 0
        while (
            not game.should_reveal_roles
            and polling_attempts < ROLE_REVEAL_TIMEOUT_ATTEMPTS
        ):
            await asyncio.sleep(POLLING_INTERVAL_SECONDS)
            polling_attempts += 1

        if polling_attempts >= ROLE_REVEAL_TIMEOUT_ATTEMPTS:
            timeout_seconds = ROLE_REVEAL_TIMEOUT_ATTEMPTS * POLLING_INTERVAL_SECONDS
            raise HTTPException(
                status_code=408,
                detail=(
                    f"Role reveal timed out after {timeout_seconds}s. "
                    "Check if roles are set to be revealed."
                ),
            )

        return player.to_out()


class SetPlayerAliveRequest(BaseModel):
    """Request to set the alive status of a player in the game."""

    name: str = Field(..., description="Player's name", examples=["Alice"])
    is_alive: bool = Field(
        ...,
        description="Whether the player is alive (true) or dead (false)",
        examples=[False],
    )


@router.post("/set_alive", responses={404: {"description": "Player not found"}})
async def set_player_alive(
    req: SetPlayerAliveRequest, game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)
) -> PlayerOut:
    """Set the alive status of a player (automatically clears status effects on death)."""
    async with game_ctx as game:
        try:
            game.set_player_alive_status(req.name, req.is_alive)
            player = game.get_player_by_name(req.name)
            return player.to_out()
        except ValueError as e:
            raise HTTPException(status_code=404, detail=str(e)) from e


class SetPlayerHasUsedDeadVoteRequest(BaseModel):
    """Request to set whether a dead player has used their single vote."""

    name: str = Field(..., description="Player's name", examples=["Alice"])
    has_used_dead_vote: bool = Field(
        ...,
        description="Whether the dead player has used their one vote",
        examples=[True],
    )


@router.post("/set_has_used_dead_vote", responses={404: {"description": "Player not found"}})
async def set_player_has_used_dead_vote(
    req: SetPlayerHasUsedDeadVoteRequest,
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> PlayerOut:
    """Mark whether a dead player has used their single vote."""
    async with game_ctx as game:
        player = game.get_player_by_name(req.name)
        if player is None:
            raise HTTPException(status_code=404, detail=f"Player not found: {req.name}")

        player.set_has_used_dead_vote(req.has_used_dead_vote)
        return player.to_out()


class SetPlayerAlignmentRequest(BaseModel):
    """Request to set the alignment of a player in the game."""

    name: str = Field(..., description="Player's name", examples=["Alice"])
    alignment: Alignment = Field(
        ...,
        description="Player's alignment (good, evil, or unknown for travelers)",
        examples=["evil"],
    )


@router.post("/set_alignment", responses={404: {"description": "Player not found"}})
async def set_player_alignment(
    req: SetPlayerAlignmentRequest,
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> PlayerOut:
    """Set the alignment of a player (good, evil, or unknown for travelers)."""
    async with game_ctx as game:
        player = game.get_player_by_name(req.name)
        if player is None:
            raise HTTPException(status_code=404, detail=f"Player not found: {req.name}")

        player.set_alignment(req.alignment)
        return player.to_out()


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
async def swap_character(
    req: SwapCharacterRequest, game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)
) -> SwapCharacterResponse:
    """Swap the characters between two players in the current game."""
    async with game_ctx as game:
        player1 = game.get_player_by_name(req.name1)
        if player1 is None:
            raise HTTPException(
                status_code=404, detail=f"Player not found: {req.name1}"
            )
        character1 = player1.character

        player2 = game.get_player_by_name(req.name2)
        if player2 is None:
            raise HTTPException(
                status_code=404, detail=f"Player not found: {req.name2}"
            )
        character2 = player2.character

        player1.set_character(character2)
        player2.set_character(character1)

        return SwapCharacterResponse(
            status="success",
            player1=player1.to_out(),
            player2=player2.to_out(),
        )


class RenamePlayerRequest(BaseModel):
    """Request to rename a player in the game."""

    name: str = Field(..., description="Current player name", examples=["Alice"])
    new_name: str = Field(
        ..., description="New player name", examples=["Alice Johnson"]
    )


@router.post("/rename", responses={404: {"description": "Player not found"}})
async def rename_player(
    req: RenamePlayerRequest, game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)
) -> PlayerOut:
    """Rename a player in the current game."""
    async with game_ctx as game:
        player = game.get_player_by_name(req.name)
        if player is None:
            raise HTTPException(status_code=404, detail=f"Player not found: {req.name}")

        player.set_name(req.new_name)
        return player.to_out()


class RemovePlayerRequest(BaseModel):
    """Request to remove a player from the game."""

    name: str = Field(..., description="Player's name to remove", examples=["Alice"])


@router.post("/remove", responses={404: {"description": "Player not found"}})
async def remove_player(
    req: RemovePlayerRequest, game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game)
) -> RemovePlayerResponse:
    """Remove a player from the current game."""
    async with game_ctx as game:
        try:
            game.remove_player_by_name(req.name)
            return RemovePlayerResponse(
                status="success", remaining_players=[p.name for p in game.players]
            )
        except ValueError as e:
            raise HTTPException(status_code=404, detail=str(e)) from e


@router.get("/names")
async def get_game_players_names(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> list[str]:
    """Return the names of the players in the current game."""
    async with game_ctx as game:
        return [player.name for player in game.players]


@router.get("/visibility")
async def get_roles_visibility(
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> bool:
    """Get the visibility of the roles for the current game."""
    async with game_ctx as game:
        return game.should_reveal_roles


class SetRolesVisibilityRequest(BaseModel):
    """Request to set the visibility of the roles for the current game."""

    should_reveal_roles: bool = Field(
        ...,
        description="Whether to reveal player roles to the storyteller immediately",
        examples=[True],
    )


@router.post("/set_visibility")
async def set_roles_visibility(
    req: SetRolesVisibilityRequest,
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> bool:
    """Set the visibility of the roles for the current game."""
    async with game_ctx as game:
        game.should_reveal_roles = req.should_reveal_roles
        return game.should_reveal_roles


class AddStatusEffectRequest(BaseModel):
    """Request to add a status effect to a player in the game."""

    name: str = Field(..., description="Player's name", examples=["Alice"])
    status_effect: str = Field(
        ...,
        description="Status effect to add (e.g., Poisoned, Safe, Drunk)",
        examples=["Poisoned", "Safe", "Drunk"],
    )


@router.post("/add_status_effect", responses={404: {"description": "Player not found"}})
async def add_status_effect(
    req: AddStatusEffectRequest,
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> PlayerOut:
    """Add a status effect to a player (e.g., Poisoned, Safe, Drunk)."""
    async with game_ctx as game:
        player = game.get_player_by_name(req.name)
        if player is None:
            raise HTTPException(status_code=404, detail=f"Player not found: {req.name}")

        player.add_status_effect(req.status_effect)
        return player.to_out()


class RemoveStatusEffectRequest(BaseModel):
    """Request to remove a status effect from a player in the game."""

    name: str = Field(..., description="Player's name", examples=["Alice"])
    status_effect: str = Field(
        ...,
        description="Status effect to remove",
        examples=["Poisoned", "Safe"],
    )


@router.post("/remove_status_effect", responses={404: {"description": "Player not found"}})
async def remove_status_effect(
    req: RemoveStatusEffectRequest,
    game_ctx: AbstractAsyncContextManager[Game] = Depends(get_current_game),
) -> PlayerOut:
    """Remove a status effect from a player."""
    async with game_ctx as game:
        player = game.get_player_by_name(req.name)
        if player is None:
            raise HTTPException(status_code=404, detail=f"Player not found: {req.name}")

        player.remove_status_effect(req.status_effect)
        return player.to_out()
