from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Saint(Character):
    """Saint character."""

    def __init__(self):
        """Initialize the Saint character."""
        self.name = "Saint"
        self.description = "If you die by execution, you lose."
        self.category = CharacterType.OUTSIDER
        self.alignment = Alignment.GOOD
        self.status_effects = []
