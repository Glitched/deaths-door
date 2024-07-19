from ...character import Character
from ...script import Alignment, CharacterType


class FortuneTeller(Character):
    """Fortune Teller character."""

    def __init__(self):
        """Initialize the Fortune Teller character."""
        self.name = "Fortune Teller"
        self.description = (
            "Each night, choose 2 players: you learn if either is a Demon."
            + " There is 1 good player that registers falsely to you."
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = []
