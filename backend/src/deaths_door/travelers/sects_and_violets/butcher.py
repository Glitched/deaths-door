from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Butcher(Character):
    """Butcher character."""

    def __init__(self):
        """Initialize the Butcher character."""
        self.name = "Butcher"
        self.description = (
            "Each day, after the 1st execution, " + "you may nominate again."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
