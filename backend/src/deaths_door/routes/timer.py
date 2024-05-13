from fastapi import APIRouter

from ..timer_state import TimerState

timer = APIRouter()
state = TimerState()


@timer.get("/timer/fetch")
async def fetch_timer():
    """Fetch the current state of the timer."""
    return {
        "is_running": await state.get_is_running(),
        "seconds": await state.get_seconds(),
    }


@timer.get("/timer/set/{seconds}")
async def set_timer(seconds: int):
    """Set the timer to the given number of seconds."""
    await state.set_seconds(seconds)
    await state.set_is_running(True)


@timer.get("/timer/add/{seconds}")
async def add_seconds(seconds: int):
    """Add the given number of seconds to the timer."""
    await state.add_seconds(seconds)


@timer.get("/timer/start")
async def start_timer(seconds: int | None = None):
    """Start the timer."""
    await state.set_is_running(True)


@timer.get("/timer/stop")
async def stop_timer():
    """Stop the timer."""
    await state.set_is_running(False)
