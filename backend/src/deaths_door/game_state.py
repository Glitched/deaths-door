"""Immutable game state models for event sourcing."""

from __future__ import annotations

import math
from uuid import UUID

from pydantic import BaseModel

from .alignment import Alignment
from .character import Character, CharacterOut
from .night_step import NightStep
from .player import PlayerOut
from .script import Script
from .scripts.registry import get_script_by_name
from .status_effects import StatusEffectOut


class PlayerState(BaseModel, frozen=True):
    """Immutable snapshot of a player's state."""

    name: str
    character_name: str
    alignment: str
    is_alive: bool = True
    has_used_dead_vote: bool = False
    status_effects: tuple[str, ...] = ()


class GameState(BaseModel, frozen=True):
    """Immutable snapshot of the entire game state, derived from events."""

    game_id: UUID
    script_name: str
    included_role_names: tuple[str, ...] = ()
    players: tuple[PlayerState, ...] = ()
    should_reveal_roles: bool = False
    current_night_step: str = "Dusk"
    is_first_night: bool = True
    version: int = 0

    # --- Script lookup ---

    def get_script(self) -> Script:
        """Get the Script object for this game."""
        script = get_script_by_name(self.script_name)
        if script is None:
            raise ValueError(f"Unknown script: {self.script_name}")
        return script

    def get_character(self, name: str) -> Character | None:
        """Look up a character by name from the script."""
        return self.get_script().get_character(name)

    # --- Player lookups ---

    def get_player(self, name: str) -> PlayerState | None:
        """Get a player by name."""
        return next((p for p in self.players if p.name == name), None)

    def has_living_character_named(self, character_name: str) -> bool:
        """Check if any living player has the specified character."""
        normalized = character_name.lower().strip()
        return any(p.character_name.lower().strip() == normalized and p.is_alive for p in self.players)

    def has_dead_character_named(self, character_name: str) -> bool:
        """Check if any dead player has the specified character."""
        normalized = character_name.lower().strip()
        return any(p.character_name.lower().strip() == normalized and not p.is_alive for p in self.players)

    # --- Derived vote info ---

    @property
    def living_player_count(self) -> int:
        """Get the number of living players."""
        return sum(1 for p in self.players if p.is_alive)

    @property
    def execution_threshold(self) -> int:
        """Get the number of votes needed to execute."""
        return math.ceil(self.living_player_count / 2)

    def get_dead_players_with_vote(self) -> list[str]:
        """Get names of dead players who haven't used their dead vote."""
        return [p.name for p in self.players if not p.is_alive and not p.has_used_dead_vote]

    # --- Night steps ---

    def get_night_steps(self) -> list[NightStep]:
        """Get filtered night steps for the current phase."""
        script = self.get_script()
        if self.is_first_night:
            steps = script.get_first_night_steps()
        else:
            steps = script.get_other_night_steps()
        return list(self._filter_active_night_steps(steps))

    def _filter_active_night_steps(self, steps: list[NightStep]) -> list[NightStep]:
        """Filter night steps based on current game state."""
        result: list[NightStep] = []
        for step in steps:
            if step.always_show:
                result.append(step)
            elif step.show_when_dead and self.has_dead_character_named(step.name):
                result.append(step)
            elif self.has_living_character_named(step.name):
                result.append(step)
        return result

    # --- Status effects ---

    def get_status_effects(self) -> list[StatusEffectOut]:
        """Get all status effects from characters in the game."""
        script = self.get_script()
        effects: list[StatusEffectOut] = []
        for player in self.players:
            character = script.get_character(player.character_name)
            if character:
                effects.extend(character.get_status_effects_out())
        effects.sort(key=lambda e: e.character_name)
        return effects

    # --- Unclaimed travelers ---

    def get_unclaimed_travelers(self) -> list[Character]:
        """Get travelers that haven't been claimed by any player."""
        script = self.get_script()
        claimed = {p.character_name for p in self.players}
        return [t for t in script.travelers if t.name not in claimed]

    # --- Immutable update helpers ---

    def replace_player(self, player_name: str, **updates: object) -> GameState:
        """Return new GameState with one player's fields updated."""
        new_players = tuple(p.model_copy(update=updates) if p.name == player_name else p for p in self.players)
        return self.model_copy(update={"players": new_players})

    # --- Included roles as Character objects ---

    def get_included_roles(self) -> list[Character]:
        """Get Character objects for all included role names."""
        script = self.get_script()
        roles: list[Character] = []
        for name in self.included_role_names:
            char = script.get_character(name)
            if char:
                roles.append(char)
        return roles


def player_state_to_out(player: PlayerState, script: Script) -> PlayerOut:
    """Convert a PlayerState to the API response model."""
    # Search both characters and travelers
    character = script.get_character(player.character_name)
    if character is None:
        character = next((t for t in script.travelers if t.name == player.character_name), None)
    if character is None:
        raise ValueError(f"Character not found: {player.character_name}")
    return PlayerOut(
        name=player.name,
        character=character.to_out(),
        alignment=Alignment(player.alignment),
        is_alive=player.is_alive,
        has_used_dead_vote=player.has_used_dead_vote,
        status_effects=list(player.status_effects),
    )


def game_state_to_included_role_outs(state: GameState) -> list[CharacterOut]:
    """Convert included role names to CharacterOut API models."""
    return [c.to_out() for c in state.get_included_roles()]
