from fastapi import APIRouter
from fastapi.exceptions import HTTPException

from ..sound_fx import SoundFX, SoundName

router = APIRouter()


@router.get("/sounds/play/{name}")
async def play_sound(name: str):
    """Sample API endpoint."""
    sound_name = SoundName.from_str(name)

    if sound_name is None:
        raise HTTPException(status_code=404, detail="Sound not found")

    try:
        SoundFX().play(sound_name)
    except Exception as e:
        raise HTTPException(status_code=500, detail="Failed to play sound") from e


@router.get("/sounds/list")
async def list_sounds():
    """Return the names of available sounds to play."""
    return list(SoundName)
