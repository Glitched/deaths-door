from pydantic import BaseModel

from .alignment import Alignment
from .changes import Changes
from .character_type import CharacterType
from .status_effects import StatusEffect, StatusEffectOut


class CharacterOut(BaseModel):
    """A character with only fields meant to be sent to the client."""

    name: str
    description: str
    icon_path: str
    alignment: Alignment
    category: CharacterType


class Character:
    """A character in the script."""

    name: str
    description: str
    category: CharacterType
    alignment: Alignment
    status_effects: list[StatusEffect]
    changes: None | Changes

    def get_name(self) -> str:
        """Return the character's name."""
        return self.name

    def get_description(self) -> str:
        """Return the character's description."""
        return self.description

    def get_alignment(self) -> Alignment:
        """Return the character's alignment."""
        return self.alignment

    def get_category(self) -> CharacterType:
        """Return the character's category."""
        return self.category

    def normalize(self, s: str) -> str:
        """Normalize a character name for comparisons."""
        return s.lower().strip()

    def is_named(self, name: str) -> bool:
        """Check if the character matches the given name."""
        return self.normalize(self.name) == self.normalize(name)

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

    def __repr__(self) -> str:
        """Return a string representation of the character."""
        return f"Character({self.name})"
