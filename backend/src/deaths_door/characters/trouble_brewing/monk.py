from ...character import Character
from ...script import Alignment, CharacterType


class Monk(Character):
    """Monk character."""

    def __init__(self):
        """Initialize the Monk character."""
        self.name = "Monk"
        self.description = (
            "Each night*, choose a player (not yourself):"
            + " they are safe from the Demon tonight."
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = []