from threading import Lock

from .game import Game


class GameManager:
    """Manager for the game."""

    def __init__(self):
        """Create a new game manager."""
        self.game = Game.get_sample_game()
        self.lock = Lock()

    def get_game(self) -> Game:
        """Get the current game."""
        with self.lock:
            return self.game

    def replace_game(self, new_game: Game):
        """Replace the current game."""
        with self.lock:
            self.game = new_game


game_manager = GameManager()


def get_current_game():
    """Get the current game."""
    return game_manager.get_game()


def replace_game(new_game: Game):
    """Replace the current game."""
    game_manager.replace_game(new_game)
