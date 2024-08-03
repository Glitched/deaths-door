from ...alignment import Alignment
from ...character import Character
from ...character_type import CharacterType
from ...status_effects import NoAbility


class Slayer(Character):
    """Slayer character."""

    def __init__(self):
        """Initialize the Slayer character."""
        self.name = "Slayer"
        self.description = (
            "Once per game, during the day, publicly choose a player: "
            + "if they are the Demon, they die."
        )
        self.category = CharacterType.TOWNSFOLK
        self.alignment = Alignment.GOOD
        self.status_effects = [NoAbility()]
