from __future__ import annotations

import secrets

from .character import Character
from .player import Player
from .script import Script, ScriptName
from .scripts.registry import get_script_by_name


class Game:
    """Representation of the current game state."""

    script: Script
    included_roles: list[Character]
    players: list[Player]

    def __init__(self, script_name: ScriptName) -> None:
        """Create a new game."""
        script = get_script_by_name(script_name)
        if script is None:
            raise ValueError(f"Invalid script: {script_name}")

        self.script = script
        self.included_roles = []
        self.players = []

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
        character = next(
            char for char in self.included_roles if char.is_named(role_name)
        )
        player = Player(name, character)
        self.players.append(player)
        # TODO: Don't double assign roles
        # TODO: Reveal another character instead of drunk
        # TODO: Reveal the demon instead of the lunatic
        return player

    def add_player_with_random_role(self, name: str) -> Player:
        """Add a player with a random role to the game."""
        player = Player(name, secrets.choice(self.included_roles))
        self.players.append(player)
        # TODO: Reveal another character instead of drunk
        # TODO: Reveal the demon instead of the lunatic
        return player

    def get_player_by_name(self, name: str) -> Player:
        """Get a player by name."""
        return next(player for player in self.players if player.name == name)

    @classmethod
    def get_sample_game(cls) -> Game:
        """Get a sample game."""
        game = cls(ScriptName.TROUBLE_BREWING)
        game.include_role("imp")
        game.include_role("baron")
        game.include_role("poisoner")
        game.include_role("washerwoman")
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
        return game
