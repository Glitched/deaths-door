from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Voudon(Character):
    """Voudon character."""

    def __init__(self):
        """Initialize the Voudon character."""
        self.name = "Voudon"
        self.description = (
            "Only you & the dead can vote. "
            + "They don’t need a vote token to do so. A 50% majority is still required."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
