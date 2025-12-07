from __future__ import annotations

import asyncio
import logging
import sys
import uuid
from concurrent.futures import ThreadPoolExecutor
from typing import Any

from obswebsocket import obsws, requests

from .obs.types import Input, Scene, SceneItemTransform, VideoSettings

logger = logging.getLogger(__name__)

TIMER_NAME = "Countdown Timer"
SCENE_NAME = "Countdown Scene"
FONT_SIZE = 240


def get_text_source_type() -> str:
    """Get the appropriate OBS text source type for the current platform."""
    if sys.platform == "win32":
        return "text_gdiplus"
    return "text_ft2_source_v2"


def get_font_settings(face: str, size: int) -> dict[str, Any]:
    """Get font settings appropriate for the current platform."""
    if sys.platform == "win32":
        return {
            "face": face,
            "size": size,
            "style": "Regular",
            "flags": 0,
        }
    return {
        "face": face,
        "size": size,
    }


def get_text_input_settings(
    text: str, font_face: str, font_size: int
) -> dict[str, Any]:
    """Get text input settings appropriate for the current platform."""
    font = get_font_settings(font_face, font_size)

    if sys.platform == "win32":
        return {
            "text": text,
            "font": font,
            "color": 0xFFFFFFFF,  # White in ABGR
        }
    return {
        "text": text,
        "font": font,
        "color1": 0xFFFFFFFF,
        "color2": 0xFF000069,
    }


class ObsManager:
    """
    Communicate with OBS over the websocket.

    Best OBS Websocket API documentation I've found:
    https://wiki.streamer.bot/en/Broadcasters/OBS/Requests

    Supports automatic reconnection with exponential backoff if connection is lost.
    """

    ws: obsws
    """The websocket connection to OBS."""

    run_id: str
    """
    The ID of the current run.
    OBS scene names are unique to each run, because of what seems like a race condition.
    """

    input_id: int = -1
    """The ID of the input for the countdown."""

    connected: bool = False
    """Whether we're currently connected to OBS."""

    _video_settings: VideoSettings | None = None
    """Cached video settings (canvas dimensions). Reset on reconnect."""

    _executor: ThreadPoolExecutor
    """Thread pool for running blocking OBS calls without blocking the event loop."""

    # Connection credentials for reconnection
    _host: str
    _port: int
    _password: str

    # Reconnection state
    _reconnect_task: asyncio.Task[None] | None = None
    _reconnect_delay: float = 1.0
    _max_reconnect_delay: float = 30.0
    _reconnect_enabled: bool = True

    def __init__(self, host: str, port: int, password: str) -> None:
        """Create a new connection to OBS."""
        self._host = host
        self._port = port
        self._password = password
        self.ws = obsws(host, port, password)
        self.connected = False
        self._video_settings = None
        self._executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="obs")
        self._reconnect_task = None
        self._reconnect_delay = 1.0
        self._reconnect_enabled = True

        if self._try_connect():
            logger.info("Successfully connected to OBS WebSocket")
        else:
            logger.warning("Failed to connect to OBS. Will retry in background.")
            self._schedule_reconnect()

        self.run_id = str(uuid.uuid4())

    def _try_connect(self) -> bool:
        """Attempt to connect to OBS. Returns True on success."""
        try:
            self.ws = obsws(self._host, self._port, self._password)
            self.ws.connect()
            self.connected = True
            self._reconnect_delay = 1.0  # Reset backoff on success
            return True
        except Exception as e:
            logger.debug(f"Connection attempt failed: {e}")
            self.connected = False
            return False

    def _schedule_reconnect(self) -> None:
        """Schedule a reconnection attempt if not already running."""
        if not self._reconnect_enabled:
            return
        if self._reconnect_task is not None and not self._reconnect_task.done():
            return  # Already reconnecting
        try:
            loop = asyncio.get_running_loop()
            self._reconnect_task = loop.create_task(self._reconnect_loop())
        except RuntimeError:
            # No running event loop - can't schedule async reconnect
            logger.debug("No event loop available for reconnection scheduling")

    async def _reconnect_loop(self) -> None:
        """Background task that attempts to reconnect with exponential backoff."""
        while not self.connected and self._reconnect_enabled:
            logger.info(f"Attempting OBS reconnection in {self._reconnect_delay:.1f}s")
            await asyncio.sleep(self._reconnect_delay)

            if not self._reconnect_enabled:
                break

            # Run blocking connect in executor
            loop = asyncio.get_running_loop()
            success = await loop.run_in_executor(self._executor, self._try_connect)

            if success:
                logger.info("Reconnected to OBS WebSocket")
                # Re-setup the scene with a new run_id
                self.run_id = str(uuid.uuid4())
                try:
                    await loop.run_in_executor(self._executor, self.setup_obs_scene)
                    logger.info("OBS scene re-initialized after reconnect")
                except Exception as e:
                    logger.error(f"Failed to setup OBS scene after reconnect: {e}")
                    self.connected = False
                    # Continue trying to reconnect
            else:
                # Exponential backoff, capped at max
                self._reconnect_delay = min(
                    self._reconnect_delay * 2, self._max_reconnect_delay
                )

    def _mark_disconnected(self) -> None:
        """Mark as disconnected and trigger reconnection."""
        self.connected = False
        self._video_settings = None
        self._schedule_reconnect()

    # Sadly, the OBS websocket API is not typed.
    def call(self, request: Any) -> Any:
        """Call a request."""
        return self.ws.call(request)

    def get_object_name(self, obj: str) -> str:
        """Get the name of an object for this run."""
        return f"{obj} • {self.run_id}"

    def get_video_settings(self) -> VideoSettings:
        """Get the video info."""
        return VideoSettings(**self.call(requests.GetVideoSettings()).datain)

    def get_scene_item_transform(self, id: int) -> SceneItemTransform:
        """Get the transform of a scene item."""
        req = self.call(
            requests.GetSceneItemTransform(
                sceneName=self.get_object_name(SCENE_NAME),
                sceneItemId=id,
            )
        )
        return SceneItemTransform(**req.datain["sceneItemTransform"])

    def set_scene_item_transform(self, id: int, transform: dict[str, Any]) -> None:
        """Set the transform of a scene item."""
        self.call(
            requests.SetSceneItemTransform(
                sceneName=self.get_object_name(SCENE_NAME),
                sceneItemId=id,
                sceneItemTransform=transform,
            )
        )

    def get_scenes(self) -> list[Scene]:
        """Get a list of all scenes."""
        req = self.call(requests.GetSceneList())
        return [Scene(**scene) for scene in req.datain["scenes"]]

    def reset_scene(self) -> None:
        """
        Remove the existing scenes and create a new one.

        Prefer setting up a new scene from scratch to avoid accidental mutations.
        """
        scenes = self.get_scenes()
        for scene in scenes:
            if "Countdown Scene" in scene.sceneName:
                self.call(requests.RemoveScene(sceneName=scene.sceneName))
        self.call(requests.CreateScene(sceneName=self.get_object_name(SCENE_NAME)))

    def set_current_scene(self) -> None:
        """Set the current scene."""
        self.call(
            requests.SetCurrentProgramScene(sceneName=self.get_object_name(SCENE_NAME))
        )

    def create_input(
        self, input_name: str, input_kind: str, input_settings: dict[str, Any]
    ) -> Input:
        """Create an input for the countdown."""
        req = self.call(
            requests.CreateInput(
                sceneName=self.get_object_name(SCENE_NAME),
                inputName=self.get_object_name(input_name),
                inputKind=input_kind,
                inputSettings=input_settings,
            )
        )
        return Input(**req.datain)

    def set_input_settings(
        self, input_name: str, input_settings: dict[str, Any]
    ) -> None:
        """Set the settings of an input."""
        self.call(
            requests.SetInputSettings(
                inputName=self.get_object_name(input_name),
                inputSettings=input_settings,
            )
        )

    def setup_obs_scene(self) -> None:
        """Create the OBS scene for the countdown."""
        if not self.connected:
            logger.debug("Skipping OBS scene setup - not connected")
            return

        try:
            self.reset_scene()
            self.set_current_scene()

            # Add a text source to the scene for the countdown timer.
            # Try with custom font, fallback to Arial if not available.
            # Uses platform-specific source (GDI+ on Windows, FreeType2 elsewhere).
            source_type = get_text_source_type()
            try:
                el = self.create_input(
                    TIMER_NAME,
                    source_type,
                    get_text_input_settings("5:00", "Help Me", FONT_SIZE),
                )
            except Exception:
                logger.warning("Font 'Help Me' not found, using Arial as fallback")
                el = self.create_input(
                    TIMER_NAME,
                    source_type,
                    get_text_input_settings("5:00", "Arial", FONT_SIZE),
                )

            self.input_id = el.sceneItemId
            self.set_scene_item_transform(self.input_id, {"scaleX": 2, "scaleY": 2})

            # Cache video settings (canvas dimensions don't change mid-stream)
            self._video_settings = self.get_video_settings()
        except Exception as e:
            logger.error(f"Failed to setup OBS scene: {e}")
            self.connected = False
            self._video_settings = None
            raise

    def _update_timer_sync(self, seconds: int) -> None:
        """Update the timer (blocking). Use update_timer_async() from async code."""
        if not self.connected or self._video_settings is None:
            return  # Gracefully skip if not connected

        try:
            # Set the text to the current time
            minutes, secs = divmod(seconds, 60)
            time_text = f"{minutes:01}:{secs:02}"
            self.set_input_settings(TIMER_NAME, {"text": time_text})

            # Center the text using cached video settings
            transform = self.get_scene_item_transform(self.input_id)
            x = (self._video_settings.baseWidth - transform.width) / 2
            self.set_scene_item_transform(self.input_id, {"positionX": x})

        except Exception as e:
            # Connection lost during operation - trigger reconnection
            logger.warning(f"OBS connection lost during timer update: {e}")
            self._mark_disconnected()

    async def update_timer_async(self, seconds: int) -> None:
        """Update the timer without blocking the event loop."""
        if not self.connected or self._video_settings is None:
            return
        loop = asyncio.get_running_loop()
        await loop.run_in_executor(self._executor, self._update_timer_sync, seconds)

    def close(self) -> None:
        """Close the OBS connection and clean up resources."""
        # Stop reconnection attempts
        self._reconnect_enabled = False
        if self._reconnect_task is not None and not self._reconnect_task.done():
            self._reconnect_task.cancel()

        self._executor.shutdown(wait=False, cancel_futures=True)
        if self.connected:
            try:
                self.ws.disconnect()
            except Exception as e:
                logger.debug(f"Error disconnecting from OBS: {e}")
        self.connected = False
        self._video_settings = None
        logger.info("Closed OBS connection")

    def __enter__(self) -> ObsManager:
        """Context manager entry."""
        return self

    def __exit__(
        self, exc_type: type | None, exc_val: Exception | None, exc_tb: object
    ) -> None:
        """Context manager exit with cleanup."""
        self.close()
