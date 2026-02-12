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
            );
            CREATE TABLE IF NOT EXISTS friends (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                nickname TEXT,
                added_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS friend_requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_user TEXT NOT NULL,
                to_user TEXT NOT NULL,
                status TEXT DEFAULT 'pending',
                created_at TEXT NOT NULL,
                UNIQUE(from_user, to_user)
            );
            CREATE TABLE IF NOT EXISTS preferences (
                id INTEGER PRIMARY KEY,
                custom_background_path TEXT
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

    pub fn add_friend(&self, username: String, nickname: Option<String>) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO friends (username, nickname, added_at)
             VALUES (?, ?, datetime('now'))",
            params![&username, &nickname],
        )?;
        Ok(())
    }

    pub fn get_friends(&self) -> SqliteResult<Vec<(String, Option<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT username, nickname FROM friends ORDER BY added_at DESC",
        )?;
        let friends = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        let mut result = Vec::new();
        for friend in friends {
            result.push(friend?);
        }
        Ok(result)
    }

    pub fn remove_friend(&self, username: &str) -> SqliteResult<()> {
        self.conn.execute(
            "DELETE FROM friends WHERE username = ?",
            params![username],
        )?;
        Ok(())
    }

    pub fn add_friend_request(&self, from: String, to: String) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO friend_requests (from_user, to_user, created_at)
             VALUES (?, ?, datetime('now'))",
            params![&from, &to],
        )?;
        Ok(())
    }

    pub fn get_pending_friend_requests(&self, to_user: &str) -> SqliteResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT from_user FROM friend_requests WHERE to_user = ? AND status = 'pending'
             ORDER BY created_at DESC",
        )?;
        let requests = stmt.query_map(params![to_user], |row| {
            row.get(0)
        })?;

        let mut result = Vec::new();
        for req in requests {
            result.push(req?);
        }
        Ok(result)
    }

    pub fn respond_to_friend_request(&self, from: &str, to: &str, accepted: bool) -> SqliteResult<()> {
        if accepted {
            // Add to friends table
            self.conn.execute(
                "INSERT OR IGNORE INTO friends (username, added_at) VALUES (?, datetime('now'))",
                params![from],
            )?;
        }
        // Update request status
        let status = if accepted { "accepted" } else { "declined" };
        self.conn.execute(
            "UPDATE friend_requests SET status = ? WHERE from_user = ? AND to_user = ?",
            params![status, from, to],
        )?;
        Ok(())
    }

    pub fn save_background_path(&self, path: &str) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO preferences (id, custom_background_path) VALUES (1, ?)",
            params![path],
        )?;
        Ok(())
    }

    pub fn load_background_path(&self) -> SqliteResult<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT custom_background_path FROM preferences WHERE id = 1"
        )?;
        let result = stmt.query_row([], |row| row.get(0)).ok();
        Ok(result)
    }

    pub fn clear_background(&self) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE preferences SET custom_background_path = NULL WHERE id = 1",
            [],
        )?;
        Ok(())
    }

    pub fn save_remembered_username(&self, username: &str) -> SqliteResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO preferences (id, custom_background_path) VALUES (1, (SELECT custom_background_path FROM preferences WHERE id = 1 LIMIT 1))",
            [],
        )?;
        // Store username in a separate table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS app_settings (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )?;
        self.conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('remembered_username', ?)",
            params![username],
        )?;
        Ok(())
    }

    pub fn load_remembered_username(&self) -> SqliteResult<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT value FROM app_settings WHERE key = 'remembered_username'"
        )?;
        let result = stmt.query_row([], |row| row.get(0)).ok();
        Ok(result)
    }
}
