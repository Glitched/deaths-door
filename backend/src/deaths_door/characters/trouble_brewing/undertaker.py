from ...character import Character
from ...script import Alignment, CharacterType


class Undertaker(Character):
    """Undertaker character."""

    def __init__(self):
        """Initialize the Undertaker character."""
        self.name = "Undertaker"
        self.description = (
            "Each night*, you learn a character that died by execution today."
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = []
