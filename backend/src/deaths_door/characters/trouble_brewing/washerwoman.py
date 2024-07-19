from ...character import Character
from ...script import Alignment, CharacterType


class Washerwoman(Character):
    """Washerwoman character."""

    def __init__(self):
        """Initialize the Washerwoman character."""
        self.name = "Washerwoman"
        self.description = (
            "You start knowing that 1 of 2 players is a particular Townsfolk."
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = []
