from asyncio import sleep

from fastapi import APIRouter, Depends
from fastapi.exceptions import HTTPException
from pydantic import BaseModel

from ..alignment import Alignment
from ..game import Game
from ..game_manager import get_current_game

router = APIRouter(prefix="/players")


class AddPlayerRequest(BaseModel):
    """Request to add a player to the game."""

    name: str


@router.post("/add")
async def add_player(req: AddPlayerRequest, game: Game = Depends(get_current_game)):
    """Add a player to the current game."""
    try:
        existing_player = game.get_player_by_name(req.name)
        if existing_player:
            raise ValueError(f"Player with name {req.name} already exists.")
        player = game.add_player_with_random_role(req.name)
    except ValueError as e:
        raise HTTPException(status_code=404, detail=e.args) from e

    return player.to_out()


@router.get("/list")
async def list_players(game: Game = Depends(get_current_game)):
    """List the players in the current game."""
    return [player.to_out() for player in game.players]


@router.get("/name/{name}")
async def get_player_role(name: str, game: Game = Depends(get_current_game)):
    """Get the role of a player in the current game."""
    count = 0
    while not game.get_should_reveal_roles() and count < 100:
        await sleep(0.1)
        count += 1

    if count >= 100:
        raise HTTPException(
            status_code=408, detail="Timed out waiting for role assignment."
        )

    player = game.get_player_by_name(name)
    if player is None:
        raise HTTPException(status_code=404, detail=f"Player not found: {name}")

    return player.to_out()


class SetPlayerAliveRequest(BaseModel):
    """Request to set the alive status of a player in the game."""

    name: str
    is_alive: bool


@router.post("/set_alive")
async def set_player_alive(
    req: SetPlayerAliveRequest, game: Game = Depends(get_current_game)
):
    """Set the alive status of a player in the current game."""
    player = game.get_player_by_name(req.name)
    if player is None:
        raise HTTPException(status_code=404, detail=f"Player not found: {req.name}")

    player.set_is_alive(req.is_alive)
    return player.to_out()


class SetPlayerHasUsedDeadVoteRequest(BaseModel):
    """Request to set the has used dead vote status of a player in the game."""

    name: str
    has_used_dead_vote: bool


@router.post("/set_has_used_dead_vote")
async def set_player_has_used_dead_vote(
    req: SetPlayerHasUsedDeadVoteRequest, game: Game = Depends(get_current_game)
):
    """Set the has used dead vote status of a player in the current game."""
    player = game.get_player_by_name(req.name)
    if player is None:
        raise HTTPException(status_code=404, detail=f"Player not found: {req.name}")

    player.set_has_used_dead_vote(req.has_used_dead_vote)
    return player.to_out()


class SetPlayerAlignmentRequest(BaseModel):
    """Request to set the alignment of a player in the game."""

    name: str
    alignment: Alignment


@router.post("/set_alignment")
async def set_player_alignment(
    req: SetPlayerAlignmentRequest, game: Game = Depends(get_current_game)
):
    """Set the alignment of a player in the current game."""
    player = game.get_player_by_name(req.name)
    if player is None:
        raise HTTPException(status_code=404, detail=f"Player not found: {req.name}")

    player.set_alignment(req.alignment)
    return player.to_out()


class SwapCharacterRequest(BaseModel):
    """Request to swap the characters of two players in the game."""

    name1: str
    name2: str


@router.post("/swap_character")
async def swap_character(
    req: SwapCharacterRequest, game: Game = Depends(get_current_game)
):
    """Swap the characters of two players in the current game."""
    player1 = game.get_player_by_name(req.name1)
    if player1 is None:
        raise HTTPException(status_code=404, detail=f"Player not found: {req.name1}")
    character1 = player1.character

    player2 = game.get_player_by_name(req.name2)
    if player2 is None:
        raise HTTPException(status_code=404, detail=f"Player not found: {req.name2}")
    character2 = player2.character

    player1.set_character(character2)
    player2.set_character(character1)

    return player1.to_out()


class RenamePlayerRequest(BaseModel):
    """Request to rename a player in the game."""

    name: str
    new_name: str


@router.post("/rename")
async def rename_player(
    req: RenamePlayerRequest, game: Game = Depends(get_current_game)
):
    """Rename a player in the current game."""
    player = game.get_player_by_name(req.name)
    if player is None:
        raise HTTPException(status_code=404, detail=f"Player not found: {req.name}")

    player.set_name(req.new_name)
    return player.to_out()


class RemovePlayerRequest(BaseModel):
    """Request to remove a player from the game."""

    name: str


@router.post("/remove")
async def remove_player(
    req: RemovePlayerRequest, game: Game = Depends(get_current_game)
):
    """Remove a player from the current game."""
    game.remove_player_by_name(req.name)


@router.get("/names")
async def get_game_players_names(game: Game = Depends(get_current_game)):
    """Return the names of the players in the current game."""
    return [player.name for player in game.players]


@router.get("/visibility")
async def get_roles_visibility(game: Game = Depends(get_current_game)):
    """Get the visibility of the roles for the current game."""
    return game.get_should_reveal_roles()


class SetRolesVisibilityRequest(BaseModel):
    """Request to set the visibility of the roles for the current game."""

    should_reveal_roles: bool


@router.post("/set_visibility")
async def set_roles_visibility(
    req: SetRolesVisibilityRequest, game: Game = Depends(get_current_game)
):
    """Set the visibility of the roles for the current game."""
    return game.set_should_reveal_roles(req.should_reveal_roles)


class AddStatusEffectRequest(BaseModel):
    """Request to add a status effect to a player in the game."""

    name: str
    status_effect: str


@router.post("/add_status_effect")
async def add_status_effect(
    req: AddStatusEffectRequest, game: Game = Depends(get_current_game)
):
    """Add a status effect to a player in the current game."""
    player = game.get_player_by_name(req.name)
    if player is None:
        raise HTTPException(status_code=404, detail=f"Player not found: {req.name}")

    player.add_status_effect(req.status_effect)
    return player.to_out()


class RemoveStatusEffectRequest(BaseModel):
    """Request to remove a status effect from a player in the game."""

    name: str
    status_effect: str


@router.post("/remove_status_effect")
async def remove_status_effect(
    req: RemoveStatusEffectRequest, game: Game = Depends(get_current_game)
):
    """Remove a status effect from a player in the current game."""
    player = game.get_player_by_name(req.name)
    if player is None:
        raise HTTPException(status_code=404, detail=f"Player not found: {req.name}")

    player.remove_status_effect(req.status_effect)
    return player.to_out()
