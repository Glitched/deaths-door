# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Version Control

**Use `jj` (Jujutsu) instead of `git` for all version control operations.** This repo is colocated, so both tools see the same commits, but prefer jj commands.

I'm learning jj - when using jj commands, please explain what the equivalent git command would be so I can build mental mappings.

## Development Commands

### Backend (Rust / axum)
```bash
cd backend-rust

# Run development server (serves on 0.0.0.0:8000)
make run
# or with sample game data:
make sample
# or directly:
cargo run
SAMPLE_GAME=true cargo run

# Testing
make test
# or directly:
cargo test

# Linting and formatting
cargo fmt
cargo fmt --check
cargo clippy --all-targets
```

Useful env vars: `PORT` (default 8000), `DATABASE_PATH` (default `games.db`), `SAMPLE_GAME=true`, `RUST_LOG` (e.g. `info`, `debug`).

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
make build       # build frontend into backend-rust/static/app/
```

## Architecture Overview

This is a **Blood on the Clocktower game management system** with streaming overlay. The backend is an axum application using **event sourcing** for game state, while the frontend is a React overlay projected on a wall during games. An iOS app serves as the primary storyteller interface.

### Core Components

**Event Sourcing**: All game state mutations are persisted as events to SQLite. A pure `apply(state, event) -> GameState` function rebuilds state from events. This enables game reload, rewind (undo), and forking (branching alternate timelines).

**Game State** (`game_state.rs`): Immutable `GameState`/`PlayerState` structs (owned, cloned on update). Derived methods include `living_player_count()`, `execution_threshold()`, and `get_dead_players_with_vote()`. Updated via `GameManager::dispatch()` which atomically applies, persists, and notifies SSE subscribers.

**Events** (`events.rs`): `EventPayload` is an internally-tagged serde enum (`#[serde(tag = "type")]`) — the compile-time-exhaustive equivalent of the original Pydantic discriminated union. Tag values match the on-disk JSON exactly.

**Event Store** (`event_store.rs`): `rusqlite`-backed persistence. Single `events` table with `game_id`, `sequence`, `event_type`, and JSON `payload`. Supports `get_events()`, `delete_after_sequence()` (rewind), and `fork_game()`.

**Character/Script System**: Character, traveler, and night-step definitions are embedded from `data/botc_data.json` (`include_str!`) and parsed into `Script`/`Character` structs. `scripts.rs` is the registry (`get_script_by_name`); only "Trouble Brewing" has data wired up.

**Real-time Updates**: `GET /game/stream` provides Server-Sent Events (SSE) via a `tokio::sync::broadcast` channel. REST endpoints remain available for polling (used by the iOS app).

**API Architecture**: RESTful endpoints in `routes/` organized by domain (game, players, characters, scripts, sounds, timer, lights). Shared state is an `AppState` injected via axum's `State` extractor. Endpoints/DTOs are annotated with `utoipa`; `GET /openapi.json` serves the full OpenAPI 3.1 spec (with a usage guide in `info.description`).

**DMX Lighting** (`lighting.rs`): Optional serial connection (`serialport`) to OpenDMX/FTDI USB interfaces for moving head lights, fog machine, and game event lighting scenes. **Sound** (`sound.rs`) uses `rodio`; **APNS** (`apns.rs`) uses `reqwest` + `jsonwebtoken` (ES256). All degrade gracefully when hardware/keys are absent.

### Key Data Flow

1. **Game Creation**: Script selected → Game state initialized → Roles manually included via API → Ready for players
2. **Mutations**: Route handler calls `manager.dispatch(event)` → `apply()` produces new immutable state → Event persisted to SQLite → SSE subscribers notified via broadcast
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

- Rust stable (1.94+); `cargo` manages dependencies
- Node.js 25+ with mise for version management
- Set `SAMPLE_GAME=true` to load sample game data on startup
- **Optional:** DMX USB interface for lighting effects; APNS key at `backend-rust/keys/AuthKey_*.p8`
- Font "Help Me" for timer overlay (falls back to Impact)

### Source-of-truth note

`data/botc_data.json` was originally dumped from the retired Python backend and is now the canonical source for character/traveler/night-step definitions. Edit it directly to change game data.

### Testing Strategy

Tests live in `backend-rust/tests/`: `event_sourcing.rs` (unit tests for `apply`/`replay`/store) and `api.rs` (HTTP integration tests through the real router via `tower::ServiceExt::oneshot`, backed by an in-memory store). `tests/common/mod.rs` has helpers:

```rust
let app = test_app();                       // in-memory-backed router
new_game(&app).await;
// Add one role then one player -> deterministic assignment (only role available).
let alice = add_player_with_role(&app, "Alice", "Imp").await;
let (status, body) = get(&app, "/game/state").await;
```

### Code Standards

- **Backend**: `cargo fmt` (rustfmt) and `cargo clippy` clean (CI runs `clippy -D warnings`); proper error enums (`StoreError`, `GameError`) via `thiserror`, mapped to HTTP status by `From<…> for AppError`.
- **Frontend**: ESLint, TypeScript strict mode, Tailwind v4 + shadcn/ui
- **Immutability**: Game state is effectively immutable — all mutations go through `dispatch()`, which produces a new state via the pure `apply()` function.

### API Response Format

Player data is returned with nested character objects:
```json
{
  "name": "Alice",
  "character": {
    "name": "Imp",
    "description": "...",
    "icon_path": "imp.png",
    "alignment": "evil",
    "category": "demon"
  },
  "is_alive": true,
  "has_used_dead_vote": false,
  "status_effects": ["Poisoned"]
}
```

Errors use a FastAPI-style `{"detail": "..."}` body.

### Error Handling

- **400 Bad Request**: Validation errors (duplicate players, invalid input, invalid rewind/fork version, unknown game id)
- **404 Not Found**: Resource not found (player, role, script, scene, etc.)
- **408 Request Timeout**: Role reveal timeout
- **409 Conflict**: Duplicate player name
- Status effect removal is safe (no error if effect doesn't exist)
