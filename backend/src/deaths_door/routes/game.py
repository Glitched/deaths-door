from asyncio import sleep

from fastapi import APIRouter
from fastapi.exceptions import HTTPException
from pydantic import BaseModel

from ..game import Game
from ..script import ScriptName

router = APIRouter()

# Initialize a sample game state for debugging purposes
game = Game.get_sample_game()

should_reveal_roles = True


class NewGameRequest(BaseModel):
    """Request to add a player to the game."""

    script_name: str


@router.post("/game/new")
async def new_game(req: NewGameRequest):
    """Start a new game."""
    global game

    script_name = ScriptName.from_str(req.script_name)
    if script_name is None:
        raise HTTPException(status_code=404, detail="Script not found")

    game = Game(script_name)


@router.get("/game/script/name")
async def get_game_script():
    """Return the name of the script for the current game."""
    global game
    return game.script.name.value


@router.get("/game/script/roles")
async def get_game_script_roles():
    """Return the name of the script for the current game."""
    global game
    return [c.to_out() for c in game.script.characters]


@router.get("/game/players/names")
async def get_game_players_names():
    """Return the names of the players in the current game."""
    global game
    return [player.name for player in game.players]


@router.get("/game/roles/list")
async def get_game_roles():
    """List the names of roles present in the current game."""
    global game
    return game.included_roles


class AddRoleRequest(BaseModel):
    """Request to add a role to the game."""

    name: str


@router.post("/game/roles/add")
async def add_role(req: AddRoleRequest):
    """Add a role to the current game."""
    global game

    game.include_role(req.name)


class AddRoleMultiRequest(BaseModel):
    """Request to add multiple roles to the game."""

    names: list[str]


@router.post("/game/roles/add/multi")
async def add_role_multi(req: AddRoleMultiRequest):
    """Add multiple roles to the current game."""
    global game

    for name in req.names:
        game.include_role(name)


class RemoveRoleRequest(BaseModel):
    """Request to remove a role from the game."""

    name: str


@router.post("/game/roles/remove")
async def remove_role(req: RemoveRoleRequest):
    """Remove a role from the current game."""
    global game

    game.remove_role(req.name)


@router.get("/game/roles/visibility")
async def get_roles_visibility():
    """Get the visibility of the roles for the current game."""
    global should_reveal_roles
    return should_reveal_roles


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


class AddPlayerRequest(BaseModel):
    """Request to add a player to the game."""

    name: str


@router.post("/game/players/add")
async def add_player(req: AddPlayerRequest):
    """Add a player to the current game."""
    global game, should_reveal_roles
    try:
        player = game.add_player_with_random_role(req.name)
    except ValueError as e:
        raise HTTPException(status_code=404, detail=e.args) from e

    return player.to_out()


@router.get("/game/players/list")
async def list_players():
    """List the players in the current game."""
    global game

    return [player.to_out() for player in game.players]


@router.get("/game/players/name/{name}")
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
