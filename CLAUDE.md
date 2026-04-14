# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Version Control

**Use `jj` (Jujutsu) instead of `git` for all version control operations.** This repo is colocated, so both tools see the same commits, but prefer jj commands.

I'm learning jj - when using jj commands, please explain what the equivalent git command would be so I can build mental mappings.

## Development Commands

### Backend (Python/FastAPI)
```bash
# Setup (uv manages Python and dependencies)
cd backend
uv sync

# Run development server
make run
# or with sample game data:
make sample

# Testing
make test
# or directly: uv run pytest
uv run pytest --cov=deaths_door --cov-report=html

# Linting and type checking
uv run ruff check src/
uv run ruff format src/
uv run pyright src/
```

### Frontend (Vite + React)
```bash
cd frontend
npm install
npm run dev      # dev server at :5173 with proxy to backend
npm run build
npm run lint
```

### Root level
```bash
make run         # run backend
make test        # run backend tests
make sample      # run backend with sample game
make build       # build frontend into backend/static/app/
```

## Architecture Overview

This is a **Blood on the Clocktower game management system** with streaming overlay. The backend is a FastAPI application using **event sourcing** for game state, while the frontend is a React overlay projected on a wall during games. An iOS app serves as the primary storyteller interface.

### Core Components

**Event Sourcing**: All game state mutations are persisted as events to SQLite. A pure `apply(state, event) -> state` function rebuilds state from events. This enables game reload, rewind (undo), and forking (branching alternate timelines).

**Game State** (`game_state.py`): Frozen Pydantic models (`GameState`, `PlayerState`) — fully immutable. Derived properties include `living_player_count`, `execution_threshold`, and `dead_players_with_vote`. Updated via `GameManager.dispatch()` which atomically applies, persists, and notifies SSE subscribers.

**Event Store** (`event_store.py`): SQLite-backed persistence. Single `events` table with `game_id`, `sequence`, `event_type`, and JSON `payload`. Supports `get_events()`, `delete_after_sequence()` (rewind), and `fork_game()`.

**Character System**: Hierarchical character classes (Townsfolk/Outsiders/Minions/Demons/Travelers) defined in `characters/` and `travelers/` directories organized by script.

**Script System**: Game variants (like "Trouble Brewing") define character pools, night sequences, and game rules. Scripts are registered in `scripts/registry.py`.

**Real-time Updates**: `GET /game/stream` provides Server-Sent Events (SSE) for instant state updates. REST endpoints remain available for polling (used by the iOS app).

**API Architecture**: RESTful endpoints organized by domain (game, players, characters, scripts, sounds, timer) with Pydantic models for serialization.

**DMX Lighting**: Optional serial connection to OpenDMX/FTDI USB interfaces for moving head lights, fog machine, and game event lighting scenes.

### Key Data Flow

1. **Game Creation**: Script selected → Game state initialized → Roles manually included via API → Ready for players
2. **Mutations**: Route handler calls `game_manager.dispatch(event)` → `apply()` produces new immutable state → Event persisted to SQLite → SSE subscribers notified
3. **Frontend**: React overlay connects via `EventSource` to `/game/stream` → Receives full state on connect and after every mutation → Renders timer, player list, vote info

### Important Game Setup Workflow

**Games start with NO roles included by default.** Roles must be manually selected for each game:

1. Create new game: `POST /game/new` with script name
2. Add roles: `POST /characters/add/multi` with selected character names
3. Add players: `POST /players/add` (players get random roles from included pool)
4. Add travelers: `POST /players/add_traveler` with specific traveler name

### Event Sourcing Endpoints

- `GET /game/history` — Full event log with human-readable descriptions
- `POST /game/rewind` — Rewind to a previous version (destructive — use fork first to preserve)
- `POST /game/fork` — Branch from a version into a new independent game
- `POST /game/load` — Load a saved game by ID
- `GET /game/list` — List all saved game IDs
- `GET /game/stream` — SSE stream of game state changes

### Environment

- Python 3.14+ with uv for dependency management
- Node.js 25+ with mise for version management
- Set `SAMPLE_GAME=true` to load sample game data on startup
- **Optional:** DMX USB interface for lighting effects
- Font "Help Me" for timer overlay (falls back to Impact)

### Testing Strategy

Backend tests in `src/deaths_door/tests/` using pytest with anyio backend. 210+ tests covering event sourcing, API endpoints, game state, and persistence.

**Test Helpers Available** in `tests/helpers.py`:

```python
from .helpers import get_test_client, setup_game_with_roles, add_test_players, GameTestCase

# API Tests - returns list[PlayerOut] (Pydantic models)
async with get_test_client() as client:
    await setup_game_with_roles(client)
    players = await add_test_players(client, ["Alice", "Bob"])
    assert players[0].name == "Alice"

# Unit Tests - uses immutable GameState + apply()
test_case = GameTestCase(roles=["Imp", "Chef", "Monk"])
test_case.add_players_with_roles([("Alice", "Imp"), ("Bob", "Chef")])
assert test_case.state.living_player_count == 2
```

### Code Standards

- **Backend**: Uses Ruff for linting/formatting (line-length 120), Pyright for type checking (strict mode), follows PEP 8 naming
- **Frontend**: Uses ESLint, TypeScript strict mode, Tailwind v4 + shadcn/ui
- **Type Safety**: Frozen Pydantic models throughout. Zero type errors with strict Pyright checking.
- **Immutability**: Game state is fully immutable. All mutations go through `dispatch()` which produces a new state via the pure `apply()` function.

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
