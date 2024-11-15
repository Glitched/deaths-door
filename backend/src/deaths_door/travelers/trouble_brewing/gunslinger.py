from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Gunslinger(Character):
    """Gunslinger character."""

    def __init__(self):
        """Initialize the Gunslinger character."""
        self.name = "Gunslinger"
        self.description = (
            "Each day, after the 1st vote has been tallied,"
            + " you may choose a player that voted: they die."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
