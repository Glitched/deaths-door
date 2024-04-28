from __future__ import annotations

from dataclasses import dataclass

from .script import Script, ScriptName


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

    def __init__(self, player_count: int, script_name: ScriptName) -> None:
        """Create a new game."""
        dist = distributions.get(player_count)
        if dist is None:
            raise ValueError(f"Invalid number of players: {player_count}")
        self.base_role_distribution = dist

        self.script = Script(script_name)
