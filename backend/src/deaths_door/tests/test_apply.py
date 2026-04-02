"""Tests for the pure apply function covering all 17 event types."""

from datetime import datetime, timezone
from uuid import uuid4

import pytest

from deaths_door.apply import apply, replay
from deaths_door.events import (
    CharactersSwapped,
    DeadVoteUsedSet,
    FirstNightSet,
    GameCreated,
    GameEvent,
    NightStepSet,
    PlayerAdded,
    PlayerAlignmentSet,
    PlayerAliveSet,
    PlayerRemoved,
    PlayerRenamed,
    RoleIncluded,
    RoleRemoved,
    RolesIncluded,
    RoleVisibilitySet,
    StatusEffectAdded,
    StatusEffectRemoved,
    TravelerAdded,
)
from deaths_door.game_state import GameState, PlayerState


def make_event(state: GameState, payload: object, sequence: int | None = None) -> GameEvent:
    """Create a GameEvent from a state and payload."""
    return GameEvent(
        game_id=state.game_id,
        sequence=sequence if sequence is not None else state.version,
        timestamp=datetime.now(timezone.utc),
        payload=payload,  # type: ignore[reportArgumentType]
    )


def make_game(
    roles: tuple[str, ...] = (),
    players: tuple[PlayerState, ...] = (),
) -> GameState:
    """Create a test GameState."""
    return GameState(
        game_id=uuid4(),
        script_name="trouble_brewing",
        included_role_names=roles,
        players=players,
    )


# --- Game lifecycle ---


def test_apply_game_created():
    """Test creating a new game."""
    state = GameState(game_id=uuid4(), script_name="")
    event = make_event(state, GameCreated(script_name="trouble_brewing"))
    new_state = apply(state, event)

    assert new_state.script_name == "trouble_brewing"
    assert new_state.version == 1
    assert new_state.players == ()
    assert new_state.included_role_names == ()


def test_apply_night_step_set():
    """Test setting the night step bookmark."""
    state = make_game()
    event = make_event(state, NightStepSet(step="Poisoner"))
    new_state = apply(state, event)

    assert new_state.current_night_step == "Poisoner"
    assert new_state.version == 1


def test_apply_first_night_set():
    """Test toggling first night resets step to Dusk."""
    state = make_game()
    # First set a non-Dusk step
    state = apply(state, make_event(state, NightStepSet(step="Imp")))
    assert state.current_night_step == "Imp"

    # Toggle first night — should reset to Dusk
    event = make_event(state, FirstNightSet(is_first_night=False))
    new_state = apply(state, event)

    assert new_state.is_first_night is False
    assert new_state.current_night_step == "Dusk"


def test_apply_role_visibility_set():
    """Test toggling role visibility."""
    state = make_game()
    event = make_event(state, RoleVisibilitySet(should_reveal_roles=True))
    new_state = apply(state, event)

    assert new_state.should_reveal_roles is True


# --- Role management ---


def test_apply_role_included():
    """Test including a single role."""
    state = make_game()
    event = make_event(state, RoleIncluded(name="Imp"))
    new_state = apply(state, event)

    assert new_state.included_role_names == ("Imp",)


def test_apply_roles_included():
    """Test including multiple roles at once."""
    state = make_game()
    event = make_event(state, RolesIncluded(names=("Imp", "Chef", "Butler")))
    new_state = apply(state, event)

    assert new_state.included_role_names == ("Imp", "Chef", "Butler")


def test_apply_role_removed():
    """Test removing a role from the pool."""
    state = make_game(roles=("Imp", "Chef", "Butler"))
    event = make_event(state, RoleRemoved(name="Chef"))
    new_state = apply(state, event)

    assert new_state.included_role_names == ("Imp", "Butler")


# --- Player management ---


def test_apply_player_added():
    """Test adding a player removes their character from the role pool."""
    state = make_game(roles=("Imp", "Chef"))
    event = make_event(state, PlayerAdded(player_name="Alice", character_name="Imp", alignment="evil"))
    new_state = apply(state, event)

    assert len(new_state.players) == 1
    assert new_state.players[0].name == "Alice"
    assert new_state.players[0].character_name == "Imp"
    assert new_state.players[0].alignment == "evil"
    assert new_state.players[0].is_alive is True
    assert "Imp" not in new_state.included_role_names
    assert "Chef" in new_state.included_role_names


