from pydantic import BaseModel, Field

from .alignment import Alignment
from .changes import Changes
from .character_type import CharacterType
from .status_effects import StatusEffect, StatusEffectOut


class CharacterOut(BaseModel):
    """A character with only fields meant to be sent to the client."""

    name: str = Field(
        ...,
        description="Character's display name",
        examples=["Imp", "Chef", "Empath", "Baron"],
    )
    description: str = Field(
        ...,
        description="Character's ability description",
        examples=[
            "Each night*, choose a player: they die. If you kill yourself this way, a Minion becomes the Imp."
        ],
    )
    icon_path: str = Field(
        ...,
        description="Path to the character's icon image",
        examples=["imp.png", "chef.png"],
    )
    alignment: Alignment = Field(
        ...,
        description="Character's alignment (good or evil)",
        examples=["evil", "good"],
    )
    category: CharacterType = Field(
        ...,
        description="Character's type/category",
        examples=["demon", "townsfolk", "outsider", "minion", "traveler"],
    )


class Character:
    """A character in the script."""

    name: str
    description: str
    category: CharacterType
    alignment: Alignment
    status_effects: list[StatusEffect]
    changes: None | Changes

    def __str__(self) -> str:
        """Return a human-readable string representation."""
        return f"{self.name} ({self.category.value})"

    def __repr__(self) -> str:
        """Return a detailed string representation."""
        return (
            f"Character(name={self.name!r}, "
            f"category={self.category!r}, "
            f"alignment={self.alignment!r})"
        )

    def normalize_name_for_comparison(self, name: str) -> str:
        """Convert name to lowercase and remove whitespace for comparison."""
        return name.lower().strip()

    def is_named(self, name: str) -> bool:
        """Check if the character matches the given name."""
        return self.normalize_name_for_comparison(
            self.name
        ) == self.normalize_name_for_comparison(name)

    def get_icon_path(self) -> str:
        """Return the character's icon."""
        return self.name.lower().replace(" ", "") + ".png"

    def get_status_effects_out(self) -> list[StatusEffectOut]:
        """Return the character's status effects."""
        return [
            status_effect.to_out(self.name) for status_effect in self.status_effects
        ]

    def to_out(self) -> CharacterOut:
        """Convert the character to a character out."""
        return CharacterOut(
            name=self.name,
            description=self.description,
            icon_path=self.get_icon_path(),
            alignment=self.alignment,
            category=self.category,
        )
