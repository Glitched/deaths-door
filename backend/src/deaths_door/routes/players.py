from asyncio import sleep

from fastapi import APIRouter
from fastapi.exceptions import HTTPException
from pydantic import BaseModel

from ..alignment import Alignment
from ..game import Game

router = APIRouter()

# Initialize a sample game state for debugging purposes
game = Game.get_sample_game()

should_reveal_roles = True


class AddPlayerRequest(BaseModel):
    """Request to add a player to the game."""

    name: str


@router.post("/players/add")
async def add_player(req: AddPlayerRequest):
    """Add a player to the current game."""
    global game, should_reveal_roles
    try:
        player = game.add_player_with_random_role(req.name)
    except ValueError as e:
        raise HTTPException(status_code=404, detail=e.args) from e

    return player.to_out()


@router.get("/players/list")
async def list_players():
    """List the players in the current game."""
    global game

    return [player.to_out() for player in game.players]


@router.get("/players/name/{name}")
async def get_player_role(name: str):
    """Get the role of a player in the current game."""
    global game, should_reveal_roles
    count = 0
    while not should_reveal_roles and count < 100:
        await sleep(0.1)
        count += 1

    if count >= 100:
        raise HTTPException(
            status_code=408, detail="Timed out waiting for role assignment."
        )

    try:
        player = game.get_player_by_name(name)
    except ValueError as e:
        raise HTTPException(status_code=404, detail=e.args) from e

    return player.to_out()


class SetPlayerAliveRequest(BaseModel):
    """Request to set the alive status of a player in the game."""

    name: str
    is_alive: bool


@router.post("/players/set_alive")
async def set_player_alive(req: SetPlayerAliveRequest):
    """Set the alive status of a player in the current game."""
    global game

    player = game.get_player_by_name(req.name)
    player.set_is_alive(req.is_alive)
    return player.to_out()


class SetPlayerHasUsedDeadVoteRequest(BaseModel):
    """Request to set the has used dead vote status of a player in the game."""

    name: str
    has_used_dead_vote: bool


@router.post("/players/set_has_used_dead_vote")
async def set_player_has_used_dead_vote(req: SetPlayerHasUsedDeadVoteRequest):
    """Set the has used dead vote status of a player in the current game."""
    global game

    player = game.get_player_by_name(req.name)
    player.set_has_used_dead_vote(req.has_used_dead_vote)
    return player.to_out()


class SetPlayerAlignmentRequest(BaseModel):
    """Request to set the alignment of a player in the game."""

    name: str
    alignment: Alignment


@router.post("/players/set_alignment")
async def set_player_alignment(req: SetPlayerAlignmentRequest):
    """Set the alignment of a player in the current game."""
    global game

    player = game.get_player_by_name(req.name)
    player.set_alignment(req.alignment)
    return player.to_out()


class SwapCharacterRequest(BaseModel):
    """Request to swap the characters of two players in the game."""

    name1: str
    name2: str


@router.post("/players/swap_character")
async def swap_character(req: SwapCharacterRequest):
    """Swap the characters of two players in the current game."""
    global game

    player1 = game.get_player_by_name(req.name1)
    character1 = player1.character
    player2 = game.get_player_by_name(req.name2)
    character2 = player2.character

    player1.set_character(character2)
    player2.set_character(character1)

    return player1.to_out()
