from ...character import Character
from ...script import Alignment, CharacterType


class Empath(Character):
    """Empath character."""

    def __init__(self):
        """Initialize the Empath character."""
        self.name = "Empath"
        self.description = (
            "Each night, you learn how many of your 2 alive neighbors are evil."
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = []
