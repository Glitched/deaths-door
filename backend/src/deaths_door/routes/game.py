from asyncio import sleep

from fastapi import APIRouter
from fastapi.exceptions import HTTPException
from pydantic import BaseModel

from ..game import Game
from ..script import ScriptName

router = APIRouter()

# Initialize a sample game state for debugging purposes
game = Game.get_sample_game()

should_reveal_roles = False


@router.get("/game/new/{str_script_name}")
async def new_game(str_script_name: str):
    """Start a new game."""
    global game

    script_name = ScriptName.from_str(str_script_name)
    if script_name is None:
        raise HTTPException(status_code=404, detail="Script not found")

    game = Game(script_name)


@router.get("/game/roles")
async def get_game_roles():
    """List the names of roles present in the current game."""
    global game
    return game.included_roles


@router.get("/game/script")
async def get_game_script():
    """Return the name of the script for the current game."""
    global game
    return game.script.name.value


@router.get("/game/players/names")
async def get_game_players_names():
    """Return the names of the players in the current game."""
    global game
    return [player.name for player in game.players]


class AddPlayerRequest(BaseModel):
    """Request to add a player to the game."""

    name: str


@router.post("/game/add_player")
async def add_player(req: AddPlayerRequest):
    """Add a player to the current game."""
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
        player = game.add_player_with_random_role(req.name)
    except ValueError as e:
        raise HTTPException(status_code=404, detail=e.args) from e

    return player.character.to_out()


@router.get("/game/roles/reveal")
async def reveal_roles():
    """Reveal the roles for the current game."""
    global should_reveal_roles

    should_reveal_roles = True
    return should_reveal_roles


@router.get("/game/roles/hide")
async def hide_roles():
    """Hide the roles for the current game."""
    global should_reveal_roles

    should_reveal_roles = False
    return should_reveal_roles


@router.get("/game/remove_roll/{role_name}")
async def remove_role(role_name: str):
    """Sample API endpoint."""
    global game

    did_remove = game.remove_role(role_name)
    if not did_remove:
        raise HTTPException(status_code=404, detail="Role not in script")
