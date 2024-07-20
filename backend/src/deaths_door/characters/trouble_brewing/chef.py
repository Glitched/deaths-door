from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Chef(Character):
    """Chef character."""

    def __init__(self):
        """Initialize the Chef character."""
        self.name = "Chef"
        self.description = (
            "You start knowing how many pairs of evil players are"
            + " neighboring each other."
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = []
