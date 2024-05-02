from __future__ import annotations

import copy
from dataclasses import dataclass

from .script import Category, Script, ScriptName


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
    roles: set[str]

    def __init__(self, player_count: int, script_name: ScriptName) -> None:
        """Create a new game."""
        dist = distributions.get(player_count)
        if dist is None:
            raise ValueError(f"Invalid number of players: {player_count}")
        self.base_role_distribution = dist

        self.script = Script(script_name)
        self.roles = set()

    def add_role(self, role_name: str) -> None:
        """Add a role to the game."""
        if not self.script.has_role(role_name):
            raise ValueError(f"Role not found: {role_name}")

        if role_name in self.roles:
            raise ValueError(f"Role already in game: {role_name}")

        self.roles.add(role_name)

    def remove_role(self, role_name: str) -> None:
        """Remove a role from the game."""
        if role_name not in self.roles:
            raise ValueError(f"Role not in game: {role_name}")

        self.roles.remove(role_name)

    def get_free_space(self) -> RoleDistribution:
        """Get the number of roles that can be added to the game."""
        current_role_counts = self.get_current_role_counts()
        base = copy.copy(self.base_role_distribution)

        for role_name in self.roles:
            role = self.script.get_role(role_name)
            # role is known not to be none
            if role is None:
                raise ValueError("Invariant Broken: invalid role: {role_name}")

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

        for role_name in self.roles:
            role = self.script.get_role(role_name)
            # role is known not to be none
            if role is None:
                raise ValueError("Invariant Broken: invalid role: {role_name}")

            match role.category:
                case Category.TOWNSFOLK:
                    current_roles.townsfolk += 1
                case Category.OUTSIDER:
                    current_roles.outsiders += 1
                case Category.MINION:
                    current_roles.minions += 1
                case Category.DEMON:
                    current_roles.demons += 1

        return current_roles
