from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Virgin(Character):
    """Virgin character."""

    def __init__(self):
        """Initialize the Virgin character."""
        self.name = "Virgin"
        self.description = (
            "The 1st time you are nominated, if the nominator is a Townsfolk, "
            + "they are executed."
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = []
