from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Librarian(Character):
    """Librarian character."""

    def __init__(self):
        """Initialize the Librarian character."""
        self.name = "Librarian"
        self.description = (
            "You start knowing that 1 of 2 players is a particular Outsider."
            + " (Or that zero are in play)"
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = []
