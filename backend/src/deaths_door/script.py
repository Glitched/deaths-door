from enum import Enum

from pydantic import BaseModel, TypeAdapter


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

    characters: list[Role]

    def __init__(self, script_name: str) -> None:
        """Load a script from JSON."""
        adapter = TypeAdapter(list[Role])
        try:
            with open(f"src/assets/scripts/{script_name}.json", "r") as f:
                self.characters = adapter.validate_json(f.read())
        except FileNotFoundError as err:
            raise ValueError(f"Script {script_name} not found") from err