def test_apply_traveler_added():
    """Test adding a traveler doesn't affect the role pool."""
    state = make_game(roles=("Imp", "Chef"))
    event = make_event(state, TravelerAdded(player_name="Bob", traveler_name="Beggar", alignment="unknown"))
    new_state = apply(state, event)

    assert len(new_state.players) == 1
    assert new_state.players[0].character_name == "Beggar"
    # Role pool unchanged
    assert new_state.included_role_names == ("Imp", "Chef")


def test_apply_player_removed():
    """Test removing a player returns their character to the pool."""
    state = make_game(
        roles=("Chef",),
        players=(PlayerState(name="Alice", character_name="Imp", alignment="evil"),),
    )
    event = make_event(state, PlayerRemoved(player_name="Alice"))
    new_state = apply(state, event)

    assert len(new_state.players) == 0
    assert "Imp" in new_state.included_role_names
    assert "Chef" in new_state.included_role_names


def test_apply_player_renamed():
    """Test renaming a player."""
    state = make_game(
        players=(PlayerState(name="Alice", character_name="Imp", alignment="evil"),),
    )
    event = make_event(state, PlayerRenamed(old_name="Alice", new_name="Alicia"))
    new_state = apply(state, event)

    assert new_state.get_player("Alicia") is not None
    assert new_state.get_player("Alice") is None


def test_apply_characters_swapped():
    """Test swapping characters between two players."""
    state = make_game(
        players=(
            PlayerState(name="Alice", character_name="Imp", alignment="evil"),
            PlayerState(name="Bob", character_name="Chef", alignment="good"),
        ),
    )
    event = make_event(state, CharactersSwapped(name1="Alice", name2="Bob"))
    new_state = apply(state, event)

    alice = new_state.get_player("Alice")
    bob = new_state.get_player("Bob")
    assert alice is not None and alice.character_name == "Chef"
    assert bob is not None and bob.character_name == "Imp"


# --- Player state ---


def test_apply_player_alive_set():
    """Test killing a player."""
    state = make_game(
        players=(PlayerState(name="Alice", character_name="Imp", alignment="evil"),),
    )
    event = make_event(state, PlayerAliveSet(player_name="Alice", is_alive=False))
    new_state = apply(state, event)

    alice = new_state.get_player("Alice")
    assert alice is not None and alice.is_alive is False


def test_apply_player_alive_set_with_cascading_effects():
    """Test that killing a character clears their persistent effects from other players."""
    state = make_game(
        players=(
            PlayerState(name="Alice", character_name="Poisoner", alignment="evil"),
            PlayerState(name="Bob", character_name="Chef", alignment="good", status_effects=("Poisoned",)),
            PlayerState(name="Charlie", character_name="Monk", alignment="good", status_effects=("Poisoned", "Safe")),
        ),
    )
    event = make_event(
        state,
        PlayerAliveSet(
            player_name="Alice",
            is_alive=False,
            cleared_effects=(("Bob", "Poisoned"), ("Charlie", "Poisoned")),
        ),
    )
    new_state = apply(state, event)

    bob = new_state.get_player("Bob")
    charlie = new_state.get_player("Charlie")
    assert bob is not None and "Poisoned" not in bob.status_effects
    assert charlie is not None and "Poisoned" not in charlie.status_effects
    assert "Safe" in charlie.status_effects  # Other effects untouched


def test_apply_dead_vote_used_set():
    """Test recording a dead vote."""
    state = make_game(
        players=(PlayerState(name="Alice", character_name="Imp", alignment="evil", is_alive=False),),
    )
    event = make_event(state, DeadVoteUsedSet(player_name="Alice", has_used_dead_vote=True))
    new_state = apply(state, event)

    alice = new_state.get_player("Alice")
    assert alice is not None and alice.has_used_dead_vote is True


def test_apply_player_alignment_set():
    """Test changing a player's alignment."""
    state = make_game(
        players=(PlayerState(name="Alice", character_name="Imp", alignment="evil"),),
    )
    event = make_event(state, PlayerAlignmentSet(player_name="Alice", alignment="good"))
    new_state = apply(state, event)

    alice = new_state.get_player("Alice")
    assert alice is not None and alice.alignment == "good"


# --- Status effects ---


def test_apply_status_effect_added():
    """Test adding a status effect."""
    state = make_game(
        players=(PlayerState(name="Alice", character_name="Chef", alignment="good"),),
    )
    event = make_event(state, StatusEffectAdded(player_name="Alice", effect="Poisoned"))
    new_state = apply(state, event)

    alice = new_state.get_player("Alice")
    assert alice is not None and "Poisoned" in alice.status_effects


