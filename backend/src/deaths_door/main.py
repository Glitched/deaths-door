from fastapi import FastAPI
from fastapi.exceptions import HTTPException

from .game import Game
from .script import Script, ScriptName
from .sound_fx import SoundFX

app = FastAPI()

game: None | Game = Game(7, ScriptName.TROUBLE_BREWING)


@app.get("/script/list")
async def read_scripts():
    """Return a list of available scripts."""
    return {x: str(x) for x in list(ScriptName)}


@app.get("/script/{script_name}/role")
async def read_roles(script_name: str):
    """List the roles for the given script."""
    script = Script.from_str(script_name)
    if script is None:
        raise HTTPException(status_code=404, detail="Script not found")

    return script.roles


# TODO: Can we consolidate this into the method above?
# I don't have internet so I can't check the docs.
@app.get("/script/{script}/role/{name}")
async def read_role(script_name: str, role_name: str):
    """Get a given role for a script."""
    script = Script.from_str(script_name)
    if script is None:
        raise HTTPException(status_code=404, detail="Script not found")

    return script.get_role(role_name)


@app.get("/game/new/{str_script_name}/{player_count}")
async def new_game(str_script_name: str, player_count: int):
    """Sample API endpoint."""
    global game

    script_name = ScriptName.from_str(str_script_name)
    if script_name is None:
        raise HTTPException(status_code=404, detail="Script not found")

    game = Game(player_count, script_name)


@app.get("/game/roles")
async def get_game_roles():
    """Sample API endpoint."""
    global game
    if game is None:
        raise HTTPException(status_code=404, detail="No game started")

    return game.roles


@app.get("/game/add_role/{role_name}")
async def add_role(role_name: str):
    """Sample API endpoint."""
    global game
    if game is None:
        raise HTTPException(status_code=404, detail="No game started")

    try:
        game.add_role(role_name)
    except ValueError as e:
        raise HTTPException(status_code=404, detail=e.args) from e

    return game.get_free_space()


@app.get("/game/remove_roll/{role_name}")
async def remove_role(role_name: str):
    """Sample API endpoint."""
    global game
    if game is None:
        raise HTTPException(status_code=404, detail="No game started")

    did_remove = game.remove_role(role_name)
    if not did_remove:
        raise HTTPException(status_code=404, detail="Role not in script")


@app.get("/play_sound/{name}")
async def play_sound(name: str):
    """Sample API endpoint."""
    match name:
        case "death":
            SoundFX().death().play()
        case _:
            raise HTTPException(status_code=404, detail="Sound not found")

    return {"ok": "true"}
