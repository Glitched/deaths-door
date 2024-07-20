from fastapi import APIRouter
from fastapi.exceptions import HTTPException

from ..game import Game
from ..script import ScriptName

router = APIRouter()


@router.get("/game/new/{str_script_name}/{player_count}")
async def new_game(str_script_name: str, player_count: int):
    """Start a new game."""
    global game

    script_name = ScriptName.from_str(str_script_name)
    if script_name is None:
        raise HTTPException(status_code=404, detail="Script not found")

    game = Game(player_count, script_name)


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


@router.get("/game/open_slots")
async def open_slots():
    """Add the given role to the current game."""
    global game
    return game.get_open_slots()


@router.get("/game/add_role/{role_name}")
async def add_role(role_name: str):
    """Add the given role to the current game."""
    global game

    try:
        game.add_player_with_role(role_name)
    except ValueError as e:
        raise HTTPException(status_code=404, detail=e.args) from e

    return game.get_open_slots()


@router.get("/game/remove_roll/{role_name}")
async def remove_role(role_name: str):
    """Sample API endpoint."""
    global game

    did_remove = game.remove_role(role_name)
    if not did_remove:
        raise HTTPException(status_code=404, detail="Role not in script")

    return game.get_open_slots()
