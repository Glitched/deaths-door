from __future__ import annotations

from time import sleep

from obswebsocket import obsws, requests

TIMER_NAME = "Countdown Timer"
FONT_SIZE = 240


class ObsManager:
    """Communicate with OBS over the websocket."""

    ws: obsws

    def __init__(self, host: str, port: int, password: str) -> None:
        """Create a new connection to OBS."""
        self.ws = obsws(host, port, password)
        self.ws.connect()

    def create_scene(self, scene_name: str) -> None:
        """Create a new scene."""
        self.ws.call(requests.RemoveScene(sceneName=scene_name))
        sleep(1)
        self.ws.call(requests.CreateScene(sceneName=scene_name))

    def set_current_scene(self, scene_name: str) -> None:
        """Set the current scene."""
        self.ws.call(requests.SetCurrentProgramScene(sceneName=scene_name))

    def setup_obs_scene(self) -> None:
        """Create the OBS scene for the countdown."""
        scene_name = "Countdown Scene"
        self.create_scene(scene_name)
        self.set_current_scene(scene_name)

        # Add a text source to the scene for the countdown timer.
        self.ws.call(
            requests.CreateInput(
                sceneName=scene_name,
                inputName=TIMER_NAME,
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
                inputName=TIMER_NAME,
                inputSettings={
                    "text": time_text,
                    "font": {"size": FONT_SIZE, "face": "Help Me"},
                },
            )
        )
