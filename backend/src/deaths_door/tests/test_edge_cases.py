"""Edge case tests for game state management."""

from datetime import datetime, timezone

import pytest
from httpx import ASGITransport, AsyncClient

from deaths_door.alignment import Alignment
from deaths_door.apply import apply
from deaths_door.events import GameEvent, PlayerAliveSet, StatusEffectAdded, StatusEffectRemoved, TravelerAdded
from deaths_door.game_state import GameState
from deaths_door.main import app
from deaths_door.scripts.registry import get_script_by_name

from .helpers import GameTestCase, create_empty_game_state, get_test_client, setup_game_with_roles


def _evt(state: GameState, payload: object) -> GameEvent:
    """Shorthand to create a test event."""
    return GameEvent(
        game_id=state.game_id,
        sequence=state.version,
        timestamp=datetime.now(timezone.utc),
        payload=payload,  # type: ignore[reportArgumentType]
    )


@pytest.mark.anyio
async def test_empty_game_operations():
    """Test operations on empty game."""
    state = create_empty_game_state()

    assert len(state.players) == 0
    assert state.get_player("Anyone") is None

    # All travelers unclaimed
    script = state.get_script()
    unclaimed = state.get_unclaimed_travelers()
    assert len(unclaimed) == len(script.travelers)

    # No status effects
    assert len(state.get_status_effects()) == 0

    # Night steps should still work (show always_show steps)
    first_night_state = state.model_copy(update={"is_first_night": True})
    first_night_steps = first_night_state.get_night_steps()
    always_first = [s for s in first_night_steps if s.always_show]
    assert len(always_first) > 0

    other_night_state = state.model_copy(update={"is_first_night": False})
    other_night_steps = other_night_state.get_night_steps()
    always_other = [s for s in other_night_steps if s.always_show]
    assert len(always_other) > 0


@pytest.mark.anyio
async def test_all_players_dead_scenario():
    """Test game state when all players are dead."""
    test_case = GameTestCase(roles=["Imp", "Chef", "Butler"])
    test_case.add_players_with_roles([("Alice", "Imp"), ("Bob", "Chef"), ("Charlie", "Butler")])

    # Kill all
    for p in test_case.state.players:
        event = _evt(test_case.state, PlayerAliveSet(player_name=p.name, is_alive=False))
        test_case.state = apply(test_case.state, event)

    assert test_case.state.has_living_character_named("Imp") is False
    assert test_case.state.has_living_character_named("Chef") is False
    assert test_case.state.has_living_character_named("Butler") is False

    # Only always_show steps
    other_state = test_case.state.model_copy(update={"is_first_night": False})
    other_steps = other_state.get_night_steps()
    character_steps = [s for s in other_steps if not s.always_show]
    assert len(character_steps) == 0


@pytest.mark.anyio
async def test_player_status_effect_edge_cases():
    """Test edge cases with player status effects."""
    test_case = GameTestCase(roles=["Chef"])
    test_case.add_players_with_roles([("Alice", "Chef")])

    # Add many effects
    effects = ["Poisoned", "Safe", "Used Dead Vote", "Protected", "Targeted"]
    for effect in effects:
        event = _evt(test_case.state, StatusEffectAdded(player_name="Alice", effect=effect))
        test_case.state = apply(test_case.state, event)

    alice = test_case.state.get_player("Alice")
    assert alice is not None
    assert len(alice.status_effects) == len(effects)

    # Remove effects one by one
    for effect in effects:
        event = _evt(test_case.state, StatusEffectRemoved(player_name="Alice", effect=effect))
        test_case.state = apply(test_case.state, event)

    alice = test_case.state.get_player("Alice")
    assert alice is not None
    assert len(alice.status_effects) == 0

    # Remove non-existent — should not raise
    event = _evt(test_case.state, StatusEffectRemoved(player_name="Alice", effect="NonExistent"))
    test_case.state = apply(test_case.state, event)


@pytest.mark.anyio
async def test_traveler_edge_cases():
    """Test edge cases with traveler management."""
    state = create_empty_game_state()
    script = state.get_script()
    all_travelers = script.travelers

    # Add all travelers
    for i, traveler in enumerate(all_travelers):
        state = apply(
            state,
            _evt(state, TravelerAdded(player_name=f"Player_{i}", traveler_name=traveler.name, alignment="unknown")),
        )

    # No unclaimed travelers left
    assert len(state.get_unclaimed_travelers()) == 0


