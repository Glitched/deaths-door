import asyncio
from contextlib import AbstractAsyncContextManager, asynccontextmanager

from .game import Game


class GameManager:
    """Manager for the game."""

    def __init__(self):
        """Create a new game manager."""
        self.game = Game.get_sample_game()
        self.lock = asyncio.Lock()

    @asynccontextmanager
    async def locked_game(self):
        """
        Get the game with lock held for the duration of the context.

        This ensures that game state mutations are thread-safe by holding
        the lock for the entire operation, not just while retrieving the reference.
        """
        async with self.lock:
            yield self.game

    async def get_game(self) -> Game:
        """Get the current game (for read-only access)."""
        async with self.lock:
            return self.game

    async def replace_game(self, new_game: Game):
        """Replace the current game."""
        async with self.lock:
            self.game = new_game


game_manager = GameManager()


async def get_current_game() -> AbstractAsyncContextManager[Game]:
    """
    FastAPI dependency that provides locked access to the game.

    Returns a context manager that must be used with 'async with'.
    This ensures the lock is held for the entire route handler operation.
    """
    return game_manager.locked_game()


async def replace_game(new_game: Game):
    """Replace the current game."""
    await game_manager.replace_game(new_game)
