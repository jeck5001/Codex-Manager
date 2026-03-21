use rusqlite::{params, Result};

use super::{Event, Storage};

impl Storage {
    pub fn insert_event(&self, event: &Event) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (account_id, type, message, created_at) VALUES (?1, ?2, ?3, ?4)",
            (
                &event.account_id,
                &event.event_type,
                &event.message,
                event.created_at,
            ),
        )?;
        Ok(())
    }

    pub fn event_count(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(1) FROM events", [], |row| row.get(0))
    }

    pub fn list_recent_events_by_type(
        &self,
        event_type: &str,
        since_ts: i64,
        limit: i64,
    ) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT account_id, type, message, created_at
             FROM events
             WHERE type = ?1 AND created_at >= ?2
             ORDER BY created_at DESC
             LIMIT ?3",
        )?;
        let mut rows = stmt.query(params![event_type, since_ts, limit.max(1)])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(Event {
                account_id: row.get(0)?,
                event_type: row.get(1)?,
                message: row.get(2)?,
                created_at: row.get(3)?,
            });
        }
        Ok(out)
    }
}
