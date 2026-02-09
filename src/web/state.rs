use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use uuid::Uuid;

use crate::io::EditableTreeRow;
use crate::models::ForestInventory;

/// Maximum number of inventories before oldest is evicted.
const MAX_INVENTORIES: usize = 100;
/// Maximum number of pending row sets before oldest is evicted.
const MAX_PENDING: usize = 50;
/// Time-to-live for pending rows (30 minutes).
const PENDING_TTL_SECS: u64 = 30 * 60;
/// Time-to-live for stored inventories (2 hours).
const INVENTORY_TTL_SECS: u64 = 2 * 60 * 60;

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs()
}

pub struct AppState {
    db: Mutex<Connection>,
}

impl AppState {
    pub fn new() -> Self {
        let conn =
            Connection::open("forest_analyzer.db").expect("failed to open forest_analyzer.db");

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS inventories (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS pending_rows (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                rows TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );",
        )
        .expect("failed to create database tables");

        Self {
            db: Mutex::new(conn),
        }
    }

    pub fn get_inventory(&self, id: &Uuid) -> Option<ForestInventory> {
        let conn = self.db.lock().expect("db mutex poisoned");
        evict_expired(&conn, "inventories", INVENTORY_TTL_SECS);

        let mut stmt = conn
            .prepare("SELECT data FROM inventories WHERE id = ?1")
            .expect("failed to prepare inventory select");

        stmt.query_row([id.to_string()], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
    }

    pub fn insert_inventory(&self, id: Uuid, inventory: ForestInventory) {
        let conn = self.db.lock().expect("db mutex poisoned");
        evict_expired(&conn, "inventories", INVENTORY_TTL_SECS);
        evict_if_full(&conn, "inventories", MAX_INVENTORIES);

        let json = serde_json::to_string(&inventory).expect("failed to serialize inventory");
        conn.execute(
            "INSERT OR REPLACE INTO inventories (id, name, data, created_at) VALUES (?1, ?2, ?3, ?4)",
            (id.to_string(), &inventory.name, &json, unix_now()),
        )
        .expect("failed to insert inventory");
    }

    pub fn get_pending_name(&self, id: &Uuid) -> Option<String> {
        let conn = self.db.lock().expect("db mutex poisoned");
        evict_expired(&conn, "pending_rows", PENDING_TTL_SECS);

        let mut stmt = conn
            .prepare("SELECT name FROM pending_rows WHERE id = ?1")
            .expect("failed to prepare pending name select");

        stmt.query_row([id.to_string()], |row| row.get(0)).ok()
    }

    pub fn has_pending(&self, id: &Uuid) -> bool {
        let conn = self.db.lock().expect("db mutex poisoned");
        evict_expired(&conn, "pending_rows", PENDING_TTL_SECS);

        let mut stmt = conn
            .prepare("SELECT EXISTS(SELECT 1 FROM pending_rows WHERE id = ?1)")
            .expect("failed to prepare pending exists check");

        stmt.query_row([id.to_string()], |row| row.get::<_, bool>(0))
            .unwrap_or(false)
    }

    pub fn insert_pending(&self, id: Uuid, name: String, rows: Vec<EditableTreeRow>) {
        let conn = self.db.lock().expect("db mutex poisoned");
        evict_expired(&conn, "pending_rows", PENDING_TTL_SECS);
        evict_if_full(&conn, "pending_rows", MAX_PENDING);

        let json = serde_json::to_string(&rows).expect("failed to serialize pending rows");
        conn.execute(
            "INSERT OR REPLACE INTO pending_rows (id, name, rows, created_at) VALUES (?1, ?2, ?3, ?4)",
            (id.to_string(), &name, &json, unix_now()),
        )
        .expect("failed to insert pending rows");
    }

    pub fn remove_pending(&self, id: &Uuid) -> Option<(String, Vec<EditableTreeRow>)> {
        let conn = self.db.lock().expect("db mutex poisoned");
        evict_expired(&conn, "pending_rows", PENDING_TTL_SECS);

        let mut stmt = conn
            .prepare("SELECT name, rows FROM pending_rows WHERE id = ?1")
            .expect("failed to prepare pending select");

        let result = stmt
            .query_row([id.to_string()], |row| {
                let name: String = row.get(0)?;
                let json: String = row.get(1)?;
                Ok((name, json))
            })
            .ok();

        if let Some((name, json)) = result {
            conn.execute("DELETE FROM pending_rows WHERE id = ?1", [id.to_string()])
                .expect("failed to delete pending rows");
            let rows: Vec<EditableTreeRow> =
                serde_json::from_str(&json).expect("failed to deserialize pending rows");
            Some((name, rows))
        } else {
            None
        }
    }
}

/// Delete rows older than `ttl_secs` from the given table.
fn evict_expired(conn: &Connection, table: &str, ttl_secs: u64) {
    let cutoff = unix_now().saturating_sub(ttl_secs);
    // Table name is always a compile-time constant from our code, not user input.
    let sql = format!("DELETE FROM {table} WHERE created_at < ?1");
    let _ = conn.execute(&sql, [cutoff]);
}

/// If the table has reached `max` entries, delete the oldest one.
fn evict_if_full(conn: &Connection, table: &str, max: usize) {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    let count: usize = conn.query_row(&sql, [], |row| row.get(0)).unwrap_or(0);

    if count >= max {
        let delete_sql = format!(
            "DELETE FROM {table} WHERE id = (SELECT id FROM {table} ORDER BY created_at ASC LIMIT 1)"
        );
        let _ = conn.execute(&delete_sql, []);
    }
}
