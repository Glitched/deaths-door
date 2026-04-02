"""SQLite-backed event store for game event persistence."""

from __future__ import annotations

import json
import sqlite3
from datetime import datetime
from pathlib import Path
from typing import cast
from uuid import UUID, uuid4

from .events import EventPayload, EventType, GameEvent

CREATE_TABLE = """
CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    UNIQUE(game_id, sequence)
);
CREATE INDEX IF NOT EXISTS idx_events_game_id ON events(game_id);
"""

# Map EventType values to their payload classes for deserialization
_EVENT_TYPE_MAP: dict[str, type] = {}


def _build_event_type_map() -> dict[str, type]:
    """Build the event type -> payload class mapping from the EventPayload union."""
    if _EVENT_TYPE_MAP:
        return _EVENT_TYPE_MAP

    import typing

    from . import events as events_module

    # Get all union members from EventPayload
    for member in typing.get_args(typing.get_args(events_module.EventPayload)[0]):
        # Each member has a 'type' field with a Literal default
        model_fields = member.model_fields
        if "type" in model_fields:
            default = model_fields["type"].default
            if default is not None:
                _EVENT_TYPE_MAP[default.value if isinstance(default, EventType) else str(default)] = member

    return _EVENT_TYPE_MAP


class EventStore:
    """SQLite-backed event store."""

    def __init__(self, db_path: str | Path = "games.db") -> None:
        """Initialize the event store with a SQLite database."""
        self.db_path = str(db_path)
        self.conn = sqlite3.connect(self.db_path)
        self.conn.executescript(CREATE_TABLE)

    def append(self, event: GameEvent) -> None:
        """Persist an event to the store."""
        self.conn.execute(
            "INSERT INTO events (id, game_id, sequence, timestamp, event_type, payload) VALUES (?, ?, ?, ?, ?, ?)",
            (
                str(event.id),
                str(event.game_id),
                event.sequence,
                event.timestamp.isoformat(),
                event.payload.type.value,
                event.payload.model_dump_json(),
            ),
        )
        self.conn.commit()

    def get_events(self, game_id: UUID, up_to_sequence: int | None = None) -> list[GameEvent]:
        """Load events for a game, optionally up to a sequence number."""
        query = "SELECT id, game_id, sequence, timestamp, event_type, payload FROM events WHERE game_id = ?"
        params: list[object] = [str(game_id)]
        if up_to_sequence is not None:
            query += " AND sequence <= ?"
            params.append(up_to_sequence)
        query += " ORDER BY sequence ASC"
        rows = self.conn.execute(query, params).fetchall()
        return [self._row_to_event(row) for row in rows]

    def get_latest_sequence(self, game_id: UUID) -> int:
        """Get the latest sequence number for a game, or -1 if no events exist."""
        row = self.conn.execute(
            "SELECT MAX(sequence) FROM events WHERE game_id = ?",
            (str(game_id),),
        ).fetchone()
        return row[0] if row[0] is not None else -1

    def get_all_game_ids(self) -> list[UUID]:
        """Get all distinct game IDs in the store."""
        rows = self.conn.execute("SELECT DISTINCT game_id FROM events ORDER BY game_id").fetchall()
        return [UUID(row[0]) for row in rows]

    def delete_after_sequence(self, game_id: UUID, after_sequence: int) -> int:
        """Delete events after a given sequence number. Returns count deleted."""
        cursor = self.conn.execute(
            "DELETE FROM events WHERE game_id = ? AND sequence > ?",
            (str(game_id), after_sequence),
        )
        self.conn.commit()
        return cursor.rowcount

    def fork_game(self, source_game_id: UUID, up_to_sequence: int) -> UUID:
        """Create a new game by copying events up to a certain point."""
        new_game_id = uuid4()
        events = self.get_events(source_game_id, up_to_sequence)
        for event in events:
            new_event = GameEvent(
                id=uuid4(),
                game_id=new_game_id,
                sequence=event.sequence,
                timestamp=event.timestamp,
                payload=event.payload,
            )
            self.append(new_event)
        return new_game_id

    def close(self) -> None:
        """Close the database connection."""
        self.conn.close()

    def _row_to_event(self, row: tuple[object, ...]) -> GameEvent:
        """Convert a database row to a GameEvent."""
        event_id, game_id, sequence, timestamp, event_type, payload_json = row
        type_map = _build_event_type_map()
        payload_cls = type_map.get(str(event_type))
        if payload_cls is None:
            raise ValueError(f"Unknown event type: {event_type}")

        payload_data = json.loads(str(payload_json))
        payload = cast(EventPayload, payload_cls.model_validate(payload_data))  # type: ignore[reportUnknownMemberType]

        return GameEvent(
            id=UUID(str(event_id)),
            game_id=UUID(str(game_id)),
            sequence=int(str(sequence)),
            timestamp=datetime.fromisoformat(str(timestamp)),
            payload=payload,
        )
