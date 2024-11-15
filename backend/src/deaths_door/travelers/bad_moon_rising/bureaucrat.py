from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Apprentice(Character):
    """Apprentice character."""

    def __init__(self):
        """Initialize the Apprentice character."""
        self.name = "Apprentice"
        self.description = (
            "On your 1st night, you gain a Townsfolk ability "
            + "(if good), or a Minion ability (if evil)."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
