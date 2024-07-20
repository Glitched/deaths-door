from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Investigator(Character):
    """Investigator character."""

    def __init__(self):
        """Initialize the Investigator character."""
        self.name = "Investigator"
        self.description = (
            "You start knowing that 1 of 2 players is a particular Minion."
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = []
