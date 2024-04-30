from __future__ import annotations

from dataclasses import dataclass

from .script import Category, Role, Script, ScriptName


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
    roles: list[Role]

    def __init__(self, player_count: int, script_name: ScriptName) -> None:
        """Create a new game."""
        dist = distributions.get(player_count)
        if dist is None:
            raise ValueError(f"Invalid number of players: {player_count}")
        self.base_role_distribution = dist

        self.script = Script(script_name)
        self.roles = []

    def get_remaining_roles(self) -> set[Role]:
        """Get the roles that are not yet in the game."""
        return set(self.script.roles) - set(self.roles)

    def add_role(self, role_name: str) -> None:
        """Add a role to the game."""
        role = self.script.get_role(role_name)
        if role is None:
            raise ValueError(f"Role not found: {role_name}")
        self.roles.append(role)

    def remove_role(self, role_name: str) -> None:
        """Remove a role from the game."""
        for role in self.roles:
            if role.name == role_name:
                self.roles.remove(role)
                break

    def get_free_space(self) -> RoleDistribution:
        """Get the number of roles that can be added to the game."""
        current_role_counts = self.get_current_role_counts()

        for role in self.roles:
            if role.changes is not None:
                change = role.changes
                if change.townsfolk is not None:
                    current_role_counts.townsfolk += change.townsfolk
                if change.outsider is not None:
                    current_role_counts.outsiders += change.outsider
                if change.minion is not None:
                    current_role_counts.minions += change.minion
                if change.demon is not None:
                    current_role_counts.demons += change.demon

        return RoleDistribution(
            townsfolk=self.base_role_distribution.townsfolk
            - current_role_counts.townsfolk,
            outsiders=self.base_role_distribution.outsiders
            - current_role_counts.outsiders,
            minions=self.base_role_distribution.minions - current_role_counts.minions,
            demons=self.base_role_distribution.demons - current_role_counts.demons,
        )

    def get_current_role_counts(self) -> RoleDistribution:
        """Get the number of roles that are currently in the game."""
        current_roles = RoleDistribution(townsfolk=0, outsiders=0, minions=0, demons=0)

        for role in self.roles:
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
