from pydantic import BaseModel

from .alignment import Alignment
from .changes import Changes
from .character_type import CharacterType


class CharacterOut(BaseModel):
    """A character with only fields meant to be sent to the client."""

    name: str
    description: str
    icon_path: str
    alignment: Alignment
    category: CharacterType


class StatusEffect:
    """A status effect that can be applied to a character."""

    name: str


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

    def get_status_effects(self) -> list[StatusEffect]:
        """Return the character's status effects."""
        return self.status_effects

    def normalize(self, s: str) -> str:
        """Normalize a character name for comparisons."""
        return s.lower().strip()

    def is_named(self, name: str) -> bool:
        """Check if the character matches the given name."""
        return self.normalize(self.name) == self.normalize(name)

    def get_icon_path(self) -> str:
        """Return the character's icon."""
        return self.name.lower().replace(" ", "") + ".png"

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
