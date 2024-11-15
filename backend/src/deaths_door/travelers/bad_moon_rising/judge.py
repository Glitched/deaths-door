from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Judge(Character):
    """Judge character."""

    def __init__(self):
        """Initialize the Judge character."""
        self.name = "Judge"
        self.description = (
            "Once per game, if another player nominated,"
            + " you may choose to force the current execution to pass or fail."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
