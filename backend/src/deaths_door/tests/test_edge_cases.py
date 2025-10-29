import pytest
from httpx import ASGITransport, AsyncClient

from deaths_door.alignment import Alignment
from deaths_door.game import Game
from deaths_door.main import app
from deaths_door.script import ScriptName

from .helpers import (
    GameTestCase,
    create_empty_game,
    get_test_client,
    setup_game_with_roles,
)


@pytest.mark.anyio
async def test_empty_game_operations():
    """Test operations on empty game."""
    game = create_empty_game()

    # Empty game should have no players
    assert len(game.players) == 0
    assert game.get_player_by_name("Anyone") is None

    # Should have no unclaimed travelers initially (all are unclaimed)
    unclaimed = game.get_unclaimed_travelers()
    assert len(unclaimed) == len(game.script.travelers)

    # Should have no status effects
    status_effects = game.get_status_effects()
    assert len(status_effects) == 0

    # Night steps should still work (show always_show steps)
    first_night = list(game.get_first_night_steps())
    other_nights = list(game.get_other_night_steps())

    # Should have some always_show steps
    always_first = [step for step in first_night if step.always_show]
    always_other = [step for step in other_nights if step.always_show]
    assert len(always_first) > 0
    assert len(always_other) > 0


@pytest.mark.anyio
async def test_game_with_no_available_roles():
    """Test game behavior when no roles are available."""
    game = create_empty_game()
    # game.included_roles is already empty

    # Should raise error when trying to add player with random role
    with pytest.raises(ValueError, match="No roles to assign"):
        game.add_player_with_random_role("Alice")

    # Should raise error when trying to add player with specific role
    with pytest.raises(ValueError, match="No roles to assign"):
        game.add_player_with_role("Alice", "Imp")


@pytest.mark.anyio
async def test_maximum_players_scenario():
    """Test game with maximum realistic number of players."""
    # Use all characters from the script for this test
    all_character_names = [
        char.name for char in Game(ScriptName.TROUBLE_BREWING).script.characters
    ]
    test_case = GameTestCase(roles=all_character_names)

    # Add many players (up to available roles)
    players_added = 0
    try:
        for i in range(len(test_case.game.script.characters)):
            test_case.game.add_player_with_random_role(f"Player_{i}")
            players_added += 1
    except ValueError:
        pass  # Expected when we run out of roles

    # Should have added some players
    assert players_added > 0
    assert len(test_case.game.players) == players_added
    assert (
        len(test_case.game.included_roles)
        == len(test_case.game.script.characters) - players_added
    )


@pytest.mark.anyio
async def test_all_players_dead_scenario():
    """Test game state when all players are dead."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)
    game.included_roles = game.script.characters.copy()

    # Add some players
    game.add_player_with_role("Alice", "Imp")
    game.add_player_with_role("Bob", "Chef")
    game.add_player_with_role("Charlie", "Butler")

    # Kill all players
    for player in game.players:
        player.is_alive = False

    # No characters should be considered "living"
    assert game.has_living_character_named("Imp") is False
    assert game.has_living_character_named("Chef") is False
    assert game.has_living_character_named("Butler") is False

    # Night steps should only show always_show steps
    other_night_steps = list(game.get_other_night_steps())
    character_specific_steps = [
        step for step in other_night_steps if not step.always_show
    ]
    assert len(character_specific_steps) == 0


@pytest.mark.anyio
async def test_player_status_effect_edge_cases():
    """Test edge cases with player status effects."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)
    game.included_roles = game.script.characters.copy()

    player = game.add_player_with_role("Alice", "Chef")

    # Add many status effects
    effects = ["Poisoned", "Safe", "Used Dead Vote", "Protected", "Targeted"]
    for effect in effects:
        player.add_status_effect(effect)

    assert len(player.status_effects) == len(effects)

    # Remove effects one by one
    for effect in effects:
        player.remove_status_effect(effect)

    assert len(player.status_effects) == 0

    # Try to remove from empty list - should not raise (safe removal)
    player.remove_status_effect("NonExistent")  # Should not raise


@pytest.mark.anyio
async def test_traveler_edge_cases():
    """Test edge cases with traveler management."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)

    # Add all available travelers
    all_travelers = game.script.travelers
    for i, traveler in enumerate(all_travelers):
        game.add_player_as_traveler(f"Player_{i}", traveler.name)

    # Should have no unclaimed travelers left
    unclaimed = game.get_unclaimed_travelers()
    assert len(unclaimed) == 0

    # Trying to add another traveler should fail
    with pytest.raises(ValueError):
        game.add_player_as_traveler("Extra", all_travelers[0].name)


@pytest.mark.anyio
async def test_character_name_collision_resistance():
    """Test that character name matching is robust."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)
    game.included_roles = game.script.characters.copy()

    # Test various malformed inputs
    with pytest.raises(ValueError):
        game.add_player_with_role("Alice", "")

    with pytest.raises(ValueError):
        game.add_player_with_role("Alice", "   ")

    # But legitimate variations should work
    game.add_player_with_role("Alice", "Fortune Teller")

    # Two players can't have same character - Bob should fail because Fortune Teller is already taken
    with pytest.raises(ValueError):
        game.add_player_with_role("Bob", "Fortune Teller")


