from fastapi import FastAPI
from fastapi.exceptions import HTTPException

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


@app.get("/play_sound/{name}")
async def play_sound(name: str):
    """Sample API endpoint."""
    match name:
        case "death":
            SoundFX().death().play()
        case _:
            raise HTTPException(status_code=404, detail="Sound not found")

    return {"ok": "true"}
