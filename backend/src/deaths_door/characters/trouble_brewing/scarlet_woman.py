from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType
from ...status_effects import IsTheDemon


class ScarletWoman(Character):
    """Scarlet Woman character."""

    def __init__(self):
        """Initialize the Scarlet Woman character."""
        self.name = "Scarlet Woman"
        self.description = (
            "If there are 5 or more players alive & the Demon dies,"
            + " you become the Demon. (Travellers do not count)"
        )
        self.category = CharacterType.MINION
        self.alignment = Alignment.EVIL
        self.status_effects = [IsTheDemon()]
