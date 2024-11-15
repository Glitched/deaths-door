from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Scapegoat(Character):
    """Scapegoat character."""

    def __init__(self):
        """Initialize the Scapegoat character."""
        self.name = "Scapegoat"
        self.description = (
            "If a player of your alignment is executed,"
            + " you might be executed instead."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
