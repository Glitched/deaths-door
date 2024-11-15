from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Deviant(Character):
    """Deviant character."""

    def __init__(self):
        """Initialize the Deviant character."""
        self.name = "Deviant"
        self.description = "If you were funny today, you cannot die by exile."
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
