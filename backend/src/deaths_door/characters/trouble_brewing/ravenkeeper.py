from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Ravenkeeper(Character):
    """Ravenkeeper character."""

    def __init__(self):
        """Initialize the Ravenkeeper character."""
        self.name = "Ravenkeeper"
        self.description = (
            "If you die at night, you are woken to choose a player: "
            + "you learn their character."
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = []
