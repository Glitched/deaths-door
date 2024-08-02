from __future__ import annotations

from .character import Character
from .night_step import NightStep
from .script_name import ScriptName


class Script:
    """Represent a Blood on the Clocktower script."""

    characters: list[Character]
    name: ScriptName
    first_night_steps: list[NightStep]
    other_night_steps: list[NightStep]

    def get_character(self, name: str) -> Character | None:
        """Get a character by name."""
        for character in self.characters:
            if character.is_named(name):
                return character
        return None

    def has_character(self, name: str) -> bool:
        """Return True if the character is in the given script."""
        return self.get_character(name) is not None

    def get_first_night_steps(self) -> list[NightStep]:
        """Get the first night steps."""
        return self.first_night_steps

    def get_other_night_steps(self) -> list[NightStep]:
        """Get the other night steps."""
        return self.other_night_steps
