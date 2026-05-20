# Death's Door — Rust backend

A Rust port of the Python/FastAPI `backend/`, built with **axum** + **tokio**. It
is a drop-in replacement at the HTTP layer: the JSON shapes for every endpoint
match the Python implementation byte-for-byte, so the existing React overlay and
iOS app work against it unchanged.

## Run

```bash
cargo run                 # serves on http://0.0.0.0:8000
SAMPLE_GAME=true cargo run # boot with the sample game loaded
PORT=8080 cargo run        # override the port
DATABASE_PATH=games.db cargo run  # override the SQLite path (default: games.db)
```

## Test / lint

```bash
cargo test          # 33 tests: event-sourcing core, store, and HTTP API
cargo clippy        # clean
cargo build --release
```

## Architecture

Same event-sourcing model as the Python version:

- **`events.rs`** — `EventPayload` is an internally-tagged serde enum (the
  compile-time-exhaustive equivalent of the Python Pydantic discriminated union).
  The `type` tag values match the Python `EventType` strings exactly.
- **`apply.rs`** — pure `apply(state, event) -> GameState` and `replay()`.
- **`game_state.rs`** — immutable `GameState`/`PlayerState` with derived
  properties (`living_player_count`, `execution_threshold`, night-step filtering…).
- **`event_store.rs`** — `rusqlite`-backed store. Schema and JSON payloads match
  Python, so a `games.db` written by either backend is readable by the other.
- **`game_manager.rs`** — `tokio::Mutex` guards the dispatch (apply → persist →
  notify); a `tokio::sync::broadcast` channel fans state out to SSE subscribers.
- **`routes/`** — axum handlers mirroring the Python `routes/` modules, including
  the `/game/stream` SSE endpoint. Errors render as FastAPI-style
  `{"detail": "..."}` with matching status codes.
- **OpenAPI** — `GET /openapi.json` serves a complete OpenAPI 3.1 spec
  (generated from `utoipa` annotations on the handlers/DTOs), including a usage
  guide in `info.description`. Hand this to a client generator or another agent.
- **Hardware** — `lighting.rs` (DMX over `serialport`), `sound.rs` (`rodio`),
  `apns.rs` (`reqwest` + `jsonwebtoken` ES256), `timer_state.rs` (background tick
  task). All degrade gracefully when hardware/keys are absent.

## Character data

Character, traveler, and night-step definitions live in `data/botc_data.json`,
embedded into the binary via `include_str!`. This file (originally dumped from the
retired Python backend) is now the canonical source — edit it directly to change
game data.

## Notes / intentional differences

- Only **Trouble Brewing** has character data (the other script names appear in
  `/scripts/list` but resolve to no roles).
- The original `LightingSequence` cue-timeline system is **not** carried over — no
  HTTP route reaches it (it was dead code).
- Game manager, timer, and lighting live in an `AppState` injected into handlers
  via axum's `State` extractor (rather than module-level singletons).
