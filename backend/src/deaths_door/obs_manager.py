from __future__ import annotations

import uuid

from obswebsocket import obsws, requests

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

    def __init__(self, host: str, port: int, password: str) -> None:
        """Create a new connection to OBS."""
        self.ws = obsws(host, port, password)
        self.ws.connect()
        self.run_id = str(uuid.uuid4())

    def get_object_name(self, obj: str) -> str:
        """Get the name of an object for this run."""
        return f"{obj} • {self.run_id}"

    def get_scenes(self) -> list[dict[str, str]]:
        """Get a list of all scenes."""
        req = self.ws.call(requests.GetSceneList())
        return req.datain["scenes"]

    def reset_scene(self) -> None:
        """
        Remove the existing scenes and create a new one.

        Prefer setting up a new scene from scratch to avoid accidental mutations.
        """
        scenes = self.get_scenes()
        for scene in scenes:
            if "Countdown Scene" in scene["sceneName"]:
                self.ws.call(requests.RemoveScene(sceneName=scene["sceneName"]))
        self.ws.call(requests.CreateScene(sceneName=self.get_object_name(SCENE_NAME)))

    def set_current_scene(self) -> None:
        """Set the current scene."""
        self.ws.call(
            requests.SetCurrentProgramScene(sceneName=self.get_object_name(SCENE_NAME))
        )

    def setup_obs_scene(self) -> None:
        """Create the OBS scene for the countdown."""
        self.reset_scene()
        self.set_current_scene()

        # Add a text source to the scene for the countdown timer.
        self.ws.call(
            requests.CreateInput(
                sceneName=self.get_object_name(SCENE_NAME),
                inputName=self.get_object_name(TIMER_NAME),
                # Windows source is text_gdiplus_v2
                inputKind="text_ft2_source_v2",
                inputSettings={
                    "text": "5:00",
                    "font": {"size": FONT_SIZE, "face": "Help Me"},
                    "color": 0xFFFFFFFF,
                    "color1": 0xFF001FEF,
                    "color2": 0xFF000069,
                },
            )
        )

    def update_timer(self, seconds: int) -> None:
        """Update the timer."""
        minutes, seconds = divmod(seconds, 60)
        time_text = f"{minutes:01}:{seconds:02}"
        self.ws.call(
            requests.SetInputSettings(
                inputName=self.get_object_name(TIMER_NAME),
                inputSettings={
                    "text": time_text,
                    "font": {"size": FONT_SIZE, "face": "Help Me"},
                },
            )
        )
