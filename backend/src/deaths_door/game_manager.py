"""Game manager with event sourcing dispatch pattern."""

from __future__ import annotations

import asyncio
from datetime import datetime, timezone
from uuid import UUID, uuid4

from .apply import apply, replay
from .event_store import EventStore
from .events import EventPayload, GameCreated, GameEvent
from .game_state import GameState


class GameManager:
    """
    Manages game state via event sourcing.

    All mutations go through dispatch(), which atomically:
    1. Creates an event from the payload
    2. Applies it to produce a new state (validates)
    3. Persists the event to SQLite
    4. Updates the in-memory cache
    """

    def __init__(self, event_store: EventStore) -> None:
        """Create a new game manager backed by an event store."""
        self._store = event_store
        self._state: GameState | None = None
        self._lock = asyncio.Lock()

    @property
    def state(self) -> GameState:
        """Get the current game state. Raises if no game is active."""
        if self._state is None:
            raise RuntimeError("No active game")
        return self._state

    async def dispatch(self, payload: EventPayload) -> GameState:
        """Atomically apply and persist an event. Returns the new state."""
        async with self._lock:
            current = self.state
            event = GameEvent(
                game_id=current.game_id,
                sequence=current.version,
                timestamp=datetime.now(timezone.utc),
                payload=payload,
            )
            # Apply first to validate (raises on invalid state transitions)
            new_state = apply(current, event)
            # Persist only after successful apply
            self._store.append(event)
            self._state = new_state
            return new_state

    async def get_state(self) -> GameState:
        """Get the current game state (no lock needed — single reference read)."""
        return self.state

    async def create_game(self, script_name: str) -> GameState:
        """Create a new game with a fresh game_id."""
        async with self._lock:
            game_id = uuid4()
            payload = GameCreated(script_name=script_name)
            event = GameEvent(
                game_id=game_id,
                sequence=0,
                timestamp=datetime.now(timezone.utc),
                payload=payload,
            )
            initial = GameState(game_id=game_id, script_name="")
            new_state = apply(initial, event)
            self._store.append(event)
            self._state = new_state
            return new_state

    async def load_game(self, game_id: UUID) -> GameState:
        """Load a game from the event store by replaying its events."""
        async with self._lock:
            events = self._store.get_events(game_id)
            if not events:
                raise ValueError(f"No events found for game {game_id}")
            self._state = replay(events)
            return self._state

    async def rewind(self, to_version: int) -> GameState:
        """
        Rewind the current game to a specific version.

        Deletes events after the target version and rebuilds state.
        """
        async with self._lock:
            current = self.state
            if to_version < 1 or to_version > current.version:
                raise ValueError(f"Invalid version {to_version} (current: {current.version})")

            # Delete events after the target and rebuild
            self._store.delete_after_sequence(current.game_id, to_version - 1)
            events = self._store.get_events(current.game_id)
            self._state = replay(events)
            return self._state

    async def fork(self, from_version: int) -> GameState:
        """
        Fork the current game from a specific version.

        Creates a new game_id with events copied up to from_version,
        then loads and returns the forked game.
        """
        async with self._lock:
            current = self.state
            if from_version < 1 or from_version > current.version:
                raise ValueError(f"Invalid version {from_version} (current: {current.version})")

            new_game_id = self._store.fork_game(current.game_id, up_to_sequence=from_version - 1)
            events = self._store.get_events(new_game_id)
            self._state = replay(events)
            return self._state

    async def get_history(self) -> list[GameEvent]:
        """Get all events for the current game."""
        current = self.state
        return self._store.get_events(current.game_id)

    async def list_games(self) -> list[UUID]:
        """List all game IDs in the store."""
        return self._store.get_all_game_ids()

    def has_active_game(self) -> bool:
        """Check if there is an active game loaded."""
        return self._state is not None


# Module-level singleton
event_store = EventStore()
game_manager = GameManager(event_store)
