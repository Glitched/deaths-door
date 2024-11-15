from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Bishop(Character):
    """Bishop character."""

    def __init__(self):
        """Initialize the Bishop character."""
        self.name = "Bishop"
        self.description = (
            "Only the Storyteller can nominate."
            + " At least 1 opposing player must be nominated each day."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
