//! SQLite-backed event store for game event persistence.
//!
//! Schema matches the Python implementation exactly (same table, columns, and
//! JSON payload shape), so a database written by either backend is readable by
//! the other.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::StoreError;
use crate::events::{EventPayload, GameEvent};

const CREATE_TABLE: &str = "
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
";

pub type StoreResult<T> = Result<T, StoreError>;

pub struct EventStore {
    conn: Connection,
}

impl EventStore {
    /// Open (or create) the event store at the given path.
    pub fn open(db_path: &str) -> StoreResult<Self> {
        let conn = Connection::open(db_path)?;
        configure(&conn)?;
        conn.execute_batch(CREATE_TABLE)?;
        Ok(EventStore { conn })
    }

    /// An in-memory store, for tests.
    pub fn in_memory() -> StoreResult<Self> {
        let conn = Connection::open_in_memory()?;
        configure(&conn)?;
        conn.execute_batch(CREATE_TABLE)?;
        Ok(EventStore { conn })
    }

    /// Persist an event to the store.
    pub fn append(&self, event: &GameEvent) -> StoreResult<()> {
        let payload_json = serde_json::to_string(&event.payload)?;
        self.conn.execute(
            "INSERT INTO events (id, game_id, sequence, timestamp, event_type, payload) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.id.to_string(),
                event.game_id.to_string(),
                event.sequence,
                event.timestamp.to_rfc3339(),
                event.payload.event_type(),
                payload_json,
            ],
        )?;
        Ok(())
    }

    /// Load events for a game, optionally up to (and including) a sequence.
    pub fn get_events(
        &self,
        game_id: Uuid,
        up_to_sequence: Option<i64>,
    ) -> StoreResult<Vec<GameEvent>> {
        let mut query = String::from(
            "SELECT id, game_id, sequence, timestamp, payload FROM events WHERE game_id = ?1",
        );
        if up_to_sequence.is_some() {
            query.push_str(" AND sequence <= ?2");
        }
        query.push_str(" ORDER BY sequence ASC");

        let mut stmt = self.conn.prepare(&query)?;
        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<RawRow> {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        };

        let rows = if let Some(seq) = up_to_sequence {
            stmt.query_map(params![game_id.to_string(), seq], map_row)?
        } else {
            stmt.query_map(params![game_id.to_string()], map_row)?
        };

        let mut events = Vec::new();
        for row in rows {
            events.push(row_to_event(row?)?);
        }
        Ok(events)
    }

    /// Latest sequence number for a game, or -1 if none.
    pub fn get_latest_sequence(&self, game_id: Uuid) -> StoreResult<i64> {
        let result: Option<i64> = self.conn.query_row(
            "SELECT MAX(sequence) FROM events WHERE game_id = ?1",
            params![game_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(result.unwrap_or(-1))
    }

    /// All distinct game IDs in the store, ordered by their string form.
    pub fn get_all_game_ids(&self) -> StoreResult<Vec<Uuid>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT game_id FROM events ORDER BY game_id")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut ids = Vec::new();
        for row in rows {
            let s = row?;
            ids.push(Uuid::parse_str(&s)?);
        }
        Ok(ids)
    }

    /// Delete events after a sequence number. Returns count deleted.
    pub fn delete_after_sequence(&self, game_id: Uuid, after_sequence: i64) -> StoreResult<usize> {
        let count = self.conn.execute(
            "DELETE FROM events WHERE game_id = ?1 AND sequence > ?2",
            params![game_id.to_string(), after_sequence],
        )?;
        Ok(count)
    }

    /// Create a new game by copying events up to a certain point.
    pub fn fork_game(&self, source_game_id: Uuid, up_to_sequence: i64) -> StoreResult<Uuid> {
        let new_game_id = Uuid::new_v4();
        let events = self.get_events(source_game_id, Some(up_to_sequence))?;
        for event in events {
            let new_event = GameEvent {
                id: Uuid::new_v4(),
                game_id: new_game_id,
                sequence: event.sequence,
                timestamp: event.timestamp,
                payload: event.payload,
            };
            self.append(&new_event)?;
        }
        Ok(new_game_id)
    }
}

/// Apply durability/concurrency PRAGMAs.
///
/// WAL gives better crash resilience and lets readers proceed during writes;
/// `busy_timeout` makes the connection wait (rather than instantly erroring)
/// if the database file is briefly locked by another process. Both are no-ops
/// for in-memory databases.
fn configure(conn: &Connection) -> StoreResult<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(())
}

/// A raw `events` row: (id, game_id, sequence, timestamp, payload).
type RawRow = (String, String, i64, String, String);

fn row_to_event((id, game_id, sequence, timestamp, payload): RawRow) -> StoreResult<GameEvent> {
    let payload: EventPayload = serde_json::from_str(&payload)?;
    let ts = DateTime::parse_from_rfc3339(&timestamp)
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    Ok(GameEvent {
        id: Uuid::parse_str(&id)?,
        game_id: Uuid::parse_str(&game_id)?,
        sequence,
        timestamp: ts,
        payload,
    })
}
