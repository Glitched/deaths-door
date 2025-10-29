from pydantic import BaseModel, Field

from .alignment import Alignment
from .character import Character, CharacterOut


class PlayerOut(BaseModel):
    """Player outgoing data."""

    name: str = Field(
        ...,
        description="Player's display name",
        examples=["Alice", "Bob", "Charlie"],
    )
    character: CharacterOut = Field(
        ...,
        description="Character assigned to this player",
    )
    alignment: Alignment = Field(
        ...,
        description="Player's current alignment (can change from character's base alignment)",
        examples=["good", "evil", "unknown"],
    )
    is_alive: bool = Field(
        ...,
        description="Whether the player is currently alive",
        examples=[True, False],
    )
    has_used_dead_vote: bool = Field(
        ...,
        description="Whether the dead player has used their single vote",
        examples=[False, True],
    )
    status_effects: list[str] = Field(
        ...,
        description="List of active status effects on the player",
        examples=[["Poisoned"], ["Safe", "Drunk"], []],
    )


class Player:
    """Player represents a human playing the game."""

    name: str
    character: Character
    alignment: Alignment
    is_alive: bool = True
    has_used_dead_vote: bool = False
    status_effects: list[str]

    def __str__(self) -> str:
        """Return a human-readable string representation."""
        status = "dead" if not self.is_alive else "alive"
        return f"{self.name} as {self.character.name} ({status})"

    def __repr__(self) -> str:
        """Return a detailed string representation."""
        return (
            f"Player(name={self.name!r}, "
            f"character={self.character.name!r}, "
            f"is_alive={self.is_alive}, "
            f"alignment={self.alignment!r})"
        )

    def __init__(self, name: str, character: Character) -> None:
        """Create a new player, getting their alignment from the Character."""
        self.name = name
        self.character = character
        self.alignment = character.alignment
        self.status_effects = []

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
        if status_effect not in self.status_effects:
            self.status_effects.append(status_effect)

    def remove_status_effect(self, status_effect: str) -> None:
        """Remove a status effect from the player."""
        try:
            self.status_effects.remove(status_effect)
        except ValueError:
            pass  # Effect wasn't present, which is fine

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
