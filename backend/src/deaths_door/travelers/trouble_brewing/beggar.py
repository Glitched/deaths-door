from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Beggar(Character):
    """Beggar character."""

    def __init__(self):
        """Initialize the Beggar character."""
        self.name = "Beggar"
        self.description = (
            "You must use a vote token to vote."
            + " If a dead player gives you theirs, you learn their alignment."
            + " You are sober & healthy."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
