# Death's Door

A tool to increase the production value when we host games of [Blood on the Clocktower](https://bloodontheclocktower.com/).

The backend ([Rust](https://www.rust-lang.org/) + [axum](https://github.com/tokio-rs/axum)) manages game state with event sourcing, provides a soundboard, controls DMX lighting, and pushes live updates via SSE and APNS.

The frontend ([React](https://react.dev/) + [Vite](https://vite.dev/)) is a streaming overlay projected on a wall — countdown timer, player list, and vote threshold — designed to replace OBS scenes.

An iOS app (maintained separately) serves as the storyteller's primary remote control interface.

## Quick Start

```bash
# Backend
cd backend
make run         # or: make sample (loads a pre-built test game)

# Frontend (dev mode with hot reload)
cd frontend
npm install
npm run dev      # http://localhost:5173/overlay

# Build frontend into the backend for single-server mode
make build       # then: make run serves everything on :8000
```

## Running a Game

1. Start the backend: `make run` (or `make sample` for a test game)
2. Start the frontend: `cd frontend && npm run dev`
3. Project `http://localhost:5173/overlay` on the wall
4. Control the game from the iOS app
5. Players visit `http://localhost:8000/reveal` on their phones to see their roles

## Requirements

- Rust (stable, 1.94+) — install via [rustup](https://rustup.rs/)
- Node.js 25+ (managed by [mise](https://mise.jdx.dev/))
- Font [Help Me](https://www.dafont.com/help-me.font) (optional — falls back to Impact)
- **Optional:** DMX USB interface for lighting effects
