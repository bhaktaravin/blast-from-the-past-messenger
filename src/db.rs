use rusqlite::{Connection, Result as SqliteResult, params};
use crate::{ChatMessage, ChatTarget};
use std::path::PathBuf;

pub struct LocalDb {
    conn: Connection,
}

impl LocalDb {
    pub fn new() -> SqliteResult<Self> {
        let db_path = Self::get_db_path();
        std::fs::create_dir_all(db_path.parent().unwrap()).ok();
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn get_db_path() -> PathBuf {
        let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("chatmessenger");
        path.push("messages.db");
        path
    }

    fn init_schema(&self) -> SqliteResult<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                target TEXT NOT NULL,
                from_user TEXT NOT NULL,
                body TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                synced BOOLEAN DEFAULT 1
            );
            CREATE TABLE IF NOT EXISTS queued_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                target TEXT NOT NULL,
                to_user TEXT,
                body TEXT NOT NULL,
                created_at TEXT NOT NULL,
                sent BOOLEAN DEFAULT 0
            );",
        )?;
        Ok(())
    }

    pub fn save_message(
        &self,
        target: &ChatTarget,
        from: String,
        body: String,
        timestamp: String,
    ) -> SqliteResult<()> {
        let target_str = match target {
            ChatTarget::Lobby => "lobby".to_string(),
            ChatTarget::Direct(name) => format!("direct:{}", name),
        };
        self.conn.execute(
            "INSERT INTO messages (target, from_user, body, timestamp, synced)
             VALUES (?, ?, ?, ?, 1)",
            params![&target_str, &from, &body, &timestamp],
        )?;
        Ok(())
    }

    pub fn get_messages(&self, target: &ChatTarget) -> SqliteResult<Vec<ChatMessage>> {
        let target_str = match target {
            ChatTarget::Lobby => "lobby".to_string(),
            ChatTarget::Direct(name) => format!("direct:{}", name),
        };
        let mut stmt = self.conn.prepare(
            "SELECT from_user, body, timestamp FROM messages 
             WHERE target = ?
             ORDER BY id ASC",
        )?;
        let messages = stmt.query_map(params![&target_str], |row| {
            Ok(ChatMessage {
                from: row.get(0)?,
                body: row.get(1)?,
                at: row.get(2)?,
            })
        })?;

        let mut result = Vec::new();
        for msg in messages {
            result.push(msg?);
        }
        Ok(result)
    }

    pub fn queue_message(&self, target: &ChatTarget, body: String) -> SqliteResult<()> {
        let (target_str, to_user) = match target {
            ChatTarget::Lobby => ("lobby".to_string(), None),
            ChatTarget::Direct(name) => (format!("direct:{}", name), Some(name.clone())),
        };
        self.conn.execute(
            "INSERT INTO queued_messages (target, to_user, body, created_at, sent)
             VALUES (?, ?, ?, datetime('now'), 0)",
            params![&target_str, &to_user, &body],
        )?;
        Ok(())
    }

    pub fn get_queued_messages(&self) -> SqliteResult<Vec<(i32, ChatTarget, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, target, to_user, body FROM queued_messages WHERE sent = 0",
        )?;
        let messages = stmt.query_map([], |row| {
            let id: i32 = row.get(0)?;
            let target_str: String = row.get(1)?;
            let to_user: Option<String> = row.get(2)?;
            let body: String = row.get(3)?;
            let target = if target_str == "lobby" {
                ChatTarget::Lobby
            } else if let Some(name) = to_user {
                ChatTarget::Direct(name)
            } else {
                ChatTarget::Lobby
            };
            Ok((id, target, body, String::new()))
        })?;

        let mut result = Vec::new();
        for msg in messages {
            result.push(msg?);
        }
        Ok(result)
    }

    pub fn mark_queued_sent(&self, id: i32) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE queued_messages SET sent = 1 WHERE id = ?",
            params![id],
        )?;
        Ok(())
    }

    pub fn clear_old_messages(&self, days: i64) -> SqliteResult<()> {
        self.conn.execute(
            "DELETE FROM messages 
             WHERE timestamp < datetime('now', ? || ' days')",
            params![format!("-{}", days)],
        )?;
        Ok(())
    }
}
