from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class Recluse(Character):
    """Recluse character."""

    def __init__(self):
        """Initialize the Recluse character."""
        self.name = "Recluse"
        self.description = (
            "You might register as evil & as a Minion or Demon, even if dead."
        )
        self.category = CharacterType.OUTSIDER
        self.alignment = Alignment.GOOD
        self.status_effects = []