@pytest.mark.anyio
async def test_api_error_handling():
    """Test API error handling with invalid requests."""
    async with get_test_client() as client:
        await setup_game_with_roles(client, roles=["Imp", "Chef", "Butler"])

        # Invalid JSON
        response = await client.post("/players/add", content="invalid json")
        assert response.status_code == 422

        # Missing required fields
        response = await client.post("/players/add", json={})
        assert response.status_code == 422

        # Invalid field types
        response = await client.post("/players/add", json={"name": 123})
        assert response.status_code == 422

        # Extremely long names
        long_name = "x" * 1000
        response = await client.post("/players/add", json={"name": long_name})
        assert response.status_code in [200, 400, 422]


@pytest.mark.anyio
async def test_concurrent_player_operations():
    """Test that concurrent operations don't break game state."""
    async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
        await client.post("/game/new", json={"script_name": "trouble_brewing"})
        await client.post("/characters/add/multi", json={"names": ["Imp", "Chef", "Butler"]})

        await client.post("/players/add", json={"name": "Alice"})

        operations = [
            client.post("/players/set_alive", json={"name": "Alice", "is_alive": False}),
            client.post("/players/add_status_effect", json={"name": "Alice", "status_effect": "Poisoned"}),
            client.get("/players/list"),
            client.post("/players/add_status_effect", json={"name": "Alice", "status_effect": "Safe"}),
        ]

        for operation in operations:
            response = await operation
            assert response.status_code in [200, 404]


@pytest.mark.anyio
async def test_script_boundary_conditions():
    """Test script-related boundary conditions."""
    script = get_script_by_name("trouble_brewing")
    assert script is not None

    assert len(script.characters) >= 10
    assert len(script.travelers) > 0
    assert len(script.get_first_night_steps()) > 0
    assert len(script.get_other_night_steps()) > 0

    for character in script.characters:
        assert character.name is not None
        assert len(character.name.strip()) > 0
        assert character.category is not None
        assert character.alignment is not None


@pytest.mark.anyio
async def test_alignment_consistency():
    """Test that character alignments are consistent with game rules."""
    script = get_script_by_name("trouble_brewing")
    assert script is not None

    good_chars = [c for c in script.characters if c.alignment == Alignment.GOOD]
    evil_chars = [c for c in script.characters if c.alignment == Alignment.EVIL]

    assert len(good_chars) > len(evil_chars)

    demons = [c for c in script.characters if c.category.value == "demon"]
    assert len(demons) >= 1
    for demon in demons:
        assert demon.alignment == Alignment.EVIL


@pytest.mark.anyio
async def test_game_state_consistency_after_operations():
    """Test that game state remains consistent after various operations."""
    async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as client:
        await client.post("/game/new", json={"script_name": "trouble_brewing"})
        await client.post("/characters/add/multi", json={"names": ["Imp", "Chef", "Butler", "Baron", "Librarian"]})

        await client.post("/players/add", json={"name": "Alice"})
        await client.post("/players/add", json={"name": "Bob"})
        await client.post("/players/add_traveler", json={"name": "Charlie", "traveler": "Beggar"})

        await client.post("/players/set_alive", json={"name": "Alice", "is_alive": False})
        await client.post("/players/set_alive", json={"name": "Alice", "is_alive": True})

        await client.post("/players/add_status_effect", json={"name": "Bob", "status_effect": "Poisoned"})
        await client.post("/players/remove_status_effect", json={"name": "Bob", "status_effect": "Poisoned"})

        await client.post("/players/remove", json={"name": "Bob"})
        await client.post("/players/add", json={"name": "Bob"})

        response = await client.get("/players/list")
        players = response.json()

        assert len(players) == 3
        names = [p["name"] for p in players]
        assert len(set(names)) == len(names)

        for player in players:
            assert player["character"]["name"] is not None
            assert player["character"]["name"] != ""
            assert isinstance(player["is_alive"], bool)
            assert isinstance(player["status_effects"], list)
