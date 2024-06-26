from __future__ import annotations

from enum import Enum

from pydantic import BaseModel, TypeAdapter


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


class Category(str, Enum):
    """The category of the role."""

    TOWNSFOLK = "townsfolk"
    OUTSIDER = "outsider"
    MINION = "minion"
    DEMON = "demon"


class Alignment(str, Enum):
    """The alignment of the role."""

    GOOD = "good"
    EVIL = "evil"


class Changes(BaseModel):
    """Represents the changes to the number of roles in the Script."""

    townsfolk: None | int = None
    outsider: None | int = None
    minion: None | int = None
    demon: None | int = None


class Role(BaseModel):
    """A role in the script."""

    name: str
    description: str
    category: Category
    alignment: Alignment
    changes: None | Changes = None


class Script:
    """Represent a Blood on the Clocktower script."""

    roles: dict[str, Role]
    name: ScriptName

    @classmethod
    def from_str(cls, name: str) -> Script | None:
        """Return the Script for a given string if present, else return none."""
        script_name = ScriptName.from_str(name)
        if script_name is None:
            return None
        return cls(script_name)

    def __init__(self, script_name: ScriptName) -> None:
        """Load a script from JSON."""
        self.name = script_name

        adapter = TypeAdapter(list[Role])
        try:
            with open(f"src/assets/scripts/{script_name.value}.json", "r") as f:
                role_list = adapter.validate_json(f.read())
                self.roles = {
                    self.normalize_role_name(role.name): role for role in role_list
                }
        except FileNotFoundError as err:
            raise ValueError(f"Script {script_name} not found") from err

    def get_role(self, name: str) -> Role | None:
        """Get a role by name."""
        return self.roles[self.normalize_role_name(name)]

    def has_role(self, name: str) -> bool:
        """Return True if the role is in the given script."""
        return self.normalize_role_name(name) in self.roles

    def normalize_role_name(self, name: str) -> str:
        """Normalize a role name for simplicity."""
        return name.lower().strip()
