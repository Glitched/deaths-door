from pydantic import BaseModel

from .alignment import Alignment
from .character import Character, CharacterOut


class PlayerOut(BaseModel):
    """Player outgoing data."""

    name: str
    character: CharacterOut
    alignment: Alignment
    is_alive: bool
    has_used_dead_vote: bool
    status_effects: list[str]


class Player:
    """Player represents a human playing the game."""

    name: str
    character: Character
    alignment: Alignment
    is_alive: bool = True
    has_used_dead_vote: bool = False
    status_effects: list[str] = []

    def __init__(self, name: str, character: Character) -> None:
        """Create a new player, getting their alignment from the Character."""
        self.name = name
        self.character = character
        self.alignment = character.get_alignment()

    def set_name(self, name: str) -> None:
        """Set the player's name."""
        self.name = name

    def set_has_used_dead_vote(self, has_used_dead_vote: bool) -> None:
        """Mark a player as having used their dead vote."""
        self.has_used_dead_vote = has_used_dead_vote

    def set_alignment(self, alignment: Alignment) -> None:
        """Mark a player as having used their dead vote."""
        self.alignment = alignment

    def set_is_alive(self, is_alive: bool) -> None:
        """Set the player's alive status."""
        self.is_alive = is_alive

    def set_character(self, character: Character) -> None:
        """Set the player's character."""
        self.character = character

    def add_status_effect(self, status_effect: str) -> None:
        """Add a status effect to the player."""
        self.status_effects.append(status_effect)

    def remove_status_effect(self, status_effect: str) -> None:
        """Remove a status effect from the player."""
        self.status_effects.remove(status_effect)

    def to_out(self) -> PlayerOut:
        """Convert a player to outgoing data."""
        return PlayerOut(
            name=self.name,
            character=self.character.to_out(),
            alignment=self.alignment,
            is_alive=self.is_alive,
            has_used_dead_vote=self.has_used_dead_vote,
            status_effects=self.status_effects,
        )

    def __repr__(self) -> str:
        """Return a string representation of the player."""
        return f"Player({self.name}, {self.character.get_name()})"
