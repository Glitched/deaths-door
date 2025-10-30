from fastapi import APIRouter, HTTPException, Path, Query
from pydantic import BaseModel, Field

from ..config import Config
from ..timer_state import TimerState


class TimerStateResponse(BaseModel):
    """Current state of the countdown timer."""

    is_running: bool = Field(..., description="Whether the timer is currently running")
    seconds: int = Field(..., description="Remaining seconds on the timer", examples=[300])


class TimerOperationResponse(BaseModel):
    """Response after a timer operation."""

    status: str = Field(..., description="Operation status", examples=["success"])
    is_running: bool = Field(..., description="Whether the timer is currently running")
    seconds: int = Field(..., description="Current seconds on the timer", examples=[300])


def validate_timer_seconds(seconds: int, allow_negative: bool = False) -> None:
    """Validate timer seconds are within acceptable range."""
    min_seconds = -Config.TIMER_MAX_SECONDS if allow_negative else 0
    max_seconds = Config.TIMER_MAX_SECONDS

    if not (min_seconds <= seconds <= max_seconds):
        range_desc = f"between {min_seconds} and {max_seconds}"
        raise HTTPException(
            status_code=400, detail=f"Timer seconds must be {range_desc}"
        )


timer = APIRouter(prefix="/timer", tags=["Timer"])
state = TimerState()


@timer.get("/fetch")
async def fetch_timer() -> TimerStateResponse:
    """Fetch the current state of the timer."""
    return TimerStateResponse(
        is_running=await state.get_is_running(),
        seconds=await state.get_seconds(),
    )


@timer.get("/set/{seconds}", responses={400: {"description": "Invalid timer seconds"}})
async def set_timer(
    seconds: int = Path(
        ...,
        description="Number of seconds to set on the timer",
        examples=[300],
    )
) -> TimerOperationResponse:
    """Set the timer to the given number of seconds and start it."""
    validate_timer_seconds(seconds)
    await state.set_seconds(seconds)
    await state.set_is_running(True)
    return TimerOperationResponse(
        status="success", is_running=True, seconds=seconds
    )


@timer.get("/add/{seconds}", responses={400: {"description": "Invalid timer seconds"}})
async def add_seconds(
    seconds: int = Path(
        ...,
        description="Number of seconds to add (can be negative)",
        examples=[60, -30],
    )
) -> TimerOperationResponse:
    """Add the given number of seconds to the timer (can be negative to subtract)."""
    validate_timer_seconds(seconds, allow_negative=True)
    await state.add_seconds(seconds)
    return TimerOperationResponse(
        status="success",
        is_running=await state.get_is_running(),
        seconds=await state.get_seconds(),
    )


@timer.get("/start", responses={400: {"description": "Invalid timer seconds"}})
async def start_timer(
    seconds: int | None = Query(
        None,
        description="Optional number of seconds to set before starting the timer",
        examples=[300, 60],
    )
) -> TimerOperationResponse:
    """Start the timer, optionally setting seconds first."""
    if seconds is not None:
        validate_timer_seconds(seconds)
        await state.set_seconds(seconds)
    await state.set_is_running(True)
    return TimerOperationResponse(
        status="success",
        is_running=True,
        seconds=await state.get_seconds(),
    )


@timer.get("/stop")
async def stop_timer() -> TimerOperationResponse:
    """Stop the timer."""
    await state.set_is_running(False)
    return TimerOperationResponse(
        status="success",
        is_running=False,
        seconds=await state.get_seconds(),
    )