def test_apply_status_effect_added_idempotent():
    """Test that adding a duplicate status effect is idempotent."""
    state = make_game(
        players=(PlayerState(name="Alice", character_name="Chef", alignment="good", status_effects=("Poisoned",)),),
    )
    event = make_event(state, StatusEffectAdded(player_name="Alice", effect="Poisoned"))
    new_state = apply(state, event)

    alice = new_state.get_player("Alice")
    assert alice is not None and alice.status_effects == ("Poisoned",)


def test_apply_status_effect_removed():
    """Test removing a status effect."""
    state = make_game(
        players=(PlayerState(name="Alice", character_name="Chef", alignment="good", status_effects=("Poisoned",)),),
    )
    event = make_event(state, StatusEffectRemoved(player_name="Alice", effect="Poisoned"))
    new_state = apply(state, event)

    alice = new_state.get_player("Alice")
    assert alice is not None and "Poisoned" not in alice.status_effects


def test_apply_status_effect_removed_missing():
    """Test that removing a non-existent effect is safe."""
    state = make_game(
        players=(PlayerState(name="Alice", character_name="Chef", alignment="good"),),
    )
    event = make_event(state, StatusEffectRemoved(player_name="Alice", effect="Poisoned"))
    new_state = apply(state, event)

    alice = new_state.get_player("Alice")
    assert alice is not None and alice.status_effects == ()


# --- Replay ---


def test_replay_full_game():
    """Test replaying a sequence of events builds correct state."""
    game_id = uuid4()
    now = datetime.now(timezone.utc)

    events = [
        GameEvent(game_id=game_id, sequence=0, timestamp=now, payload=GameCreated(script_name="trouble_brewing")),
        GameEvent(game_id=game_id, sequence=1, timestamp=now, payload=RolesIncluded(names=("Imp", "Chef", "Monk"))),
        GameEvent(
            game_id=game_id,
            sequence=2,
            timestamp=now,
            payload=PlayerAdded(player_name="Alice", character_name="Imp", alignment="evil"),
        ),
        GameEvent(
            game_id=game_id,
            sequence=3,
            timestamp=now,
            payload=PlayerAdded(player_name="Bob", character_name="Chef", alignment="good"),
        ),
        GameEvent(
            game_id=game_id,
            sequence=4,
            timestamp=now,
            payload=PlayerAliveSet(player_name="Bob", is_alive=False),
        ),
    ]

    state = replay(events)

    assert state.script_name == "trouble_brewing"
    assert len(state.players) == 2
    assert state.included_role_names == ("Monk",)
    assert state.version == 5

    alice = state.get_player("Alice")
    bob = state.get_player("Bob")
    assert alice is not None and alice.is_alive is True
    assert bob is not None and bob.is_alive is False


def test_replay_empty_raises():
    """Test that replaying empty events raises ValueError."""
    with pytest.raises(ValueError, match="Cannot replay empty event list"):
        replay([])


# --- Version tracking ---


def test_version_increments():
    """Test that version increments with each event."""
    state = make_game()
    assert state.version == 0

    state = apply(state, make_event(state, RoleIncluded(name="Imp")))
    assert state.version == 1

    state = apply(state, make_event(state, RoleIncluded(name="Chef")))
    assert state.version == 2


# --- GameState derived properties ---


def test_game_state_living_player_count():
    """Test living player count on GameState."""
    state = make_game(
        players=(
            PlayerState(name="Alice", character_name="Imp", alignment="evil"),
            PlayerState(name="Bob", character_name="Chef", alignment="good", is_alive=False),
            PlayerState(name="Charlie", character_name="Monk", alignment="good"),
        ),
    )
    assert state.living_player_count == 2
    assert state.execution_threshold == 1


def test_game_state_dead_players_with_vote():
    """Test dead players with vote tracking."""
    state = make_game(
        players=(
            PlayerState(name="Alice", character_name="Imp", alignment="evil", is_alive=False),
            PlayerState(name="Bob", character_name="Chef", alignment="good", is_alive=False, has_used_dead_vote=True),
            PlayerState(name="Charlie", character_name="Monk", alignment="good"),
        ),
    )
    assert state.get_dead_players_with_vote() == ["Alice"]
