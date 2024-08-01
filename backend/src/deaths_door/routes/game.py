from fastapi import APIRouter
from fastapi.exceptions import HTTPException
from pydantic import BaseModel

from ..game import Game
from ..script import ScriptName

router = APIRouter(prefix="/game")

# Initialize a sample game state for debugging purposes
game = Game.get_sample_game()


class NewGameRequest(BaseModel):
    """Request to add a player to the game."""

    script_name: str


@router.post("/new")
async def new_game(req: NewGameRequest):
    """Start a new game."""
    global game

    script_name = ScriptName.from_str(req.script_name)
    if script_name is None:
        raise HTTPException(status_code=404, detail="Script not found")

    game = Game(script_name)


@router.get("/script/name")
async def get_game_script():
    """Return the name of the script for the current game."""
    global game
    return game.script.name.value


@router.get("/script/roles")
async def get_game_script_roles():
    """Return the name of the script for the current game."""
    global game
    return [c.to_out() for c in game.script.characters]
