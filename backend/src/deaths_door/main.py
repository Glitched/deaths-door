from fastapi import FastAPI

from .game import Game
from .routes import game, scripts, sounds
from .script import ScriptName

app = FastAPI()
app.include_router(sounds.router)
app.include_router(scripts.router)
app.include_router(game.router)

# Initialize with a sample game to aid debugging
# Also simplifies types, since this app doesn't make sense without a game
game = Game(7, ScriptName.TROUBLE_BREWING)
