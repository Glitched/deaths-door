"""Tests for game state management using immutable GameState."""

from datetime import datetime, timezone

import pytest

from deaths_door.apply import apply
from deaths_door.events import (
    FirstNightSet,
    GameEvent,
    NightStepSet,
    PlayerAliveSet,
    PlayerRemoved,
    RoleVisibilitySet,
    TravelerAdded,
)
from deaths_door.game_state import GameState

from .helpers import GameTestCase, create_empty_game_state


def _evt(state: GameState, payload: object) -> GameEvent:
    """Shorthand to create a test event."""
    return GameEvent(
        game_id=state.game_id,
        sequence=state.version,
        timestamp=datetime.now(timezone.utc),
        payload=payload,  # type: ignore[reportArgumentType]
    )


@pytest.mark.anyio
async def test_add_player_with_specific_role():
    """Test adding a player with a specific role."""
    test_case = GameTestCase()
    test_case.add_players_with_roles([("Ryan", "Imp")])

    player = test_case.state.get_player("Ryan")
    assert player is not None
    assert player.character_name == "Imp"
    assert player.alignment == "evil"
    assert player.is_alive is True
    assert len(test_case.state.players) == 1
    # Role was removed from pool
    assert "Imp" not in test_case.state.included_role_names


@pytest.mark.anyio
async def test_add_player_with_nonexistent_role():
    """Test that adding a player with non-existent role raises error."""
    test_case = GameTestCase()

    with pytest.raises(ValueError, match="Character not found: NonExistentRole"):
        test_case.add_players_with_roles([("Alice", "NonExistentRole")])


@pytest.mark.anyio
async def test_add_traveler():
    """Test adding a traveler character."""
    state = create_empty_game_state()
    state = apply(
        state,
        _evt(state, TravelerAdded(player_name="Traveler_Alice", traveler_name="Beggar", alignment="unknown")),
    )

    player = state.get_player("Traveler_Alice")
    assert player is not None
    assert player.character_name == "Beggar"
    assert player.alignment == "unknown"
    assert len(state.players) == 1


@pytest.mark.anyio
async def test_has_living_character_named():
    """Test checking for living characters by name."""
    test_case = GameTestCase()
    test_case.add_players_with_roles([("Evil_Alice", "Imp")])

    assert test_case.state.has_living_character_named("Imp") is True
    assert test_case.state.has_living_character_named("Chef") is False

    # Kill the Imp
    test_case.state = apply(
        test_case.state,
        _evt(test_case.state, PlayerAliveSet(player_name="Evil_Alice", is_alive=False)),
    )

    assert test_case.state.has_living_character_named("Imp") is False


@pytest.mark.anyio
async def test_get_unclaimed_travelers():
    """Test getting travelers that haven't been claimed yet."""
    state = create_empty_game_state()
    script = state.get_script()
    all_travelers = script.travelers
    unclaimed = state.get_unclaimed_travelers()
    assert len(unclaimed) == len(all_travelers)

    # Claim one traveler
    state = apply(
        state,
        _evt(state, TravelerAdded(player_name="Alice", traveler_name="Beggar", alignment="unknown")),
    )

    unclaimed_after = state.get_unclaimed_travelers()
    assert len(unclaimed_after) == len(all_travelers) - 1
    assert not any(t.name == "Beggar" for t in unclaimed_after)


@pytest.mark.anyio
async def test_night_steps_filtering():
    """Test that night steps are filtered based on living characters."""
    test_case = GameTestCase()
    test_case.add_players_with_roles([("Chef_Player", "Chef"), ("Imp_Player", "Imp")])

    # Get first night steps (Chef has first night ability)
    first_night_state = test_case.state.model_copy(update={"is_first_night": True})
    first_night_steps = first_night_state.get_night_steps()
    chef_steps = [s for s in first_night_steps if s.name == "Chef"]
    assert len(chef_steps) > 0

    # Get other night steps (Imp has night ability)
    other_night_state = test_case.state.model_copy(update={"is_first_night": False})
    other_night_steps = other_night_state.get_night_steps()
    imp_steps = [s for s in other_night_steps if s.name == "Imp"]
    assert len(imp_steps) > 0

    # Kill the Chef
    state = apply(
        test_case.state,
        _evt(test_case.state, PlayerAliveSet(player_name="Chef_Player", is_alive=False)),
    )
    dead_chef_state = state.model_copy(update={"is_first_night": False})
    updated_steps = dead_chef_state.get_night_steps()
    chef_other_steps = [s for s in updated_steps if s.name == "Chef"]
    assert len(chef_other_steps) == 0


@pytest.mark.anyio
async def test_role_reveal_visibility():
    """Test role reveal visibility toggle."""
    state = create_empty_game_state()
    assert state.should_reveal_roles is False

    state = apply(state, _evt(state, RoleVisibilitySet(should_reveal_roles=True)))
    assert state.should_reveal_roles is True


