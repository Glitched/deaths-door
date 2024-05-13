from fastapi import FastAPI

from .game import Game
from .routes import game, scripts, sounds, timer
from .script import ScriptName

app = FastAPI()
app.include_router(sounds.router)
app.include_router(scripts.router)
app.include_router(game.router)
app.include_router(timer.timer)

# Initialize with a sample game to aid debugging
# Also simplifies types, since this app doesn't make sense without a game
game = Game(7, ScriptName.TROUBLE_BREWING)


@app.get("/health")
def health():
    """Health check for the service to validate connection."""
    return {"status": "ok", "version": "0.0.1"}
