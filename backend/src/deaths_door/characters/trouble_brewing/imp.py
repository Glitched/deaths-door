from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType
from ...status_effects import Dead, IsTheDrunk


class Imp(Character):
    """Imp character."""

    def __init__(self):
        """Initialize the Imp character."""
        self.name = "Imp"
        self.description = (
            "Each night*, choose a player: they die. "
            + "If you chose yourself, you die & a Minion becomes the Imp."
        )
        self.category = CharacterType.DEMON
        self.alignment = Alignment.EVIL
        # Imp includes Drunk because imp is always present in the game
        self.status_effects = [IsTheDrunk(), Dead()]
