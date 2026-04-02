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
        response = await client.post("/game/night/phase/step", json={"step": "Poisoner"})
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
        response = await client.post("/game/night/phase/first_night", json={"is_first_night": False})
        assert response.status_code == 200

        data = response.json()
        assert data["is_first_night"] is False
        assert data["current_night_step"] == "Dusk"

        # Verify it persists
        get_response = await client.get("/game/night/phase")
        assert get_response.status_code == 200
        get_data = get_response.json()
        assert get_data["is_first_night"] is False

        # Set back to True (should also reset to Dusk)
        response2 = await client.post("/game/night/phase/first_night", json={"is_first_night": True})
        assert response2.status_code == 200
        assert response2.json()["is_first_night"] is True
        assert response2.json()["current_night_step"] == "Dusk"


@pytest.mark.anyio
async def test_set_first_night_resets_to_dusk():
    """Test that changing is_first_night always resets current_night_step to Dusk."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        # Start at Dusk on first night
        phase = await client.get("/game/night/phase")
        assert phase.json()["current_night_step"] == "Dusk"
        assert phase.json()["is_first_night"] is True

        # Move to a different step
        await client.post("/game/night/phase/step", json={"step": "Poisoner"})
        phase = await client.get("/game/night/phase")
        assert phase.json()["current_night_step"] == "Poisoner"

        # Change to subsequent night - should reset to Dusk
        response = await client.post("/game/night/phase/first_night", json={"is_first_night": False})
        assert response.json()["current_night_step"] == "Dusk"
        assert response.json()["is_first_night"] is False

        # Move to another step on subsequent night
        await client.post("/game/night/phase/step", json={"step": "Imp"})
        phase = await client.get("/game/night/phase")
        assert phase.json()["current_night_step"] == "Imp"

        # Change back to first night - should reset to Dusk again
        response = await client.post("/game/night/phase/first_night", json={"is_first_night": True})
        assert response.json()["current_night_step"] == "Dusk"
        assert response.json()["is_first_night"] is True


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
        assert [s["name"] for s in first_night_steps] == [s["name"] for s in dedicated_first]

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
        assert [s["name"] for s in other_night_steps] == [s["name"] for s in dedicated_other]

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

        # Start second night - switch to other nights (automatically resets to Dusk)
        response = await client.post("/game/night/phase/first_night", json={"is_first_night": False})

        # Verify state was reset to Dusk
        assert response.json()["current_night_step"] == "Dusk"
        assert response.json()["is_first_night"] is False


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
        assert "night_steps" in state
        assert "timer" in state

        # Check default values
        assert state["script_name"] == "trouble_brewing"
        assert state["players"] == []  # No players yet
        assert state["current_night_step"] == "Dusk"
        assert state["is_first_night"] is True
        assert state["should_reveal_roles"] is False
        assert isinstance(state["status_effects"], list)
        assert isinstance(state["included_roles"], list)
        assert len(state["included_roles"]) > 0  # Should have roles available
        assert isinstance(state["night_steps"], list)
        assert len(state["night_steps"]) > 0  # Should have first night steps

        # Verify timer state structure
        assert "is_running" in state["timer"]
        assert "seconds" in state["timer"]
        assert isinstance(state["timer"]["is_running"], bool)
        assert isinstance(state["timer"]["seconds"], int)
        assert state["timer"]["seconds"] >= 0

        # Verify night steps contain expected structure
        first_step = state["night_steps"][0]
        assert "name" in first_step
        assert "description" in first_step
        assert first_step["name"] == "Dusk"  # First step should be Dusk


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

        # Set step to Poisoner and verify it's reflected in state
        await client.post("/game/night/phase/step", json={"step": "Poisoner"})
        response = await client.get("/game/state")
        assert response.status_code == 200
        first_night_state = response.json()
        assert first_night_state["current_night_step"] == "Poisoner"
        assert first_night_state["is_first_night"] is True

        # Store first night steps for comparison
        first_night_steps = first_night_state["night_steps"]
        first_night_step_names = [s["name"] for s in first_night_steps]

        # Change to subsequent night (resets to Dusk)
        await client.post("/game/night/phase/first_night", json={"is_first_night": False})

        # Get game state and verify changes
        response2 = await client.get("/game/state")
        assert response2.status_code == 200

        state = response2.json()

        # Verify night phase changes are reflected (should be Dusk after changing night type)
        assert state["current_night_step"] == "Dusk"
        assert state["is_first_night"] is False

        # Verify night_steps changed to subsequent night steps
        other_night_steps = state["night_steps"]
        other_night_step_names = [s["name"] for s in other_night_steps]

        # Both should have Dusk and Dawn (always shown)
        assert "Dusk" in first_night_step_names
        assert "Dusk" in other_night_step_names
        assert "Dawn" in first_night_step_names
        assert "Dawn" in other_night_step_names

        # First night has Minion Info and Demon Info (always shown on first night)
        assert "Minion Info" in first_night_step_names
        assert "Demon Info" in first_night_step_names

        # Subsequent nights don't have Minion Info or Demon Info
        assert "Minion Info" not in other_night_step_names
        assert "Demon Info" not in other_night_step_names


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


@pytest.mark.anyio
async def test_get_game_state_includes_vote_info():
    """Test that game state includes vote tracking fields."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)
        await add_test_players(client, ["Alice", "Bob", "Charlie", "Dave", "Eve"])

        response = await client.get("/game/state")
        assert response.status_code == 200
        state = response.json()

        # 5 living players, threshold is 3
        assert state["living_player_count"] == 5
        assert state["execution_threshold"] == 3
        assert state["dead_players_with_vote"] == []

        # Kill Alice
        await client.post("/players/set_alive", json={"name": "Alice", "is_alive": False})

        response = await client.get("/game/state")
        state = response.json()

        # 4 living, threshold is 2, Alice has her dead vote
        assert state["living_player_count"] == 4
        assert state["execution_threshold"] == 2
        assert state["dead_players_with_vote"] == ["Alice"]

        # Alice uses her dead vote
        await client.post(
            "/players/set_has_used_dead_vote",
            json={"name": "Alice", "has_used_dead_vote": True},
        )

        response = await client.get("/game/state")
        state = response.json()
        assert state["dead_players_with_vote"] == []


