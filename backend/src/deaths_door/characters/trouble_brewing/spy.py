from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Spy(Character):
    """Spy character."""

    def __init__(self):
        """Initialize the Spy character."""
        self.name = "Spy"
        self.description = (
            "Each night, you see the Grimoire. "
            + "You might register as good & as a Townsfolk or Outsider, even if dead."
        )
        self.category = CharacterType.MINION
        self.alignment = Alignment.EVIL
        self.status_effects = []
