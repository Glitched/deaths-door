from fastapi import APIRouter, HTTPException

from ..config import Config
from ..timer_state import TimerState


def validate_timer_seconds(seconds: int, allow_negative: bool = False) -> None:
    """Validate timer seconds are within acceptable range."""
    min_seconds = -Config.TIMER_MAX_SECONDS if allow_negative else 0
    max_seconds = Config.TIMER_MAX_SECONDS

    if not (min_seconds <= seconds <= max_seconds):
        range_desc = f"between {min_seconds} and {max_seconds}"
        raise HTTPException(
            status_code=400, detail=f"Timer seconds must be {range_desc}"
        )


timer = APIRouter(prefix="/timer")
state = TimerState()


@timer.get("/fetch")
async def fetch_timer():
    """Fetch the current state of the timer."""
    return {
        "is_running": await state.get_is_running(),
        "seconds": await state.get_seconds(),
    }


@timer.get("/set/{seconds}")
async def set_timer(seconds: int):
    """Set the timer to the given number of seconds."""
    validate_timer_seconds(seconds)
    await state.set_seconds(seconds)
    await state.set_is_running(True)


@timer.get("/add/{seconds}")
async def add_seconds(seconds: int):
    """Add the given number of seconds to the timer."""
    validate_timer_seconds(seconds, allow_negative=True)
    await state.add_seconds(seconds)


@timer.get("/start")
async def start_timer(seconds: int | None = None):
    """Start the timer."""
    if seconds is not None:
        validate_timer_seconds(seconds)
        await state.set_seconds(seconds)
    await state.set_is_running(True)


@timer.get("/stop")
async def stop_timer():
    """Stop the timer."""
    await state.set_is_running(False)
