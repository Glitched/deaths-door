from fastapi import FastAPI
from fastapi.exceptions import HTTPException

from .sound_fx import SoundFX

app = FastAPI()


@app.get("/")
async def read_root():
    """Sample API endpoint."""
    return {"Hello": "World"}


@app.get("/items/{item_id}")
async def read_item(item_id: int, q: str | None = None):
    """Sample API endpoint."""
    return {"item_id": item_id, "q": q}


@app.get("/play_sound_fx/{name}")
async def play_sound_fx(name: str):
    """Sample API endpoint."""
    match name:
        case "death":
            SoundFX().death().play()
        case _:
            raise HTTPException(status_code=404, detail="Sound not found")

    return {"ok": "true"}
