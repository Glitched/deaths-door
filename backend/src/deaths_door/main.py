from fastapi import FastAPI
from fastapi.exceptions import HTTPException
from outcome import Value

from .game import Game
from .script import Script, ScriptName
from .sound_fx import SoundFX

app = FastAPI()

game = None


@app.get("/scripts")
async def read_scripts():
    """Return a list of available scripts."""
    return list(ScriptName)


@app.get("script/{script}/roles")
async def read_roles(script_name: str):
    """Sample API endpoint."""
    script = Script.from_str(script_name)
    if script is None:
        raise HTTPException(status_code=404, detail="Script not found")

    return script.roles


@app.get("script/{script}/role/{name}")
async def read_role(script_name: str, role_name: str):
    """Sample API endpoint."""
    script = Script.from_str(script_name)
    if script is None:
        raise HTTPException(status_code=404, detail="Script not found")

    return script.get_role(role_name)


@app.get("game/new/{script}/{player_count}")
async def new_game(str_script_name: str, player_count: int):
    """Sample API endpoint."""
    global game

    script_name = ScriptName.from_str(str_script_name)
    if script_name is None:
        raise HTTPException(status_code=404, detail="Script not found")

    game = Game(player_count, script_name)


@app.get("game/add_role/{role_name}")
async def add_role(role_name: str):
    """Sample API endpoint."""
    global game
    if game is None:
        raise HTTPException(status_code=404, detail="No game started")

    try:
        game.add_role(role_name)
    except ValueError as e:
        raise HTTPException(status_code=404, detail="Role not in script") from e


@app.get("game/remove_roll/{role_name}")
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
