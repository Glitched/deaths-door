from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Harlot(Character):
    """Harlot character."""

    def __init__(self):
        """Initialize the Harlot character."""
        self.name = "Harlot"
        self.description = (
            "Each night, choose a living player. If they agree, "
            + "you learn their character, but you both might die."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