@pytest.mark.anyio
async def test_player_removal():
    """Test removing players from the game."""
    test_case = GameTestCase(roles=["Imp", "Chef"])
    test_case.add_players_with_roles([("Alice", "Imp"), ("Bob", "Chef")])

    assert len(test_case.state.players) == 2
    original_roles_count = len(test_case.state.included_role_names)

    # Remove Alice
    test_case.state = apply(
        test_case.state,
        _evt(test_case.state, PlayerRemoved(player_name="Alice")),
    )

    assert len(test_case.state.players) == 1
    assert test_case.state.get_player("Alice") is None
    assert test_case.state.get_player("Bob") is not None
    assert len(test_case.state.included_role_names) == original_roles_count + 1


@pytest.mark.anyio
async def test_player_removal_by_name():
    """Test removing players by name."""
    test_case = GameTestCase(roles=["Imp", "Chef"])
    test_case.add_players_with_roles([("Alice", "Imp"), ("Bob", "Chef")])

    test_case.state = apply(
        test_case.state,
        _evt(test_case.state, PlayerRemoved(player_name="Alice")),
    )
    assert len(test_case.state.players) == 1
    assert test_case.state.get_player("Alice") is None


@pytest.mark.anyio
async def test_living_player_count():
    """Test living player count updates as players die."""
    test_case = GameTestCase(roles=["Imp", "Chef", "Monk"])
    test_case.add_players_with_roles([("Alice", "Imp"), ("Bob", "Chef"), ("Charlie", "Monk")])

    assert test_case.state.living_player_count == 3

    test_case.state = apply(
        test_case.state,
        _evt(test_case.state, PlayerAliveSet(player_name="Alice", is_alive=False)),
    )
    assert test_case.state.living_player_count == 2

    test_case.state = apply(
        test_case.state,
        _evt(test_case.state, PlayerAliveSet(player_name="Bob", is_alive=False)),
    )
    assert test_case.state.living_player_count == 1


@pytest.mark.anyio
async def test_execution_threshold():
    """Test execution threshold is ceil(living / 2)."""
    test_case = GameTestCase(roles=["Imp", "Chef", "Monk", "Empath", "Poisoner"])
    test_case.add_players_with_roles(
        [("Alice", "Imp"), ("Bob", "Chef"), ("Charlie", "Monk"), ("Dave", "Empath"), ("Eve", "Poisoner")]
    )

    assert test_case.state.execution_threshold == 3  # 5 living

    test_case.state = apply(test_case.state, _evt(test_case.state, PlayerAliveSet(player_name="Alice", is_alive=False)))
    assert test_case.state.execution_threshold == 2  # 4 living

    test_case.state = apply(test_case.state, _evt(test_case.state, PlayerAliveSet(player_name="Bob", is_alive=False)))
    assert test_case.state.execution_threshold == 2  # 3 living

    test_case.state = apply(
        test_case.state, _evt(test_case.state, PlayerAliveSet(player_name="Charlie", is_alive=False))
    )
    assert test_case.state.execution_threshold == 1  # 2 living


@pytest.mark.anyio
async def test_dead_players_with_vote():
    """Test tracking dead players who still have their vote."""
    state = GameState.model_validate(
        {
            "game_id": "00000000-0000-0000-0000-000000000000",
            "script_name": "trouble_brewing",
            "players": [
                {"name": "Alice", "character_name": "Imp", "alignment": "evil"},
                {"name": "Bob", "character_name": "Chef", "alignment": "good"},
                {"name": "Charlie", "character_name": "Monk", "alignment": "good"},
            ],
        }
    )

    assert state.get_dead_players_with_vote() == []

    # Kill Alice
    state = apply(state, _evt(state, PlayerAliveSet(player_name="Alice", is_alive=False)))
    assert state.get_dead_players_with_vote() == ["Alice"]

    # Alice uses her vote
    state = state.replace_player("Alice", has_used_dead_vote=True)
    assert state.get_dead_players_with_vote() == []

    # Kill Bob
    state = apply(state, _evt(state, PlayerAliveSet(player_name="Bob", is_alive=False)))
    assert state.get_dead_players_with_vote() == ["Bob"]


@pytest.mark.anyio
async def test_night_phase_defaults():
    """Test that night phase fields have correct default values."""
    state = create_empty_game_state()
    assert state.current_night_step == "Dusk"
    assert state.is_first_night is True


@pytest.mark.anyio
async def test_night_phase_modification():
    """Test modifying night phase fields via events."""
    state = create_empty_game_state()

    state = apply(state, _evt(state, NightStepSet(step="Poisoner")))
    assert state.current_night_step == "Poisoner"

    state = apply(state, _evt(state, FirstNightSet(is_first_night=False)))
    assert state.is_first_night is False
    assert state.current_night_step == "Dusk"  # Reset on toggle
