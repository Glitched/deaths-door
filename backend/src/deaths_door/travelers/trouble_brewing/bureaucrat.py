from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Bureaucrat(Character):
    """Bureaucrat character."""

    def __init__(self):
        """Initialize the Bureaucrat character."""
        self.name = "Bureaucrat"
        self.description = (
            "Each night, choose a player (not yourself);"
            + " their vote counts as 3 votes tomorrow."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