@pytest.mark.anyio
async def test_game_history():
    """Test getting event history for the current game."""
    async with get_test_client() as client:
        await setup_game_with_roles(client, roles=["Imp", "Chef"])
        await add_test_players(client, ["Alice", "Bob"])

        response = await client.get("/game/history")
        assert response.status_code == 200

        history = response.json()
        assert history["version"] > 0
        assert len(history["events"]) == history["version"]
        assert history["events"][0]["event_type"] == "game_created"


@pytest.mark.anyio
async def test_rewind():
    """Test rewinding a game to a previous version."""
    async with get_test_client() as client:
        await setup_game_with_roles(client, roles=["Imp", "Chef", "Monk"])
        await add_test_players(client, ["Alice", "Bob"])

        # Kill Alice
        await client.post("/players/set_alive", json={"name": "Alice", "is_alive": False})

        # Verify Alice is dead
        state = (await client.get("/game/state")).json()
        alice = next(p for p in state["players"] if p["name"] == "Alice")
        assert alice["is_alive"] is False

        # Get version before rewind
        history = (await client.get("/game/history")).json()
        rewind_to = history["version"] - 1  # Before the kill

        # Rewind
        response = await client.post("/game/rewind", json={"to_version": rewind_to})
        assert response.status_code == 200

        # Alice should be alive again
        state = response.json()
        alice = next(p for p in state["players"] if p["name"] == "Alice")
        assert alice["is_alive"] is True


@pytest.mark.anyio
async def test_rewind_invalid_version():
    """Test rewinding to an invalid version returns 400."""
    async with get_test_client() as client:
        await setup_game_with_roles(client)

        response = await client.post("/game/rewind", json={"to_version": 999})
        assert response.status_code == 400


@pytest.mark.anyio
async def test_fork():
    """Test forking a game creates a new independent branch."""
    async with get_test_client() as client:
        await setup_game_with_roles(client, roles=["Imp", "Chef", "Monk"])
        await add_test_players(client, ["Alice", "Bob"])

        # Get version to fork from (before adding players would lose them)
        history = (await client.get("/game/history")).json()
        fork_version = history["version"]

        # Fork
        response = await client.post("/game/fork", json={"from_version": fork_version})
        assert response.status_code == 200

        fork_data = response.json()
        assert "new_game_id" in fork_data
        # Forked game should have the same version as fork point
        assert fork_data["version"] == fork_version


@pytest.mark.anyio
async def test_load_game():
    """Test loading a game by ID."""
    async with get_test_client() as client:
        # Create first game
        await setup_game_with_roles(client, roles=["Imp", "Chef"])
        await add_test_players(client, ["Alice"])

        # Remember this game's ID
        history1 = (await client.get("/game/history")).json()
        game1_id = history1["game_id"]

        # Create a second game (replaces current)
        await setup_game_with_roles(client, roles=["Imp", "Chef", "Monk"])

        # Load the first game back
        response = await client.post("/game/load", json={"game_id": game1_id})
        assert response.status_code == 200

        # Should have Alice
        loaded_state = response.json()
        player_names = [p["name"] for p in loaded_state["players"]]
        assert "Alice" in player_names


@pytest.mark.anyio
async def test_list_games():
    """Test listing all saved games."""
    async with get_test_client() as client:
        # Create two games
        await setup_game_with_roles(client)
        await setup_game_with_roles(client)

        response = await client.get("/game/list")
        assert response.status_code == 200

        data = response.json()
        assert len(data["game_ids"]) >= 2
