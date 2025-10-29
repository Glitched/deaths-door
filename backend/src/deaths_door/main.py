from fastapi import FastAPI
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles

from .routes import characters, game, players, scripts, sounds, timer

app = FastAPI(
    title="Death's Door - Blood on the Clocktower API",
    description="""
Blood on the Clocktower game management system with streaming integration.

## Features

* **Game Management**: Create and manage BOTC games with custom scripts
* **Player Operations**: Add players, assign roles, track status
* **Night Phase Guidance**: Get filtered night steps based on alive players
* **Status Effects**: Automatic cleanup when characters die
* **OBS Integration**: Optional streaming overlay support
* **Timer Management**: Countdown timer with OBS sync

## Workflow

1. Create new game: `POST /game/new`
2. Add roles to game: `POST /characters/add/multi` (build your custom character pool)
3. Add players: `POST /players/add` (assigns random roles from the pool)
4. **Enable role visibility**: `POST /players/set_visibility` with `should_reveal_roles: true`
   - By default, roles are hidden until visibility is enabled
   - The `/players/name/{name}` endpoint waits for visibility before revealing roles
   - This allows the storyteller to control when player roles become visible
5. Guide night phases: `GET /game/script/night/first` and `/game/script/night/other`
6. Manage player states: death, status effects, alignment

## Key Concepts

**Role Visibility**: Controls when player roles are revealed to the storyteller. Use `POST /players/set_visibility`
to enable/disable. When disabled, `GET /players/name/{name}` will wait up to 10 seconds before timing out.

**Script vs Included Roles**: The script defines ALL possible characters. You must explicitly add roles via
`POST /characters/add/multi` to make them available for player assignment.
    """,
    version="1.0.0",
    contact={
        "name": "Death's Door",
        "url": "https://github.com/Glitched/deaths-door",
    },
    license_info={
        "name": "MIT",
    },
    openapi_tags=[
        {
            "name": "Game Management",
            "description": "Create and manage BOTC games, retrieve script information, and access night phase steps.",
        },
        {
            "name": "Players",
            "description": "Manage players in the game: add, remove, modify status, swap characters, and control role visibility.",
        },
        {
            "name": "Characters",
            "description": "Add or remove character roles from the current game. Roles must be added before players can be assigned.",
        },
        {
            "name": "Scripts",
            "description": "Browse available scripts/editions and view their character lists and travelers.",
        },
        {
            "name": "Timer",
            "description": "Control the countdown timer for discussion phases. Syncs with OBS when available.",
        },
        {
            "name": "Sounds",
            "description": "Play sound effects for game events. Requires local sound files.",
        },
    ],
)
app.include_router(sounds.router)
app.include_router(scripts.router)
app.include_router(game.router)
app.include_router(timer.timer)
app.include_router(players.router)
app.include_router(characters.router)

app.mount("/static/", StaticFiles(directory="static", html=True), name="static")


@app.get("/health")
def health():
    """Health check for the service to validate connection."""
    return {"status": "ok", "version": "0.0.1"}


# Vanity routes for the web client


@app.get("/favicon.ico")
def favicon():
    """Health check for the service to validate connection."""
    return FileResponse("static/favicon.ico", media_type="image/x-icon")


@app.get("/")
def get_role():
    """Health check for the service to validate connection."""
    return FileResponse("static/role.html", media_type="text/html")
