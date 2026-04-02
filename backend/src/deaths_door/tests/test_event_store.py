"""Tests for SQLite event store persistence."""

import tempfile
from datetime import datetime, timezone
from uuid import UUID, uuid4

import pytest

from deaths_door.event_store import EventStore
from deaths_door.events import EventPayload, GameCreated, GameEvent, PlayerAdded, RolesIncluded


@pytest.fixture
def store() -> EventStore:
    """Create an in-memory event store for testing."""
    return EventStore(db_path=":memory:")


def make_event(game_id: UUID, sequence: int, payload: EventPayload) -> GameEvent:
    """Create a test event."""
    return GameEvent(
        game_id=game_id,
        sequence=sequence,
        timestamp=datetime.now(timezone.utc),
        payload=payload,
    )


def test_append_and_get_events(store: EventStore):
    """Test appending and retrieving events."""
    game_id = uuid4()
    event = make_event(game_id, 0, GameCreated(script_name="trouble_brewing"))
    store.append(event)

    events = store.get_events(game_id)
    assert len(events) == 1
    assert events[0].game_id == game_id
    assert events[0].sequence == 0
    assert isinstance(events[0].payload, GameCreated)
    assert events[0].payload.script_name == "trouble_brewing"


def test_get_events_ordered(store: EventStore):
    """Test events are returned in sequence order."""
    game_id = uuid4()
    store.append(make_event(game_id, 0, GameCreated(script_name="trouble_brewing")))
    store.append(make_event(game_id, 1, RolesIncluded(names=("Imp", "Chef"))))
    store.append(make_event(game_id, 2, PlayerAdded(player_name="Alice", character_name="Imp", alignment="evil")))

    events = store.get_events(game_id)
    assert len(events) == 3
    assert [e.sequence for e in events] == [0, 1, 2]


def test_get_events_up_to_sequence(store: EventStore):
    """Test retrieving events up to a specific sequence."""
    game_id = uuid4()
    store.append(make_event(game_id, 0, GameCreated(script_name="trouble_brewing")))
    store.append(make_event(game_id, 1, RolesIncluded(names=("Imp", "Chef"))))
    store.append(make_event(game_id, 2, PlayerAdded(player_name="Alice", character_name="Imp", alignment="evil")))

    events = store.get_events(game_id, up_to_sequence=1)
    assert len(events) == 2
    assert events[-1].sequence == 1


def test_get_events_isolates_games(store: EventStore):
    """Test that events from different games don't mix."""
    game_a = uuid4()
    game_b = uuid4()
    store.append(make_event(game_a, 0, GameCreated(script_name="trouble_brewing")))
    store.append(make_event(game_b, 0, GameCreated(script_name="trouble_brewing")))

    assert len(store.get_events(game_a)) == 1
    assert len(store.get_events(game_b)) == 1


def test_get_latest_sequence(store: EventStore):
    """Test getting the latest sequence number."""
    game_id = uuid4()
    assert store.get_latest_sequence(game_id) == -1

    store.append(make_event(game_id, 0, GameCreated(script_name="trouble_brewing")))
    assert store.get_latest_sequence(game_id) == 0

    store.append(make_event(game_id, 1, RolesIncluded(names=("Imp",))))
    assert store.get_latest_sequence(game_id) == 1


def test_get_all_game_ids(store: EventStore):
    """Test listing all game IDs."""
    game_a = uuid4()
    game_b = uuid4()
    store.append(make_event(game_a, 0, GameCreated(script_name="trouble_brewing")))
    store.append(make_event(game_b, 0, GameCreated(script_name="trouble_brewing")))

    ids = store.get_all_game_ids()
    assert set(ids) == {game_a, game_b}


