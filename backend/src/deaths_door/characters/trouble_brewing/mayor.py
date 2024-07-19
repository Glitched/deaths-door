from ...character import Character
from ...script import Alignment, CharacterType


class Mayor(Character):
    """Mayor character."""

    def __init__(self):
        """Initialize the Mayor character."""
        self.name = "Mayor"
        self.description = (
            "If no execution occurs while only 3 players live, you win. "
            + "If you die at night, another player might die instead."
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = []
