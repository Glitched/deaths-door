from __future__ import annotations

import uuid
from typing import Any

from obswebsocket import obsws, requests

from .obs.types import Input, Scene, SceneItemTransform, VideoSettings

TIMER_NAME = "Countdown Timer"
SCENE_NAME = "Countdown Scene"
FONT_SIZE = 240


class ObsManager:
    """
    Communicate with OBS over the websocket.

    Best OBS Websocket API documentation I've found:
    https://wiki.streamer.bot/en/Broadcasters/OBS/Requests
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

    def __init__(self, host: str, port: int, password: str) -> None:
        """Create a new connection to OBS."""
        self.ws = obsws(host, port, password)
        try:
            self.ws.connect()
        except Exception as e:
            print(f"Failed to connect to OBS: {e}")
        self.run_id = str(uuid.uuid4())

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
        self.reset_scene()
        self.set_current_scene()

        # Add a text source to the scene for the countdown timer.
        el = self.create_input(
            TIMER_NAME,
            "text_ft2_source_v2",
            {
                "text": "5:00",
                "font": {"size": FONT_SIZE, "face": "Help Me"},
                "color": 0xFFFFFFFF,
                "color1": 0xFF001FEF,
                "color2": 0xFF000069,
            },
        )
        self.input_id = el.sceneItemId
        self.set_scene_item_transform(self.input_id, {"scaleX": 2, "scaleY": 2})

    def update_timer(self, seconds: int) -> None:
        """Update the timer."""
        try:
            # Set the text to the current time
            minutes, seconds = divmod(seconds, 60)
            time_text = f"{minutes:01}:{seconds:02}"
            self.set_input_settings(TIMER_NAME, {"text": time_text})

            # Center the text
            transform = self.get_scene_item_transform(self.input_id)
            screen = self.get_video_settings()
            x = (screen.baseWidth - transform.width) / 2
            self.set_scene_item_transform(self.input_id, {"positionX": x})

        except Exception:  # noqa: S110
            # If we're not connected to OBS, don't add log lines regularly
            pass
