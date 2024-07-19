from __future__ import annotations

from .script import Alignment, CharacterType


class StatusEffect:
    """A status effect that can be applied to a character."""


class Character:
    """A character in the script."""

    name: str
    description: str
    category: CharacterType
    alignment: Alignment
    status_effects: list[StatusEffect]

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

    def normalize_name(self, name: str) -> str:
        """Normalize a character name for comparisons."""
        return name.lower().strip()

    def is_named(self, name: str) -> bool:
        """Check if the character matches the given name."""
        return self.normalize_name(self.name) == self.normalize_name(name)
