from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Poisoner(Character):
    """Poisoner character."""

    def __init__(self):
        """Initialize the Poisoner character."""
        self.name = "Poisoner"
        self.description = (
            "Each night, choose a player:"
            + " their ability malfunctions tonight and tomorrow day."
        )
        self.category = CharacterType.MINION
        self.alignment = Alignment.EVIL
        self.status_effects = []
