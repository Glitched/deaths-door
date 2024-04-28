from enum import Enum

from pydantic import BaseModel, TypeAdapter


class ScriptName(str, Enum):
    """The name of a script."""

    TROUBLE_BREWING = "trouble_brewing"
    SECTS_AND_VIOLETS = "sects_and_violents"
    BAD_MOON_RISING = "bad_moon_rising"

    def __str__(self) -> str:
        """Return the human readable name of the script."""
        match self:
            case ScriptName.TROUBLE_BREWING:
                return "Trouble Brewing"
            case ScriptName.SECTS_AND_VIOLETS:
                return "Sects and Violents"
            case ScriptName.BAD_MOON_RISING:
                return "Bad Moon Rising"


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

    def __init__(self, script_name: ScriptName) -> None:
        """Load a script from JSON."""
        adapter = TypeAdapter(list[Role])
        try:
            with open(f"src/assets/scripts/{script_name}.json", "r") as f:
                self.characters = adapter.validate_json(f.read())
        except FileNotFoundError as err:
            raise ValueError(f"Script {script_name} not found") from err
