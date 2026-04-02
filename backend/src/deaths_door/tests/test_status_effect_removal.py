"""Tests for status effect removal when characters die."""

from datetime import datetime, timezone

import pytest

from deaths_door.apply import apply
from deaths_door.events import GameEvent, PlayerAliveSet, StatusEffectAdded
from deaths_door.game_state import GameState
from deaths_door.routes.players import compute_death_cleared_effects

from .helpers import GameTestCase


def _evt(state: GameState, payload: object) -> GameEvent:
    """Shorthand to create a test event."""
    return GameEvent(
        game_id=state.game_id,
        sequence=state.version,
        timestamp=datetime.now(timezone.utc),
        payload=payload,  # type: ignore[reportArgumentType]
    )


def _add_effect(state: GameState, player_name: str, effect: str) -> GameState:
    """Add a status effect to a player."""
    return apply(state, _evt(state, StatusEffectAdded(player_name=player_name, effect=effect)))


def _kill_with_cleanup(state: GameState, player_name: str) -> GameState:
    """Kill a player with cascading status effect cleanup."""
    cleared = compute_death_cleared_effects(state, player_name)
    return apply(state, _evt(state, PlayerAliveSet(player_name=player_name, is_alive=False, cleared_effects=cleared)))


@pytest.mark.anyio
async def test_poisoner_death_removes_poisoned_status():
    """Test that when Poisoner dies, Poisoned status is removed from all players."""
    tc = GameTestCase(roles=["Poisoner", "Chef", "Empath"])
    tc.add_players_with_roles([("Alice", "Poisoner"), ("Bob", "Chef"), ("Charlie", "Empath")])

    tc.state = _add_effect(tc.state, "Bob", "Poisoned")
    tc.state = _add_effect(tc.state, "Charlie", "Poisoned")

    bob = tc.state.get_player("Bob")
    charlie = tc.state.get_player("Charlie")
    assert bob is not None and "Poisoned" in bob.status_effects
    assert charlie is not None and "Poisoned" in charlie.status_effects

    # Poisoner dies
    tc.state = _kill_with_cleanup(tc.state, "Alice")

    bob = tc.state.get_player("Bob")
    charlie = tc.state.get_player("Charlie")
    assert bob is not None and "Poisoned" not in bob.status_effects
    assert charlie is not None and "Poisoned" not in charlie.status_effects


@pytest.mark.anyio
async def test_monk_death_removes_safe_status():
    """Test that when Monk dies, Safe status is removed from all players."""
    tc = GameTestCase(roles=["Monk", "Chef", "Empath"])
    tc.add_players_with_roles([("Alice", "Monk"), ("Bob", "Chef"), ("Charlie", "Empath")])

    tc.state = _add_effect(tc.state, "Bob", "Safe")
    tc.state = _add_effect(tc.state, "Charlie", "Safe")

    tc.state = _kill_with_cleanup(tc.state, "Alice")

    bob = tc.state.get_player("Bob")
    charlie = tc.state.get_player("Charlie")
    assert bob is not None and "Safe" not in bob.status_effects
    assert charlie is not None and "Safe" not in charlie.status_effects


@pytest.mark.anyio
async def test_butler_death_removes_master_status():
    """Test that when Butler dies, Butler's Master status is removed from all players."""
    tc = GameTestCase(roles=["Butler", "Chef", "Empath"])
    tc.add_players_with_roles([("Alice", "Butler"), ("Bob", "Chef"), ("Charlie", "Empath")])

    tc.state = _add_effect(tc.state, "Bob", "Butler's Master")

    tc.state = _kill_with_cleanup(tc.state, "Alice")

    bob = tc.state.get_player("Bob")
    assert bob is not None and "Butler's Master" not in bob.status_effects


@pytest.mark.anyio
async def test_non_persistent_character_death_leaves_other_effects():
    """Test that when a non-persistent character dies, other status effects remain."""
    tc = GameTestCase(roles=["Imp", "Chef", "Empath"])
    tc.add_players_with_roles([("Alice", "Imp"), ("Bob", "Chef"), ("Charlie", "Empath")])

    tc.state = _add_effect(tc.state, "Bob", "Poisoned")
    tc.state = _add_effect(tc.state, "Charlie", "Safe")

    # Imp dies — has no persistent effects to clear
    tc.state = _kill_with_cleanup(tc.state, "Alice")

    bob = tc.state.get_player("Bob")
    charlie = tc.state.get_player("Charlie")
    assert bob is not None and "Poisoned" in bob.status_effects
    assert charlie is not None and "Safe" in charlie.status_effects


@pytest.mark.anyio
async def test_resurrection_does_not_trigger_cleanup():
    """Test that resurrecting a player doesn't trigger status effect cleanup."""
    tc = GameTestCase(roles=["Poisoner", "Chef"])
    tc.add_players_with_roles([("Alice", "Poisoner"), ("Bob", "Chef")])

    tc.state = _add_effect(tc.state, "Bob", "Poisoned")

    # Kill Poisoner — Poisoned removed
    tc.state = _kill_with_cleanup(tc.state, "Alice")
    bob = tc.state.get_player("Bob")
    assert bob is not None and "Poisoned" not in bob.status_effects

    # Re-poison and resurrect — effect should stay
    tc.state = _add_effect(tc.state, "Bob", "Poisoned")
    tc.state = apply(tc.state, _evt(tc.state, PlayerAliveSet(player_name="Alice", is_alive=True)))

    bob = tc.state.get_player("Bob")
    assert bob is not None and "Poisoned" in bob.status_effects


@pytest.mark.anyio
async def test_multiple_status_effects_partial_removal():
    """Test that death only removes relevant status effects, keeping others."""
    tc = GameTestCase(roles=["Poisoner", "Chef"])
    tc.add_players_with_roles([("Alice", "Poisoner"), ("Bob", "Chef")])

    tc.state = _add_effect(tc.state, "Bob", "Poisoned")
    tc.state = _add_effect(tc.state, "Bob", "Safe")
    tc.state = _add_effect(tc.state, "Bob", "Dead")

    # Poisoner dies — only Poisoned removed
    tc.state = _kill_with_cleanup(tc.state, "Alice")

    bob = tc.state.get_player("Bob")
    assert bob is not None
    assert "Poisoned" not in bob.status_effects
    assert "Safe" in bob.status_effects
    assert "Dead" in bob.status_effects
