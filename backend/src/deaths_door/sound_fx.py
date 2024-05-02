from __future__ import annotations

from enum import Enum

import pygame


class SoundName(str, Enum):
    """The name of a script."""

    DEATH = "death"
    ROOSTER = "rooster"
    ALARM = "alarm"

    @classmethod
    def from_str(cls, name: str) -> SoundName | None:
        """Return the SoundName for a given string if present, else return none."""
        for script in cls:
            if script.value == name.lower():
                return script


class SoundFX:
    """Collection of all our sound files."""

    _instance: None | SoundFX = None

    def __new__(cls) -> SoundFX:
        """Return the existing instance of the class, if present."""
        if cls._instance is None:
            cls._instance = super(SoundFX, cls).__new__(cls)
        return cls._instance

    def __init__(self) -> None:
        """Init the pygame mixer."""
        pygame.mixer.init()

    def get_sound(self, sound_name: SoundName):
        """Return the sound effect for a given name."""
        return pygame.mixer.Sound(f"src/assets/sound_fx/{sound_name.value}.wav")

    def play(self, sound_name: SoundName):
        """Play and return the given sound."""
        sound = self.get_sound(sound_name)
        sound.play()
        return sound
