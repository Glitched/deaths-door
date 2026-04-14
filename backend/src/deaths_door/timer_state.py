"""Timer state management."""

from __future__ import annotations

import asyncio
import logging

from .apns_manager import APNSManager
from .game_manager import game_manager
from .sound_fx import SoundFX, SoundName

logger = logging.getLogger(__name__)


class TimerState:
    """The state of the timer."""

    is_running: bool = False
    seconds: int = 5 * 60
    _lock: asyncio.Lock
    _apns_manager: APNSManager

    def __init__(self) -> None:
        """Initialize the timer and APNS manager."""
        self._lock = asyncio.Lock()
        self._apns_manager = APNSManager()
        self._tick_task: asyncio.Task[None] | None = None

    def _ensure_tick_task_running(self) -> None:
        """Ensure the tick task is running if we have an event loop."""
        if self._tick_task is None or self._tick_task.done():
            try:
                loop = asyncio.get_running_loop()
                self._tick_task = loop.create_task(self.handle_tick())
            except RuntimeError:
                pass

    async def handle_tick(self) -> None:
        """Handle the tick of the timer."""
        while True:
            async with self._lock:
                if self.is_running:
                    if self.seconds > 0:
                        self.seconds -= 1
                    else:
                        self.is_running = False
                        self.seconds = 0
                        SoundFX().play(SoundName.TIMER)
            await asyncio.sleep(1)

    @property
    def apns_manager(self) -> APNSManager:
        """Access the APNS manager for token registration."""
        return self._apns_manager

    async def _get_player_counts(self) -> tuple[int, int]:
        """Get (players_alive, total_players) from the game state."""
        try:
            state = await game_manager.get_state()
            total = len(state.players)
            alive = sum(1 for p in state.players if p.is_alive)
            return alive, total
        except Exception:
            return 0, 0

    async def set_is_running(self, new_value: bool) -> None:
        """Set whether the timer should be running."""
        self._ensure_tick_task_running()
        async with self._lock:
            self.is_running = new_value
            alive, total = await self._get_player_counts()
            await self._apns_manager.send_timer_update(self.seconds, self.is_running, alive, total)

    async def set_seconds(self, new_value: int) -> None:
        """Set the number of seconds left."""
        self._ensure_tick_task_running()
        async with self._lock:
            self.seconds = new_value
            if self.seconds < 0:
                self.seconds = 0
            alive, total = await self._get_player_counts()
            await self._apns_manager.send_timer_update(self.seconds, self.is_running, alive, total)

    async def add_seconds(self, additional_seconds: int) -> None:
        """Add the given number of seconds to the timer."""
        await self.set_seconds(self.seconds + additional_seconds)

    async def get_seconds(self) -> int:
        """Get the number of seconds left."""
        async with self._lock:
            return self.seconds

    async def get_is_running(self) -> bool:
        """Get whether the timer is running."""
        async with self._lock:
            return self.is_running
