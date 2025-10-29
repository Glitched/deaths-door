# Death's Door

A game management system for [Blood on the Clocktower](https://bloodontheclocktower.com/) with streaming integration. Manages game state, player roles, and integrates with OBS Studio for professional stream overlays and production value.

## Features

- **Complete Game State Management**: Track players, characters, alignments, and status effects across multiple scripts (Trouble Brewing, etc.)
- **OBS Studio Integration**: Automated scene management, countdown timers, and stream overlays (optional)
- **RESTful API**: FastAPI backend with full type safety and async support
- **Web Control Interface**: Next.js remote for game control and scene triggering
- **Soundboard**: Trigger sound effects and music during gameplay

## Quick Start

### Backend

**Prerequisites**: Python 3.12+, [Poetry](https://python-poetry.org/)

```bash
cd backend
poetry install
poetry shell

# Run development server
make run
# or: uvicorn src.deaths_door.main:app --reload --host 0.0.0.0
```

**OBS Integration** (optional):
1. Install [OBS Studio](https://obsproject.com/downloads) and enable WebSocket server
2. Set environment variable: `export OBS_PASSWORD=your_password`
3. Install font [Help Me](https://www.dafont.com/help-me.font) for timer overlays (falls back to Arial)

> The backend runs without OBS in development mode. Set `OBS_REQUIRED=true` for production.

### Frontend

```bash
cd frontend
npm install
npm run dev
```

> Note: The Next.js frontend is being deprioritized in favor of a native iOS app (not yet published).

## Development

### Backend Commands
```bash
# Testing
poetry run pytest
poetry run pytest --cov=deaths_door --cov-report=html

# Linting & type checking
poetry run ruff check src/
poetry run ruff format src/
poetry run pyright src/
```

### Frontend Commands
```bash
npm run build
npm run lint
```

## Architecture

- **Game State**: Centralized `GameManager` with async lock-protected access for consistency
- **Character System**: Hierarchical character classes (Townsfolk/Outsiders/Minions/Demons/Travelers)
- **Scripts**: Configurable game variants defining character pools and night sequences
- **API**: RESTful endpoints with Pydantic models for type-safe serialization
- **OBS Integration**: WebSocket-based scene control with graceful degradation

### Game Setup Workflow

Games start with no roles included by default. Manual role selection required:

1. Create game: `POST /game/new` with script name
2. Add roles: `POST /characters/add/multi` with character names
3. Add players: `POST /players/add` (assigned random roles from pool)
4. Add travelers: `POST /players/add_traveler` with specific traveler name

## Documentation

See [CLAUDE.md](CLAUDE.md) for detailed architecture, API documentation, and development guidelines.