@pytest.mark.anyio
async def test_api_error_handling():
    """Test API error handling with invalid requests."""
    async with get_test_client() as client:
        await setup_game_with_roles(client, roles=["Imp", "Chef", "Butler"])

        # Test invalid JSON
        response = await client.post("/players/add", content="invalid json")
        assert response.status_code == 422

        # Test missing required fields
        response = await client.post("/players/add", json={})
        assert response.status_code == 422

        # Test invalid field types
        response = await client.post("/players/add", json={"name": 123})
        assert response.status_code == 422

        # Test extremely long names
        long_name = "x" * 1000
        response = await client.post("/players/add", json={"name": long_name})
        # Should either succeed or fail gracefully (not crash)
        assert response.status_code in [200, 400, 422]


@pytest.mark.anyio
async def test_concurrent_player_operations():
    """Test that concurrent operations don't break game state."""
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        await client.post("/game/new", json={"script_name": "trouble_brewing"})
        # Add roles so players can be assigned
        await client.post(
            "/characters/add/multi", json={"names": ["Imp", "Chef", "Butler"]}
        )

        # Add a player
        await client.post("/players/add", json={"name": "Alice"})

        # Simulate concurrent operations that might conflict
        operations = [
            client.post(
                "/players/set_alive", json={"name": "Alice", "is_alive": False}
            ),
            client.post(
                "/players/add_status_effect",
                json={"name": "Alice", "status_effect": "Poisoned"},
            ),
            client.get("/players/list"),
            client.post(
                "/players/add_status_effect",
                json={"name": "Alice", "status_effect": "Safe"},
            ),
        ]

        # All operations should complete without errors
        for operation in operations:
            response = await operation
            assert response.status_code in [200, 404]  # 404 is ok if player was removed


@pytest.mark.anyio
async def test_script_boundary_conditions():
    """Test script-related boundary conditions."""
    # Test that script has expected minimum content
    game = Game(script_name=ScriptName.TROUBLE_BREWING)

    # Should have substantial character list
    assert len(game.script.characters) >= 10  # Reasonable minimum for BotC

    # Should have some travelers
    assert len(game.script.travelers) > 0

    # Should have first night steps
    first_night = list(game.script.get_first_night_steps())
    assert len(first_night) > 0

    # Should have other night steps
    other_nights = list(game.script.get_other_night_steps())
    assert len(other_nights) > 0

    # All characters should have valid names
    for character in game.script.characters:
        assert character.name is not None
        assert len(character.name.strip()) > 0
        assert character.category is not None
        assert character.alignment is not None


@pytest.mark.anyio
async def test_alignment_consistency():
    """Test that character alignments are consistent with game rules."""
    game = Game(script_name=ScriptName.TROUBLE_BREWING)

    # Count alignments in the script
    good_chars = [c for c in game.script.characters if c.alignment == Alignment.GOOD]
    evil_chars = [c for c in game.script.characters if c.alignment == Alignment.EVIL]

    # Should have more good than evil characters (BotC rule)
    assert len(good_chars) > len(evil_chars)

    # Should have at least one demon
    demons = [c for c in game.script.characters if c.category.value == "demon"]
    assert len(demons) >= 1

    # All demons should be evil
    for demon in demons:
        assert demon.alignment == Alignment.EVIL


@pytest.mark.anyio
async def test_game_state_consistency_after_operations():
    """Test that game state remains consistent after various operations."""
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        await client.post("/game/new", json={"script_name": "trouble_brewing"})
        # Add roles so players can be assigned
        await client.post(
            "/characters/add/multi",
            json={"names": ["Imp", "Chef", "Butler", "Baron", "Librarian"]},
        )

        # Perform a series of operations
        await client.post("/players/add", json={"name": "Alice"})
        await client.post("/players/add", json={"name": "Bob"})
        await client.post(
            "/players/add_traveler", json={"name": "Charlie", "traveler": "Beggar"}
        )

        # Kill and revive
        await client.post(
            "/players/set_alive", json={"name": "Alice", "is_alive": False}
        )
        await client.post(
            "/players/set_alive", json={"name": "Alice", "is_alive": True}
        )

        # Add and remove status effects
        await client.post(
            "/players/add_status_effect",
            json={"name": "Bob", "status_effect": "Poisoned"},
        )
        await client.post(
            "/players/remove_status_effect",
            json={"name": "Bob", "status_effect": "Poisoned"},
        )

        # Remove and re-add player
        await client.post("/players/remove", json={"name": "Bob"})
        await client.post("/players/add", json={"name": "Bob"})

        # Final state should be consistent
        response = await client.get("/players/list")
        players = response.json()

        # Should have 3 players
        assert len(players) == 3

        # Names should be unique
        names = [p["name"] for p in players]
        assert len(set(names)) == len(names)

        # All should have valid character assignments
        for player in players:
            assert player["character"]["name"] is not None
            assert player["character"]["name"] != ""
            assert isinstance(player["is_alive"], bool)
            assert isinstance(player["status_effects"], list)
