from __future__ import annotations

from enum import Enum


class ScriptName(str, Enum):
    """The name of a script."""

    TROUBLE_BREWING = "trouble_brewing"
    SECTS_AND_VIOLETS = "sects_and_violets"
    BAD_MOON_RISING = "bad_moon_rising"

    def __str__(self) -> str:
        """Return the human readable name of the script."""
        match self:
            case ScriptName.TROUBLE_BREWING:
                return "Trouble Brewing"
            case ScriptName.SECTS_AND_VIOLETS:
                return "Sects and Violets"
            case ScriptName.BAD_MOON_RISING:
                return "Bad Moon Rising"

    @classmethod
    def from_str(cls, name: str) -> ScriptName | None:
        """Return the ScriptName for a given string if present, else return none."""
        for script in cls:
            if script.value == name.lower():
                return script
