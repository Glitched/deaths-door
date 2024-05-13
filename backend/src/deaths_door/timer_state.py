from __future__ import annotations

import asyncio

from .sound_fx import SoundFX, SoundName


class TimerState:
    """The state of the timer."""

    is_running: bool = False
    seconds: int = 0
    _lock: asyncio.Lock = asyncio.Lock()

    def __init__(self):
        """Initialize the timer and start the on-tick loop."""
        loop = asyncio.get_event_loop()

        loop.create_task(self.handle_tick())

    async def handle_tick(self):
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

    async def set_is_running(self, new_value: bool):
        """Set whether the timer should be running."""
        async with self._lock:
            self.is_running = new_value

    async def set_seconds(self, new_value: int):
        """Set the number of seconds left."""
        async with self._lock:
            self.seconds = new_value

    async def add_seconds(self, new_value: int):
        """Add the given number of seconds to the timer."""
        async with self._lock:
            self.seconds += new_value

    async def get_seconds(self):
        """Get the number of seconds left."""
        async with self._lock:
            return self.seconds

    async def get_is_running(self):
        """Get whether the timer is running."""
        async with self._lock:
            return self.is_running
