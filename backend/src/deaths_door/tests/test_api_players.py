import pytest
from httpx import ASGITransport, AsyncClient

from deaths_door.main import app

from .helpers import (
    add_test_players,
    add_test_traveler,
    enable_role_reveal,
    get_test_client,
    setup_game_with_roles,
)


@pytest.mark.anyio
async def test_create_new_game_and_add_players():
    """Test creating a new game and adding multiple players."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        # Add players
        players_data = await add_test_players(client, ["Alice", "Bob"])
        alice_data, bob_data = players_data

        # Verify player data
        assert alice_data.name == "Alice"
        assert alice_data.is_alive is True
        assert alice_data.character.name is not None
        assert bob_data.name == "Bob"
        assert (
            bob_data.character.name != alice_data.character.name
        )  # Different roles

        # Check player list
        response = await client.get("/players/list")
        assert response.status_code == 200
        players = response.json()
        assert len(players) == 2

        player_names = [p["name"] for p in players]
        assert "Alice" in player_names
        assert "Bob" in player_names


@pytest.mark.anyio
async def test_add_duplicate_player_name():
    """Test that adding a player with duplicate name fails."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)
        await add_test_players(client, ["Alice"])

        # Try to add player with same name - should be 409 Conflict
        response = await client.post("/players/add", json={"name": "Alice"})
        assert response.status_code == 409
        assert "already exists" in response.json()["detail"]


@pytest.mark.anyio
async def test_add_traveler():
    """Test adding a traveler character."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        # Add a traveler
        traveler_data = await add_test_traveler(client, "TravelerAlice", "Beggar")

        assert traveler_data.name == "TravelerAlice"
        assert traveler_data.character.name == "Beggar"
        assert traveler_data.alignment == "unknown"


@pytest.mark.anyio
async def test_add_invalid_traveler():
    """Test adding non-existent traveler fails."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        response = await client.post(
            "/players/add_traveler", json={"name": "Alice", "traveler": "FakeTraveler"}
        )
        assert response.status_code == 404  # Not found should be 404
        assert "not found" in response.json()["detail"]


@pytest.mark.anyio
async def test_player_death_and_resurrection():
    """Test killing and reviving a player."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)
        await add_test_players(client, ["Alice"])

        # Kill player
        response = await client.post(
            "/players/set_alive", json={"name": "Alice", "is_alive": False}
        )
        assert response.status_code == 200

        # Check player is dead
        response = await client.get("/players/list")
        alice = next(p for p in response.json() if p["name"] == "Alice")
        assert alice["is_alive"] is False

        # Revive player
        response = await client.post(
            "/players/set_alive", json={"name": "Alice", "is_alive": True}
        )
        assert response.status_code == 200

        # Check player is alive
        response = await client.get("/players/list")
        alice = next(p for p in response.json() if p["name"] == "Alice")
        assert alice["is_alive"] is True


@pytest.mark.anyio
async def test_player_status_effects():
    """Test adding and managing player status effects."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)
        await add_test_players(client, ["Alice"])

        # Add status effect
        response = await client.post(
            "/players/add_status_effect",
            json={"name": "Alice", "status_effect": "Poisoned"},
        )
        assert response.status_code == 200

        # Check status effect was added
        response = await client.get("/players/list")
        alice = next(p for p in response.json() if p["name"] == "Alice")
        assert "Poisoned" in alice["status_effects"]

        # Remove status effect
        response = await client.post(
            "/players/remove_status_effect",
            json={"name": "Alice", "status_effect": "Poisoned"},
        )
        assert response.status_code == 200

        # Check status effect was removed
        response = await client.get("/players/list")
        alice = next(p for p in response.json() if p["name"] == "Alice")
        assert "Poisoned" not in alice["status_effects"]


@pytest.mark.anyio
async def test_swap_player_characters():
    """Test swapping characters between two players."""
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        await client.post("/game/new", json={"script_name": "trouble_brewing"})
        # Add some roles so players can be assigned
        await client.post(
            "/characters/add/multi", json={"names": ["Imp", "Chef", "Butler"]}
        )

        # Add two players
        await client.post("/players/add", json={"name": "Alice"})
        await client.post("/players/add", json={"name": "Bob"})

        # Get their original characters
        response = await client.get("/players/list")
        players = response.json()
        alice_original = next(p for p in players if p["name"] == "Alice")["character"][
            "name"
        ]
        bob_original = next(p for p in players if p["name"] == "Bob")["character"][
            "name"
        ]

        # Swap characters
        response = await client.post(
            "/players/swap_character", json={"name1": "Alice", "name2": "Bob"}
        )
        assert response.status_code == 200

        # Check characters were swapped
        response = await client.get("/players/list")
        players = response.json()
        alice_new = next(p for p in players if p["name"] == "Alice")["character"][
            "name"
        ]
        bob_new = next(p for p in players if p["name"] == "Bob")["character"]["name"]

        assert alice_new == bob_original
        assert bob_new == alice_original


@pytest.mark.anyio
async def test_remove_player():
    """Test removing a player from the game."""
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        await client.post("/game/new", json={"script_name": "trouble_brewing"})
        # Add some roles so players can be assigned
        await client.post(
            "/characters/add/multi", json={"names": ["Imp", "Chef", "Butler"]}
        )
        await client.post("/players/add", json={"name": "Alice"})
        await client.post("/players/add", json={"name": "Bob"})

        # Remove Alice
        response = await client.post("/players/remove", json={"name": "Alice"})
        assert response.status_code == 200

        # Check Alice is gone, Bob remains
        response = await client.get("/players/list")
        players = response.json()
        assert len(players) == 1
        assert players[0]["name"] == "Bob"


