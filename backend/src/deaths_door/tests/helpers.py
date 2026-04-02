"""Test helper utilities for Deaths Door tests."""

from datetime import datetime, timezone
from uuid import uuid4

from httpx import ASGITransport, AsyncClient

from deaths_door.apply import apply
from deaths_door.events import GameCreated, GameEvent, PlayerAdded, RolesIncluded
from deaths_door.game_state import GameState, PlayerState
from deaths_door.main import app
from deaths_door.player import PlayerOut
from deaths_door.script_name import ScriptName
from deaths_door.scripts.registry import get_script_by_name


def _make_event(state: GameState, payload: object) -> GameEvent:
    """Create a GameEvent from a state and payload."""
    return GameEvent(
        game_id=state.game_id,
        sequence=state.version,
        timestamp=datetime.now(timezone.utc),
        payload=payload,  # type: ignore[reportArgumentType]
    )


def create_test_game_state(
    script_name: ScriptName = ScriptName.TROUBLE_BREWING,
    roles: list[str] | None = None,
) -> GameState:
    """
    Create a test GameState with specified roles included.

    Args:
        script_name: The script to use for the game
        roles: List of role names to include. If None, uses a default set.

    Returns:
        Frozen GameState with roles included, ready for adding players.

    """
    if roles is None:
        roles = ["Imp", "Chef", "Butler", "Baron", "Librarian", "Empath", "Mayor"]

    game_id = uuid4()
    state = GameState(game_id=game_id, script_name="")

    # Apply game creation
    state = apply(state, _make_event(state, GameCreated(script_name=script_name.value)))

    # Add roles
    state = apply(state, _make_event(state, RolesIncluded(names=tuple(roles))))

    return state


def get_test_client() -> AsyncClient:
    """Get an AsyncClient configured for testing the FastAPI app."""
    return AsyncClient(transport=ASGITransport(app=app), base_url="http://test")


async def setup_game_with_roles(
    client: AsyncClient,
    script_name: str = "trouble_brewing",
    roles: list[str] | None = None,
) -> None:
    """
    Set up a new game via API with specified roles.

    Args:
        client: HTTP client for making API calls
        script_name: Script name for the game
        roles: List of role names to include. If None, uses a default set.

    """
    if roles is None:
        roles = ["Imp", "Chef", "Butler", "Baron", "Librarian", "Empath", "Mayor"]

    # Create game
    await client.post("/game/new", json={"script_name": script_name})

    # Add roles
    await client.post("/characters/add/multi", json={"names": roles})


async def add_test_players(client: AsyncClient, player_names: list[str]) -> list[PlayerOut]:
    """
    Add multiple players to the game and return their data.

    Args:
        client: HTTP client for making API calls
        player_names: List of player names to add

    Returns:
        List of player data parsed as PlayerOut models

    """
    players: list[PlayerOut] = []
    for name in player_names:
        response = await client.post("/players/add", json={"name": name})
        assert response.status_code == 200
        players.append(PlayerOut(**response.json()))
    return players


async def add_test_traveler(client: AsyncClient, player_name: str, traveler_name: str) -> PlayerOut:
    """
    Add a traveler to the game and return their data.

    Args:
        client: HTTP client for making API calls
        player_name: Name for the player
        traveler_name: Name of the traveler character

    Returns:
        Player data parsed as PlayerOut model

    """
    response = await client.post("/players/add_traveler", json={"name": player_name, "traveler": traveler_name})
    assert response.status_code == 200
    return PlayerOut(**response.json())


async def enable_role_reveal(client: AsyncClient) -> None:
    """Enable role revelation for the current game."""
    await client.post("/players/set_visibility", json={"should_reveal_roles": True})


class GameTestCase:
    """Base class for game test scenarios using immutable GameState."""

    def __init__(
        self,
        script_name: ScriptName = ScriptName.TROUBLE_BREWING,
        roles: list[str] | None = None,
    ):
        """Create a test game with the given script and roles."""
        self.script_name = script_name
        self.state = create_test_game_state(script_name, roles)

    def add_players_with_roles(self, player_role_pairs: list[tuple[str, str]]) -> list[PlayerState]:
        """
        Add players with specific roles to the test game.

        Args:
            player_role_pairs: List of (player_name, role_name) tuples

        Returns:
            List of PlayerState objects from the final state

        """
        script = get_script_by_name(self.state.script_name)
        if script is None:
            raise ValueError(f"Script not found: {self.state.script_name}")

        players: list[PlayerState] = []
        for player_name, role_name in player_role_pairs:
            character = script.get_character(role_name)
            if character is None:
                raise ValueError(f"Character not found: {role_name}")
            event = _make_event(
                self.state,
                PlayerAdded(
                    player_name=player_name,
                    character_name=character.name,
                    alignment=character.alignment.value,
                ),
            )
            self.state = apply(self.state, event)
            player = self.state.get_player(player_name)
            if player is None:
                raise ValueError(f"Player not found after adding: {player_name}")
            players.append(player)
        return players


def create_empty_game_state(script_name: ScriptName = ScriptName.TROUBLE_BREWING) -> GameState:
    """Create a game state with no roles included (for testing edge cases)."""
    state = GameState(game_id=uuid4(), script_name="")
    return apply(state, _make_event(state, GameCreated(script_name=script_name.value)))


# Common test data sets
SMALL_GAME_ROLES = ["Imp", "Chef", "Butler"]
MEDIUM_GAME_ROLES = ["Imp", "Chef", "Butler", "Baron", "Librarian", "Empath", "Mayor"]
LARGE_GAME_ROLES = [
    "Imp",
    "Chef",
    "Butler",
    "Baron",
    "Librarian",
    "Empath",
    "Mayor",
    "Fortune Teller",
    "Slayer",
    "Scarlet Woman",
    "Monk",
    "Recluse",
]

COMMON_PLAYER_NAMES = ["Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace"]
