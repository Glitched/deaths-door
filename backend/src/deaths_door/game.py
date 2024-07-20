from __future__ import annotations

import copy
from dataclasses import dataclass

from .character import Character
from .character_type import CharacterType
from .player import Player
from .script import Script, ScriptName
from .scripts.registry import get_script_by_name


@dataclass
class RoleDistribution:
    """How many of each role should be in the game."""

    townsfolk: int
    outsiders: int
    minions: int
    demons: int


distributions = {
    5: RoleDistribution(townsfolk=3, outsiders=0, minions=1, demons=1),
    6: RoleDistribution(townsfolk=3, outsiders=1, minions=1, demons=1),
    7: RoleDistribution(townsfolk=5, outsiders=0, minions=1, demons=1),
    8: RoleDistribution(townsfolk=5, outsiders=1, minions=1, demons=1),
    9: RoleDistribution(townsfolk=5, outsiders=2, minions=2, demons=1),
    10: RoleDistribution(townsfolk=7, outsiders=0, minions=2, demons=1),
    11: RoleDistribution(townsfolk=7, outsiders=1, minions=2, demons=1),
    12: RoleDistribution(townsfolk=7, outsiders=2, minions=2, demons=1),
    13: RoleDistribution(townsfolk=9, outsiders=0, minions=3, demons=1),
    14: RoleDistribution(townsfolk=9, outsiders=1, minions=3, demons=1),
    15: RoleDistribution(townsfolk=9, outsiders=2, minions=3, demons=1),
}


class Game:
    """Representation of the current game state."""

    script: Script
    base_role_distribution: RoleDistribution
    included_roles: list[Character]
    players: list[Player]

    def __init__(self, player_count: int, script_name: ScriptName) -> None:
        """Create a new game."""
        dist = distributions.get(player_count)
        if dist is None:
            raise ValueError(f"Invalid number of players: {player_count}")
        self.base_role_distribution = dist

        script = get_script_by_name(script_name)
        if script is None:
            raise ValueError(f"Invalid script: {script_name}")

        self.script = script
        self.included_roles = []

    def add__role(self, role_name: str) -> None:
        """Add a role to the game."""
        character = self.script.get_character(role_name)
        if character is None:
            raise ValueError(f"Role not found: {role_name}")

        for role in self.included_roles:
            if (
                role.get_category() == CharacterType.DEMON
                and role.is_named(role_name)
                and not role.is_named("legion")
            ):
                # TODO: Can any other roles not exist twice?
                # TODO: Add exception for legion
                raise ValueError(f"Role already in game: {role_name}")

        self.included_roles.append(character)

    def add_player_with_role(self, role_name: str) -> None:
        """Add a player with a role to the game."""
        character = next(
            char for char in self.included_roles if char.is_named(role_name)
        )
        self.players.append(Player(character))

    def remove_role(self, role_name: str) -> None:
        """Remove a role from the game."""
        for role in self.included_roles:
            if role.is_named(role_name):
                self.included_roles.remove(role)
                return

        raise ValueError(f"Role not in game: {role_name}")

    def get_open_slots(self) -> RoleDistribution:
        """Get the number of roles that can be added to the game."""
        current_role_counts = self.get_current_role_counts()
        base = copy.copy(self.base_role_distribution)

        # TODO: Support village idiot

        for player in self.players:
            role = player.character

            if role.changes is not None:
                change = role.changes
                if change.outsider is not None:
                    base.outsiders += change.outsider
                    base.townsfolk -= change.outsider
                if change.minion is not None:
                    base.minions += change.minion
                    base.townsfolk -= change.minion
                if change.demon is not None:
                    base.demons += change.demon
                    base.townsfolk -= change.demon

        return RoleDistribution(
            townsfolk=base.townsfolk - current_role_counts.townsfolk,
            outsiders=base.outsiders - current_role_counts.outsiders,
            minions=base.minions - current_role_counts.minions,
            demons=base.demons - current_role_counts.demons,
        )

    def get_current_role_counts(self) -> RoleDistribution:
        """Get the number of roles that are currently in the game."""
        current_roles = RoleDistribution(townsfolk=0, outsiders=0, minions=0, demons=0)

        for player in self.players:
            role = player.character

            match role.category:
                case CharacterType.TOWNSFOLK:
                    current_roles.townsfolk += 1
                case CharacterType.OUTSIDER:
                    current_roles.outsiders += 1
                case CharacterType.MINION:
                    current_roles.minions += 1
                case CharacterType.DEMON:
                    current_roles.demons += 1

        return current_roles
