from ...character import Character
from ...script import Alignment, CharacterType


class Butler(Character):
    """Butler character."""

    def __init__(self):
        """Initialize the Butler character."""
        self.name = "Butler"
        self.description = (
            "Each night, choose a player (not yourself): tomorrow,"
            + " you may only vote if they are."
            + " You cannot be drunk or poisoned."
        )
        self.category = CharacterType.OUTSIDER
        self.alignment = Alignment.GOOD
        self.status_effects = []