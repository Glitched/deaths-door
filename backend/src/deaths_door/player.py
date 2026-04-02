"""Player API output model."""

from pydantic import BaseModel, Field

from .alignment import Alignment
from .character import CharacterOut


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
