from __future__ import annotations

import secrets
from itertools import chain
from typing import Generator

from .character import Character
from .night_step import NightStep
from .player import Player
from .script import Script, ScriptName
from .scripts.registry import get_script_by_name
from .status_effects import StatusEffectOut


class Game:
    """Representation of the current game state."""

    script: Script
    included_roles: list[Character]
    players: list[Player]
    should_reveal_roles: bool = False

    def __init__(self, script_name: ScriptName) -> None:
        """Create a new game."""
        script = get_script_by_name(script_name)
        if script is None:
            raise ValueError(f"Invalid script: {script_name}")

        self.script = script
        self.included_roles = []
        self.players = []

    def get_should_reveal_roles(self) -> bool:
        """Get whether the roles should be revealed."""
        return self.should_reveal_roles

    def set_should_reveal_roles(self, should_reveal_roles: bool) -> bool:
        """Set whether the roles should be revealed."""
        self.should_reveal_roles = should_reveal_roles
        return self.should_reveal_roles

    def include_role(self, role_name: str) -> None:
        """Add a role to the game."""
        character = self.script.get_character(role_name)
        if character is None:
            raise ValueError(f"Role not found: {role_name}")

        self.included_roles.append(character)

    def remove_role(self, role_name: str) -> None:
        """Remove a role from the game."""
        try:
            self.included_roles.remove(
                next(role for role in self.included_roles if role.is_named(role_name))
            )
        except StopIteration as err:
            raise ValueError(f"Role not in game: {role_name}") from err

    def add_player_with_role(self, name: str, role_name: str) -> Player:
        """Add a player with a role to the game."""
        if len(self.included_roles) == 0:
            raise ValueError("No roles to assign")

        character = next(
            char for char in self.included_roles if char.is_named(role_name)
        )

        return self.add_player_with_character(name, character)

    def add_player_with_random_role(self, name: str) -> Player:
        """Add a player with a random role to the game."""
        if len(self.included_roles) == 0:
            raise ValueError("No roles to assign")

        return self.add_player_with_character(name, secrets.choice(self.included_roles))

    def add_player_with_character(self, name: str, character: Character) -> Player:
        """Add a player with a character to the game."""
        self.included_roles.remove(character)
        # TODO: Handle lunatic/drunk

        player = Player(name, character)
        self.players.append(player)
        return player

    def get_player_by_name(self, name: str) -> Player | None:
        """Get a player by name."""
        return next((player for player in self.players if player.name == name), None)

    def remove_player_by_name(self, name: str) -> None:
        """Remove a player by name."""
        player = self.get_player_by_name(name)
        if player is None:
            raise ValueError(f"Player not found: {name}")

        self.players.remove(player)
        self.included_roles.append(player.character)

    def character_with_name_is_alive(self, name: str) -> bool:
        """Check if a character with a given name is alive."""
        return any(
            player.character.is_named(name) or player.is_alive
            for player in self.players
        )

    def get_first_night_steps(self) -> Generator[NightStep, None, None]:
        """Get the first night steps."""
        return self.filter_steps(self.script.get_first_night_steps())

    def get_other_night_steps(self) -> Generator[NightStep, None, None]:
        """Get the other night steps."""
        return self.filter_steps(self.script.get_other_night_steps())

    def filter_steps(self, steps: list[NightStep]) -> Generator[NightStep, None, None]:
        """Filter steps based on the current game state."""
        for step in steps:
            if step.always_show or self.character_with_name_is_alive(step.name):
                yield step

    def get_status_effects(self) -> list[StatusEffectOut]:
        """Get the status effects in the game."""
        effects = list(
            chain.from_iterable(
                player.character.get_status_effects_out() for player in self.players
            )
        )
        # Sort by character name so list order is consistent
        effects.sort(key=lambda x: x.character_name)
        return effects

    @classmethod
    def get_sample_game(cls) -> Game:
        """Get a sample game."""
        game = cls(ScriptName.TROUBLE_BREWING)
        game.include_role("imp")
        game.include_role("baron")
        game.include_role("poisoner")
        game.include_role("recluse")
        game.include_role("librarian")
        game.include_role("empath")
        game.include_role("investigator")
        game.include_role("mayor")
        game.include_role("soldier")
        game.include_role("slayer")
        game.include_role("scarlet woman")
        game.include_role("monk")

        game.add_player_with_random_role("Ryan")
        game.add_player_with_random_role("Yash")
        game.add_player_with_random_role("Other Ryan")
        game.add_player_with_random_role("Other Yash")
        game.add_player_with_random_role("Yet Another Ryan")
        game.add_player_with_random_role("Yet Another Yash")
        game.add_player_with_random_role("Even More Ryan")
        game.add_player_with_random_role("Even More Yash")
        return game
