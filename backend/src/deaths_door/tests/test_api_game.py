import pytest

from .helpers import add_test_players, get_test_client, setup_game_with_roles


@pytest.mark.anyio
async def test_get_night_phase():
    """Test getting the current night phase information."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        response = await client.get("/game/night/phase")
        assert response.status_code == 200

        data = response.json()
        assert "current_night_step" in data
        assert "is_first_night" in data

        # Check defaults
        assert data["current_night_step"] == "Dusk"
        assert data["is_first_night"] is True


@pytest.mark.anyio
async def test_set_night_step():
    """Test setting the current night step."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        # Set to Poisoner
        response = await client.post(
            "/game/night/phase/step", json={"step": "Poisoner"}
        )
        assert response.status_code == 200

        data = response.json()
        assert data["current_night_step"] == "Poisoner"
        assert data["is_first_night"] is True

        # Verify it persists
        get_response = await client.get("/game/night/phase")
        assert get_response.status_code == 200
        get_data = get_response.json()
        assert get_data["current_night_step"] == "Poisoner"


@pytest.mark.anyio
async def test_set_first_night():
    """Test setting the first night flag."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        # Set to False (subsequent night)
        response = await client.post(
            "/game/night/phase/first_night", json={"is_first_night": False}
        )
        assert response.status_code == 200

        data = response.json()
        assert data["is_first_night"] is False
        assert data["current_night_step"] == "Dusk"

        # Verify it persists
        get_response = await client.get("/game/night/phase")
        assert get_response.status_code == 200
        get_data = get_response.json()
        assert get_data["is_first_night"] is False

        # Set back to True
        response2 = await client.post(
            "/game/night/phase/first_night", json={"is_first_night": True}
        )
        assert response2.status_code == 200
        assert response2.json()["is_first_night"] is True


@pytest.mark.anyio
async def test_get_night_steps_consolidated():
    """Test getting night steps based on is_first_night flag."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        # Get first night steps via consolidated endpoint
        response = await client.get("/game/script/night/steps")
        assert response.status_code == 200
        first_night_steps = response.json()

        # Get first night steps via dedicated endpoint
        response_first = await client.get("/game/script/night/first")
        assert response_first.status_code == 200
        dedicated_first = response_first.json()

        # Should be the same
        assert len(first_night_steps) == len(dedicated_first)
        assert [s["name"] for s in first_night_steps] == [
            s["name"] for s in dedicated_first
        ]

        # Switch to subsequent night
        await client.post("/game/night/phase/first_night", json={"is_first_night": False})

        # Get other night steps via consolidated endpoint
        response2 = await client.get("/game/script/night/steps")
        assert response2.status_code == 200
        other_night_steps = response2.json()

        # Get other night steps via dedicated endpoint
        response_other = await client.get("/game/script/night/other")
        assert response_other.status_code == 200
        dedicated_other = response_other.json()

        # Should be the same
        assert len(other_night_steps) == len(dedicated_other)
        assert [s["name"] for s in other_night_steps] == [
            s["name"] for s in dedicated_other
        ]

        # First and other night steps should be different
        first_names = [s["name"] for s in first_night_steps]
        other_names = [s["name"] for s in other_night_steps]
        assert first_names != other_names


@pytest.mark.anyio
async def test_night_phase_workflow():
    """Test a complete workflow of progressing through night phases."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        # Start at Dusk (first night)
        phase = await client.get("/game/night/phase")
        data = phase.json()
        assert data["current_night_step"] == "Dusk"
        assert data["is_first_night"] is True

        # Progress through first night steps
        steps = ["Minion Info", "Demon Info", "Poisoner", "Spy"]
        for step in steps:
            response = await client.post("/game/night/phase/step", json={"step": step})
            assert response.status_code == 200
            assert response.json()["current_night_step"] == step

        # End first night with Dawn
        await client.post("/game/night/phase/step", json={"step": "Dawn"})

        # Start second night - switch to other nights and reset to Dusk
        await client.post("/game/night/phase/first_night", json={"is_first_night": False})
        await client.post("/game/night/phase/step", json={"step": "Dusk"})

        # Verify state
        phase2 = await client.get("/game/night/phase")
        data2 = phase2.json()
        assert data2["current_night_step"] == "Dusk"
        assert data2["is_first_night"] is False


@pytest.mark.anyio
async def test_get_game_state():
    """Test getting the complete game state in a single request."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        # Get initial game state
        response = await client.get("/game/state")
        assert response.status_code == 200

        state = response.json()

        # Verify all fields are present
        assert "script_name" in state
        assert "players" in state
        assert "current_night_step" in state
        assert "is_first_night" in state
        assert "should_reveal_roles" in state
        assert "status_effects" in state
        assert "included_roles" in state

        # Check default values
        assert state["script_name"] == "trouble_brewing"
        assert state["players"] == []  # No players yet
        assert state["current_night_step"] == "Dusk"
        assert state["is_first_night"] is True
        assert state["should_reveal_roles"] is False
        assert isinstance(state["status_effects"], list)
        assert isinstance(state["included_roles"], list)
        assert len(state["included_roles"]) > 0  # Should have roles available


@pytest.mark.anyio
async def test_get_game_state_with_players():
    """Test game state endpoint includes player data."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        # Add players
        await add_test_players(client, ["Alice", "Bob", "Charlie"])

        # Get game state
        response = await client.get("/game/state")
        assert response.status_code == 200

        state = response.json()

        # Verify players are included
        assert len(state["players"]) == 3
        player_names = [p["name"] for p in state["players"]]
        assert "Alice" in player_names
        assert "Bob" in player_names
        assert "Charlie" in player_names

        # Verify each player has character info
        for player in state["players"]:
            assert "name" in player
            assert "character" in player
            assert "alignment" in player
            assert "is_alive" in player
            assert player["character"]["name"] is not None


@pytest.mark.anyio
async def test_get_game_state_reflects_night_phase_changes():
    """Test that game state reflects night phase modifications."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        # Modify night phase
        await client.post("/game/night/phase/step", json={"step": "Poisoner"})
        await client.post("/game/night/phase/first_night", json={"is_first_night": False})

        # Get game state
        response = await client.get("/game/state")
        assert response.status_code == 200

        state = response.json()

        # Verify night phase changes are reflected
        assert state["current_night_step"] == "Poisoner"
        assert state["is_first_night"] is False


@pytest.mark.anyio
async def test_get_game_state_includes_status_effects():
    """Test that game state includes status effects field."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        # Get game state
        response = await client.get("/game/state")
        assert response.status_code == 200

        state = response.json()

        # Verify status effects field is present (even if empty)
        assert "status_effects" in state
        assert isinstance(state["status_effects"], list)
