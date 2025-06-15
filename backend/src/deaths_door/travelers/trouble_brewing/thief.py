from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Thief(Character):
    """Thief character."""

    def __init__(self):
        """Initialize the Thief character."""
        self.name = "Thief"
        self.description = (
            "Each night, choose a player (not yourself);"
            " their vote counts negatively tomorrow."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
