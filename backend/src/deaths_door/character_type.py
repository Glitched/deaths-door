from enum import Enum


class CharacterType(str, Enum):
    """The category of the role."""

    TOWNSFOLK = "townsfolk"
    OUTSIDER = "outsider"
    MINION = "minion"
    DEMON = "demon"
