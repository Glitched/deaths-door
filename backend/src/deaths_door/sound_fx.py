from __future__ import annotations

import pygame


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

    def death(self):
        """Return the sound effect for a player death."""
        return pygame.mixer.Sound("src/assets/sound_fx/death.wav")

    def alarm(self):
        """Return the sound effect for an alarm ringing."""
        return pygame.mixer.Sound("src/assets/sound_fx/alarm.wav")

    def rooster(self):
        """Return the sound effect for rooster crowing."""
        return pygame.mixer.Sound("src/assets/sound_fx/rooster.wav")