@pytest.mark.anyio
async def test_role_visibility_toggle():
    """Test toggling role visibility for storyteller."""
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        await client.post("/game/new", json={"script_name": "trouble_brewing"})
        # Add some roles so players can be assigned
        await client.post(
            "/characters/add/multi", json={"names": ["Imp", "Chef", "Butler"]}
        )

        # Initially roles should not be visible
        response = await client.get("/players/visibility")
        assert response.status_code == 200
        assert response.json() is False

        # Enable role visibility
        response = await client.post(
            "/players/set_visibility", json={"should_reveal_roles": True}
        )
        assert response.status_code == 200
        assert response.json() is True

        # Check visibility is now enabled
        response = await client.get("/players/visibility")
        assert response.json() is True


@pytest.mark.anyio
async def test_get_player_role_with_reveal():
    """Test getting player role when reveal is enabled."""
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        await client.post("/game/new", json={"script_name": "trouble_brewing"})
        # Add some roles so players can be assigned
        await client.post(
            "/characters/add/multi", json={"names": ["Imp", "Chef", "Butler"]}
        )
        await client.post("/players/add", json={"name": "Alice"})

        # Enable role reveal
        await client.post("/players/set_visibility", json={"should_reveal_roles": True})

        # Should be able to get player role immediately
        response = await client.get("/players/name/Alice")
        assert response.status_code == 200

        role_data = response.json()
        assert role_data["name"] == "Alice"
        assert role_data["character"]["name"] is not None


@pytest.mark.anyio
async def test_nonexistent_player_operations():
    """Test operations on non-existent players return 404."""
    async with AsyncClient(
        transport=ASGITransport(app=app), base_url="http://test"
    ) as client:
        await client.post("/game/new", json={"script_name": "trouble_brewing"})
        # Add some roles so players can be assigned
        await client.post(
            "/characters/add/multi", json={"names": ["Imp", "Chef", "Butler"]}
        )

        # Try to operate on non-existent player
        response = await client.post(
            "/players/set_alive", json={"name": "NonExistentPlayer", "is_alive": False}
        )
        assert response.status_code == 404

        response = await client.post(
            "/players/remove", json={"name": "NonExistentPlayer"}
        )
        assert response.status_code == 404

        response = await client.get("/players/name/NonExistentPlayer")
        assert response.status_code == 404


@pytest.mark.anyio
async def test_game_workflow_integration():
    """Test a complete game workflow scenario."""
    from .helpers import COMMON_PLAYER_NAMES

    async with get_test_client() as client:
        # 1. Create game with roles
        await setup_game_with_roles(client)

        # 2. Add players for a small game (7 players)
        await add_test_players(client, COMMON_PLAYER_NAMES)

        # 3. Add a traveler
        await add_test_traveler(client, "Traveler_Henry", "Beggar")

        # 4. Check we have 8 total players
        response = await client.get("/players/list")
        assert len(response.json()) == 8

        # 5. Some players die during the night
        await client.post(
            "/players/set_alive", json={"name": "Alice", "is_alive": False}
        )
        await client.post("/players/set_alive", json={"name": "Bob", "is_alive": False})

        # 6. Add status effects
        await client.post(
            "/players/add_status_effect",
            json={"name": "Charlie", "status_effect": "Poisoned"},
        )

        # 7. Enable role reveal for storyteller
        await enable_role_reveal(client)

        # 8. Verify final game state
        response = await client.get("/players/list")
        final_players = response.json()

        # Should have 8 players total
        assert len(final_players) == 8

        # 2 should be dead
        dead_players = [p for p in final_players if not p["is_alive"]]
        assert len(dead_players) == 2

        # Charlie should be poisoned
        charlie = next(p for p in final_players if p["name"] == "Charlie")
        assert "Poisoned" in charlie["status_effects"]

        # Traveler should be present
        traveler = next(p for p in final_players if p["name"] == "Traveler_Henry")
        assert traveler["character"]["name"] == "Beggar"


@pytest.mark.anyio
async def test_concurrent_role_reveal_no_deadlock():
    """
    Test that multiple concurrent requests for player roles don't deadlock
    while waiting for role reveal to be enabled.

    This simulates the scenario where 10 players are waiting on the character
    reveal page while the storyteller enables role visibility.
    """
    import anyio

    async with get_test_client() as client:
        # Set up game with enough roles for 10 players
        roles = [
            "Imp",
            "Chef",
            "Butler",
            "Baron",
            "Librarian",
            "Empath",
            "Mayor",
            "Washerwoman",
            "Investigator",
            "Monk",
        ]
        await setup_game_with_roles(client, roles=roles)

        # Add 10 players to simulate the real scenario
        player_names = [f"Player{i}" for i in range(1, 11)]
        await add_test_players(client, player_names)

        # Track results from concurrent requests
        results = []

        async def get_player_role_after_delay(player_name: str):
            """Request player role - will wait for reveal flag"""
            response = await client.get(f"/players/name/{player_name}")
            results.append((player_name, response.status_code))

        async def enable_reveal_after_delay():
            """Enable role reveal after a short delay"""
            await anyio.sleep(0.2)  # Let requests start waiting first
            await enable_role_reveal(client)

        # Start all 10 player requests concurrently, plus the reveal enabler
        async with anyio.create_task_group() as tg:
            # All players request their roles
            for name in player_names:
                tg.start_soon(get_player_role_after_delay, name)

            # Storyteller enables reveal after a delay
            tg.start_soon(enable_reveal_after_delay)

        # All requests should succeed without deadlock
        assert len(results) == 10
        for player_name, status_code in results:
            assert status_code == 200, f"Player {player_name} failed with status {status_code}"
