# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Backend (Python/FastAPI)
```bash
# Setup
cd backend
poetry install
poetry shell

# Run development server
make run
# or directly: uvicorn src.deaths_door.main:app --reload --host 0.0.0.0

# Testing
poetry run pytest
poetry run pytest --cov=deaths_door --cov-report=html

# Linting and type checking
poetry run ruff check src/
poetry run ruff format src/
poetry run pyright src/
```

### Frontend (Next.js)
```bash
cd frontend
npm install
npm run dev
npm run build
npm run lint
```

### Root level
```bash
# Run backend from root
make run
```

## Architecture Overview

This is a **Blood on the Clocktower game management system** with streaming integration. The backend is a FastAPI application that manages game state and integrates with OBS Studio for streaming, while the frontend provides a web-based remote control interface.

### Core Components

**Game State Management**: Centralized through `GameManager` singleton with async lock-protected access. Game state includes script, players, characters, and status effects. All game operations go through the game manager using an async context manager pattern to ensure consistency and prevent race conditions during concurrent requests.

**Character System**: Hierarchical character classes (Townsfolk/Outsiders/Minions/Demons/Travelers) with individual status effects and abilities. Characters are defined in separate modules under `characters/` and `travelers/` directories organized by script.

**Script System**: Game variants (like "Trouble Brewing") define character pools, night sequences, and game rules. Scripts are registered in `scripts/registry.py` and provide the complete game configuration.

**Player Management**: Links human players to character roles with alive/dead status, alignment tracking, and status effects. Supports role swapping, traveler addition, and vote management.

**API Architecture**: RESTful endpoints organized by domain (game, players, characters, scripts, sounds, timer) with Pydantic models for serialization.

**OBS Integration**: Optional WebSocket connection to OBS Studio for scene management, countdown timers, and streaming overlays. Gracefully degrades when OBS is unavailable (common in development). Handles dynamic text sources with automatic positioning and font fallbacks.

### Key Data Flow

1. **Game Creation**: Script selected → Game state initialized (empty roles) → **Roles manually included via API** → Ready for players
2. **Player Operations**: Player added → Random character assigned from included roles → Status effects applied → Game state updated → OBS scenes updated  
3. **Game Progression**: Night steps executed → Player states modified → Status effects calculated → UI/OBS updated

### Important Game Setup Workflow

**Games start with NO roles included by default.** This is intentional design - roles must be manually selected for each game:

1. Create new game: `POST /game/new` with script name
2. Add roles: `POST /characters/add/multi` with selected character names
3. Add players: `POST /players/add` (players get random roles from included pool)
4. Add travelers: `POST /players/add_traveler` with specific traveler name

### Environment Requirements

- Python 3.12+ with Poetry for dependency management
- **Optional:** OBS Studio with WebSocket server enabled for streaming features
  - Set `OBS_PASSWORD` environment variable for OBS connection
  - Set `OBS_REQUIRED=true` to fail startup if OBS unavailable (production mode)
  - Font "Help Me" recommended for timer overlays (falls back to Arial if unavailable)
- Application runs without OBS in development mode by default

### Testing Strategy

Backend tests in `src/deaths_door/tests/` using pytest with anyio backend. Test game state management, character assignment, and API endpoints. Use `coverage` to ensure comprehensive test coverage.

**Test Helpers Available**: Use the helper functions in `tests/helpers.py` to simplify test setup. Helpers return strongly-typed Pydantic models for full IDE support:

```python
from .helpers import get_test_client, setup_game_with_roles, add_test_players, GameTestCase

# API Tests - returns list[PlayerOut] (Pydantic models)
async with get_test_client() as client:
    await setup_game_with_roles(client)  # Sets up game with default roles
    players = await add_test_players(client, ["Alice", "Bob"])
    # Access with attributes: players[0].name, players[0].character.name
    assert players[0].name == "Alice"

# Unit Tests - returns list[Player] (domain objects)
test_case = GameTestCase()  # Creates game with roles included
players = test_case.add_players_with_roles([("Alice", "Imp"), ("Bob", "Chef")])
```

**Manual Setup** (if not using helpers):
```python
# Create game
await client.post("/game/new", json={"script_name": "trouble_brewing"})
# Add roles before adding players
await client.post("/characters/add/multi", json={"names": ["Imp", "Chef", "Butler"]})
# Now can add players
await client.post("/players/add", json={"name": "Alice"})
```

### Code Standards

- **Backend**: Uses Ruff for linting/formatting, Pyright for type checking, follows PEP 8 naming
- **Frontend**: Uses ESLint with Next.js config, TypeScript strict mode
- **Type Safety**: Extensive use of Pydantic models and Python type hints throughout. Zero type errors with strict Pyright checking.
- **Concurrency Safety**: All game state operations use async context managers with `asyncio.Lock` to prevent race conditions. Route handlers receive `AbstractAsyncContextManager[Game]` from dependencies.

### API Response Format

Player data is returned with nested character objects:
```json
{
  "name": "Alice",
  "character": {
    "name": "Imp",
    "description": "...",
    "alignment": "evil"
  },
  "is_alive": true,
  "status_effects": ["Poisoned"]
}
```

### Error Handling

- **400 Bad Request**: Validation errors (duplicate players, invalid input)
- **404 Not Found**: Resource not found (player, role, etc.)
- **408 Request Timeout**: Role reveal timeout
- Status effect removal is safe (no error if effect doesn't exist)