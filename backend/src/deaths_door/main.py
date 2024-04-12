from fastapi import FastAPI
from fastapi.exceptions import HTTPException

from .script import Script
from .sound_fx import SoundFX

app = FastAPI()


@app.get("/")
async def read_root():
    """Sample API endpoint."""
    return Script("trouble_brewing").characters


@app.get("/role/{name}")
async def read_role(name: str):
    """Sample API endpoint."""
    chars = Script("trouble_brewing").characters
    return next((x for x in chars if x.name.lower() == name.lower()), None)


@app.get("/play_sound_fx/{name}")
async def play_sound_fx(name: str):
    """Sample API endpoint."""
    match name:
        case "death":
            SoundFX().death().play()
        case _:
            raise HTTPException(status_code=404, detail="Sound not found")

    return {"ok": "true"}
