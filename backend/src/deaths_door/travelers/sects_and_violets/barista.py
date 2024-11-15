from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Barista(Character):
    """Barista character."""

    def __init__(self):
        """Initialize the Barista character."""
        self.name = "Barista"
        self.description = (
            "Each night, until dusk, 1) a player becomes sober,"
            + " healthy & gets true info, or 2) their ability works twice. "
            + "They learn which."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
