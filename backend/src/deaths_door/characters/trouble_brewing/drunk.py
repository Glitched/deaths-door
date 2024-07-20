from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Drunk(Character):
    """Drunk character."""

    def __init__(self):
        """Initialize the Drunk character."""
        self.name = "Drunk"
        self.description = (
            "You do not know you are the Drunk. You think you are a Townsfolk, "
            + "but your ability malfunctions."
        )
        self.category = CharacterType.OUTSIDER
        self.alignment = Alignment.GOOD
        self.status_effects = []
