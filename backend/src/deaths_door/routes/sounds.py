from fastapi import APIRouter, Path
from fastapi.exceptions import HTTPException
from pydantic import BaseModel, Field

from ..sound_fx import SoundFX, SoundName, sounds

router = APIRouter(prefix="/sounds", tags=["Sounds"])


class PlaySoundResponse(BaseModel):
    """Response after playing a sound effect."""

    status: str = Field(..., description="Operation status", examples=["success"])
    sound: str = Field(..., description="Name of the sound that was played", examples=["bell"])


@router.get(
    "/play/{name}",
    responses={
        404: {"description": "Sound not found"},
        500: {"description": "Failed to play sound"},
    },
)
async def play_sound(
    name: str = Path(..., description="Name of the sound effect to play", examples=["bell"])
) -> PlaySoundResponse:
    """Play a sound effect."""
    sound_name = SoundName.from_str(name)

    if sound_name is None:
        raise HTTPException(status_code=404, detail="Sound not found")

    try:
        SoundFX().play(sound_name)
        return PlaySoundResponse(status="success", sound=name)
    except Exception as e:
        raise HTTPException(status_code=500, detail="Failed to play sound") from e


@router.get("/list")
async def list_sounds() -> dict[str, list[str]]:
    """Return the available sounds organized by category."""
    # Convert SoundName enums to strings for JSON serialization
    return {
        category: [sound.value for sound in sound_list]
        for category, sound_list in sounds.items()
    }
