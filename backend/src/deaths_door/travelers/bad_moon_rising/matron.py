from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Matron(Character):
    """Matron character."""

    def __init__(self):
        """Initialize the Matron character."""
        self.name = "Matron"
        self.description = (
            "Each day, you may choose up to 3 sets of 2 players"
            + " to swap seats. Players may not leave their seats to talk in private."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
