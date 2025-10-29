"""Test helper utilities for Deaths Door tests."""

from httpx import ASGITransport, AsyncClient

from deaths_door.game import Game
from deaths_door.main import app
from deaths_door.player import Player, PlayerOut
from deaths_door.script import ScriptName


def create_test_game_with_roles(
    script_name: ScriptName = ScriptName.TROUBLE_BREWING, roles: list[str] | None = None
) -> Game:
    """
    Create a test game with specified roles included.

    Args:
        script_name: The script to use for the game
        roles: List of role names to include. If None, uses a default set.

    Returns:
        Game instance ready for adding players

    """
    if roles is None:
        roles = ["Imp", "Chef", "Butler", "Baron", "Librarian", "Empath", "Mayor"]

    game = Game(script_name=script_name)
    game.included_roles = game.script.characters.copy()

    # Only include the specified roles
    game.included_roles = [
        char for char in game.script.characters if char.name in roles
    ]

    return game


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


async def add_test_traveler(
    client: AsyncClient, player_name: str, traveler_name: str
) -> PlayerOut:
    """
    Add a traveler to the game and return their data.

    Args:
        client: HTTP client for making API calls
        player_name: Name for the player
        traveler_name: Name of the traveler character

    Returns:
        Player data parsed as PlayerOut model

    """
    response = await client.post(
        "/players/add_traveler", json={"name": player_name, "traveler": traveler_name}
    )
    assert response.status_code == 200
    return PlayerOut(**response.json())


async def enable_role_reveal(client: AsyncClient) -> None:
    """Enable role revelation for the current game."""
    await client.post("/players/set_visibility", json={"should_reveal_roles": True})


class GameTestCase:
    """Base class for game test scenarios with common setup."""

    def __init__(
        self,
        script_name: ScriptName = ScriptName.TROUBLE_BREWING,
        roles: list[str] | None = None,
    ):
        self.script_name = script_name
        self.game = create_test_game_with_roles(script_name, roles)

    def add_players_with_roles(self, player_role_pairs: list[tuple[str, str]]) -> list[Player]:
        """
        Add players with specific roles to the test game.

        Args:
            player_role_pairs: List of (player_name, role_name) tuples

        Returns:
            List of Player objects that were added

        """
        players: list[Player] = []
        for player_name, role_name in player_role_pairs:
            player = self.game.add_player_with_role(player_name, role_name)
            players.append(player)
        return players

    def add_random_players(self, player_names: list[str]) -> list[Player]:
        """
        Add players with random roles to the test game.

        Args:
            player_names: List of player names to add

        Returns:
            List of Player objects that were added

        """
        players: list[Player] = []
        for name in player_names:
            player = self.game.add_player_with_random_role(name)
            players.append(player)
        return players


def create_empty_game(script_name: ScriptName = ScriptName.TROUBLE_BREWING) -> Game:
    """Create a game with no roles included (for testing edge cases)."""
    return Game(script_name=script_name)


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
