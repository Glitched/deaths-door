from __future__ import annotations

import asyncio
import logging

from .config import Config
from .obs_manager import ObsManager
from .sound_fx import SoundFX, SoundName

logger = logging.getLogger(__name__)


class TimerState:
    """The state of the timer."""

    is_running: bool = False
    seconds: int = 5 * 60
    _lock: asyncio.Lock
    _obs_manager: ObsManager

    def __init__(self):
        """Initialize the timer and OBS manager."""
        self._lock = asyncio.Lock()
        self._obs_manager = ObsManager(
            host="localhost", port=4455, password=Config.get_obs_password()
        )
        self._tick_task = None

        try:
            self._obs_manager.setup_obs_scene()
        except Exception as e:
            if Config.is_obs_required():
                raise RuntimeError(f"OBS connection required but failed: {e}") from e
            logger.warning(f"OBS not available, running without streaming support: {e}")

    def _ensure_tick_task_running(self):
        """Ensure the tick task is running if we have an event loop."""
        if self._tick_task is None or self._tick_task.done():
            try:
                loop = asyncio.get_running_loop()
                self._tick_task = loop.create_task(self.handle_tick())
            except RuntimeError:
                # No running loop - that's fine, task will start when loop exists
                pass

    async def handle_tick(self):
        """Handle the tick of the timer."""
        while True:
            async with self._lock:
                if self.is_running:
                    await self._obs_manager.update_timer_async(self.seconds)
                    if self.seconds > 0:
                        self.seconds -= 1
                    else:
                        self.is_running = False
                        self.seconds = 0
                        SoundFX().play(SoundName.TIMER)
            await asyncio.sleep(1)

    async def set_is_running(self, new_value: bool):
        """Set whether the timer should be running."""
        self._ensure_tick_task_running()
        async with self._lock:
            self.is_running = new_value

    async def set_seconds(self, new_value: int):
        """Set the number of seconds left."""
        self._ensure_tick_task_running()
        async with self._lock:
            self.seconds = new_value
            if self.seconds < 0:
                self.seconds = 0

    async def add_seconds(self, additional_seconds: int):
        """Add the given number of seconds to the timer."""
        await self.set_seconds(self.seconds + additional_seconds)

    async def get_seconds(self):
        """Get the number of seconds left."""
        async with self._lock:
            return self.seconds

    async def get_is_running(self):
        """Get whether the timer is running."""
        async with self._lock:
            return self.is_running