def test_delete_after_sequence(store: EventStore):
    """Test deleting events after a sequence number."""
    game_id = uuid4()
    store.append(make_event(game_id, 0, GameCreated(script_name="trouble_brewing")))
    store.append(make_event(game_id, 1, RolesIncluded(names=("Imp", "Chef"))))
    store.append(make_event(game_id, 2, PlayerAdded(player_name="Alice", character_name="Imp", alignment="evil")))

    deleted = store.delete_after_sequence(game_id, 0)
    assert deleted == 2

    events = store.get_events(game_id)
    assert len(events) == 1
    assert events[0].sequence == 0


def test_fork_game(store: EventStore):
    """Test forking a game copies events up to a point."""
    game_id = uuid4()
    store.append(make_event(game_id, 0, GameCreated(script_name="trouble_brewing")))
    store.append(make_event(game_id, 1, RolesIncluded(names=("Imp", "Chef"))))
    store.append(make_event(game_id, 2, PlayerAdded(player_name="Alice", character_name="Imp", alignment="evil")))

    # Fork after sequence 1 (before adding Alice)
    forked_id = store.fork_game(game_id, up_to_sequence=1)

    # Original unchanged
    assert len(store.get_events(game_id)) == 3

    # Fork has only first 2 events
    forked_events = store.get_events(forked_id)
    assert len(forked_events) == 2
    assert forked_events[0].game_id == forked_id
    assert isinstance(forked_events[0].payload, GameCreated)
    assert isinstance(forked_events[1].payload, RolesIncluded)


def test_roundtrip_all_payload_fields(store: EventStore):
    """Test that complex payloads survive serialization roundtrip."""
    game_id = uuid4()
    original = make_event(
        game_id,
        0,
        PlayerAdded(player_name="Alice", character_name="Imp", alignment="evil"),
    )
    store.append(original)

    loaded = store.get_events(game_id)[0]
    assert isinstance(loaded.payload, PlayerAdded)
    assert loaded.payload.player_name == "Alice"
    assert loaded.payload.character_name == "Imp"
    assert loaded.payload.alignment == "evil"


def test_store_replay_roundtrip(store: EventStore):
    """Test the full pipeline: persist events, load them, replay to rebuild state."""
    from deaths_door.apply import replay
    from deaths_door.events import PlayerAliveSet, StatusEffectAdded

    game_id = uuid4()
    events = [
        make_event(game_id, 0, GameCreated(script_name="trouble_brewing")),
        make_event(game_id, 1, RolesIncluded(names=("Imp", "Chef", "Monk"))),
        make_event(game_id, 2, PlayerAdded(player_name="Alice", character_name="Imp", alignment="evil")),
        make_event(game_id, 3, PlayerAdded(player_name="Bob", character_name="Chef", alignment="good")),
        make_event(game_id, 4, StatusEffectAdded(player_name="Bob", effect="Poisoned")),
        make_event(game_id, 5, PlayerAliveSet(player_name="Bob", is_alive=False)),
    ]
    for e in events:
        store.append(e)

    # Load and replay
    loaded = store.get_events(game_id)
    state = replay(loaded)

    assert state.script_name == "trouble_brewing"
    assert len(state.players) == 2
    assert state.included_role_names == ("Monk",)

    alice = state.get_player("Alice")
    bob = state.get_player("Bob")
    assert alice is not None and alice.is_alive is True
    assert bob is not None and bob.is_alive is False
    assert "Poisoned" in bob.status_effects

    # Partial replay (rewind to before Bob was killed)
    partial = store.get_events(game_id, up_to_sequence=4)
    rewound = replay(partial)
    bob_alive = rewound.get_player("Bob")
    assert bob_alive is not None and bob_alive.is_alive is True


def test_persistence_to_file():
    """Test that events persist to an actual file."""
    with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
        db_path = f.name

    game_id = uuid4()

    # Write
    store1 = EventStore(db_path=db_path)
    store1.append(make_event(game_id, 0, GameCreated(script_name="trouble_brewing")))
    store1.close()

    # Read from fresh connection
    store2 = EventStore(db_path=db_path)
    events = store2.get_events(game_id)
    store2.close()

    assert len(events) == 1
    assert isinstance(events[0].payload, GameCreated)
