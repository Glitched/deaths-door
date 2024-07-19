from ...character import Character
from ...script import Alignment, CharacterType


class Soldier(Character):
    """Soldier character."""

    def __init__(self):
        """Initialize the Soldier character."""
        self.name = "Soldier"
        self.description = "You are safe from the Demon."
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = []
