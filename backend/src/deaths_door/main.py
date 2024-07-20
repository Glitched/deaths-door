from fastapi import FastAPI
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles

from .game import Game
from .routes import game, scripts, sounds, timer
from .script import ScriptName

app = FastAPI()
app.include_router(sounds.router)
app.include_router(scripts.router)
app.include_router(game.router)
app.include_router(timer.timer)

app.mount("/static/", StaticFiles(directory="static", html=True), name="static")

# Initialize with a sample game to aid debugging
# Also simplifies types, since this app doesn't make sense without a game
game = Game(7, ScriptName.TROUBLE_BREWING)


@app.get("/health")
def health():
    """Health check for the service to validate connection."""
    return {"status": "ok", "version": "0.0.1"}


# Vanity routes for the web client


@app.get("/favicon.ico")
def favicon():
    """Health check for the service to validate connection."""
    return FileResponse("static/favicon.ico", media_type="image/x-icon")


@app.get("/role")
def get_role():
    """Health check for the service to validate connection."""
    return FileResponse("static/role.html", media_type="text/html")
