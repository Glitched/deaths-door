from asyncio import Lock

from fastapi import FastAPI
from fastapi.exceptions import HTTPException

from .game import Game
from .script import Script, ScriptName
from .sound_fx import SoundFX

lock = Lock()

app = FastAPI()

game = None


@app.get("/")
async def read_root():
    """Sample API endpoint."""
    return Script(ScriptName.TROUBLE_BREWING).characters


@app.get("/scripts")
async def read_scripts():
    """Return a list of available scripts."""
    return list(ScriptName)


@app.get("game/new/{script}/{player_count}")
async def new_game(script_name: str, player_count: int):
    """Sample API endpoint."""
    global game

    try:
        script_name = ScriptName(script_name)
    except ValueError as e:
        raise HTTPException(status_code=404, detail="Script not found") from e

    game = Game(player_count, script_name)


@app.get("script/{script}/role/{name}")
async def read_role(script: str, name: str):
    """Sample API endpoint."""
    if script in ScriptName:
        return False
    chars = Script(ScriptName.TROUBLE_BREWING).characters
    return next((x for x in chars if x.name.lower() == name.lower()), None)


@app.get("/play_sound/{name}")
async def play_sound(name: str):
    """Sample API endpoint."""
    match name:
        case "death":
            SoundFX().death().play()
        case _:
            raise HTTPException(status_code=404, detail="Sound not found")

    return {"ok": "true"}
