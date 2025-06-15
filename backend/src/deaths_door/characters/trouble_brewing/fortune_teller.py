from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType
from ...status_effects import RedHerring


class FortuneTeller(Character):
    """Fortune Teller character."""

    def __init__(self):
        """Initialize the Fortune Teller character."""
        self.name = "Fortune Teller"
        self.description = (
            "Each night, choose 2 players: you learn if either is a Demon."
            " There is 1 good player that registers falsely to you."
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = [RedHerring()]
