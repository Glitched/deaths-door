from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType


class BoneCollector(Character):
    """Bone Collector character."""

    def __init__(self):
        """Initialize the Bone Collector character."""
        self.name = "Bone Collector"
        self.description = (
            "Once per game, at night, choose a dead player: "
            + "they regain their ability until dusk."
        )
        self.category = CharacterType.TRAVELER
        self.alignment = Alignment.UNKNOWN
        self.status_effects = []
