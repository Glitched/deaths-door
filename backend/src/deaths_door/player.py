from .alignment import Alignment
from .character import Character, StatusEffect


class Player:
    """Player represents a human playing the game."""

    character: Character
    alignment: Alignment
    current_status_effects: list[StatusEffect]
    is_alive: bool
    has_used_dead_vote: bool

    def __init__(self, character: Character) -> None:
        """Create a new player, getting their alignment from the Character."""
        self.character = character
        self.alignment = character.get_alignment()
        self.current_status_effects = []
        self.is_alive = True
        self.has_used_dead_vote = False

    def use_dead_vote(self) -> None:
        """Mark a player as having used their dead vote."""
        self.has_used_dead_vote = True

    def set_alignment(self, alignment: Alignment) -> None:
        """Mark a player as having used their dead vote."""
        self.alignment = alignment
