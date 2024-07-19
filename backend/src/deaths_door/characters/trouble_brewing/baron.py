from ...character import Character
from ...script import Alignment, CharacterType


class Baron(Character):
    """Baron character."""

    def __init__(self):
        """Initialize the Baron character."""
        self.name = "Baron"
        self.description = "You have no ability. [There are 2 extra Outsiders in play]"
        self.category = CharacterType.MINION
        self.alignment = Alignment.EVIL
        self.status_effects = []

        # TODO: Implement changes
        self.changes = {"outsider": 2}
