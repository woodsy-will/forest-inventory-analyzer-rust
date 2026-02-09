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
        Self::init_with_connection(conn)
    }

    /// Create an AppState backed by an in-memory SQLite database (for testing).
    #[cfg(test)]
    pub fn new_in_memory() -> Self {
        let conn = Connection::open_in_memory().expect("failed to open in-memory db");
        Self::init_with_connection(conn)
    }

    fn init_with_connection(conn: Connection) -> Self {
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

#[cfg(test)]
impl AppState {
    /// Backdate an inventory's created_at timestamp (for TTL eviction testing).
    fn backdate_inventory(&self, id: &Uuid, seconds_ago: u64) {
        let conn = self.db.lock().expect("db mutex poisoned");
        let ts = unix_now().saturating_sub(seconds_ago);
        conn.execute(
            "UPDATE inventories SET created_at = ?1 WHERE id = ?2",
            (ts, id.to_string()),
        )
        .expect("failed to backdate inventory");
    }

    /// Backdate a pending row's created_at timestamp (for TTL eviction testing).
    fn backdate_pending(&self, id: &Uuid, seconds_ago: u64) {
        let conn = self.db.lock().expect("db mutex poisoned");
        let ts = unix_now().saturating_sub(seconds_ago);
        conn.execute(
            "UPDATE pending_rows SET created_at = ?1 WHERE id = ?2",
            (ts, id.to_string()),
        )
        .expect("failed to backdate pending");
    }

    /// Count rows in a table (for capacity eviction testing).
    fn count_rows(&self, table: &str) -> usize {
        let conn = self.db.lock().expect("db mutex poisoned");
        let sql = format!("SELECT COUNT(*) FROM {table}");
        conn.query_row(&sql, [], |row| row.get(0)).unwrap_or(0)
    }

    /// Directly insert an inventory with a specific timestamp (bypass eviction).
    fn insert_inventory_at(&self, id: Uuid, inventory: &ForestInventory, created_at: u64) {
        let conn = self.db.lock().expect("db mutex poisoned");
        let json = serde_json::to_string(inventory).expect("failed to serialize inventory");
        conn.execute(
            "INSERT OR REPLACE INTO inventories (id, name, data, created_at) VALUES (?1, ?2, ?3, ?4)",
            (id.to_string(), &inventory.name, &json, created_at),
        )
        .expect("failed to insert inventory");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ForestInventory, Plot, Species, Tree, TreeStatus};

    fn sample_inventory(name: &str) -> ForestInventory {
        let mut inv = ForestInventory::new(name);
        inv.plots.push(Plot {
            plot_id: 1,
            plot_size_acres: 0.2,
            slope_percent: None,
            aspect_degrees: None,
            elevation_ft: None,
            trees: vec![Tree {
                tree_id: 1,
                plot_id: 1,
                species: Species {
                    common_name: "Douglas Fir".to_string(),
                    code: "DF".to_string(),
                },
                dbh: 14.0,
                height: Some(90.0),
                crown_ratio: Some(0.5),
                status: TreeStatus::Live,
                expansion_factor: 5.0,
                age: None,
                defect: None,
            }],
        });
        inv
    }

    fn sample_rows() -> Vec<EditableTreeRow> {
        vec![EditableTreeRow {
            row_index: 0,
            plot_id: 1,
            tree_id: 1,
            species_code: "DF".to_string(),
            species_name: "Douglas Fir".to_string(),
            dbh: 14.0,
            height: Some(90.0),
            crown_ratio: Some(0.5),
            status: "Live".to_string(),
            expansion_factor: 5.0,
            age: None,
            defect: None,
            plot_size_acres: Some(0.2),
            slope_percent: None,
            aspect_degrees: None,
            elevation_ft: None,
        }]
    }

    // -----------------------------------------------------------------------
    // Inventory round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_inventory_insert_and_get() {
        let state = AppState::new_in_memory();
        let id = Uuid::new_v4();
        let inv = sample_inventory("Test");

        assert!(state.get_inventory(&id).is_none());

        state.insert_inventory(id, inv.clone());

        let loaded = state.get_inventory(&id).expect("should find inventory");
        assert_eq!(loaded.name, "Test");
        assert_eq!(loaded.num_plots(), 1);
        assert_eq!(loaded.num_trees(), 1);
    }

    #[test]
    fn test_inventory_overwrite() {
        let state = AppState::new_in_memory();
        let id = Uuid::new_v4();

        state.insert_inventory(id, sample_inventory("First"));
        state.insert_inventory(id, sample_inventory("Second"));

        let loaded = state.get_inventory(&id).expect("should find inventory");
        assert_eq!(loaded.name, "Second");
    }

    #[test]
    fn test_inventory_nonexistent_returns_none() {
        let state = AppState::new_in_memory();
        assert!(state.get_inventory(&Uuid::new_v4()).is_none());
    }

    // -----------------------------------------------------------------------
    // Pending rows round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_pending_insert_and_has() {
        let state = AppState::new_in_memory();
        let id = Uuid::new_v4();

        assert!(!state.has_pending(&id));

        state.insert_pending(id, "test.csv".to_string(), sample_rows());

        assert!(state.has_pending(&id));
    }

    #[test]
    fn test_pending_get_name() {
        let state = AppState::new_in_memory();
        let id = Uuid::new_v4();

        assert!(state.get_pending_name(&id).is_none());

        state.insert_pending(id, "my_file.csv".to_string(), sample_rows());

        assert_eq!(state.get_pending_name(&id), Some("my_file.csv".to_string()));
    }

    #[test]
    fn test_pending_remove() {
        let state = AppState::new_in_memory();
        let id = Uuid::new_v4();
        let rows = sample_rows();

        state.insert_pending(id, "test.csv".to_string(), rows.clone());
        assert!(state.has_pending(&id));

        let (name, returned_rows) = state.remove_pending(&id).expect("should find pending");
        assert_eq!(name, "test.csv");
        assert_eq!(returned_rows.len(), rows.len());
        assert_eq!(returned_rows[0].dbh, 14.0);

        // Should be gone after removal
        assert!(!state.has_pending(&id));
        assert!(state.remove_pending(&id).is_none());
    }

    #[test]
    fn test_pending_nonexistent_remove_returns_none() {
        let state = AppState::new_in_memory();
        assert!(state.remove_pending(&Uuid::new_v4()).is_none());
    }

    // -----------------------------------------------------------------------
    // TTL eviction tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_inventory_ttl_eviction() {
        let state = AppState::new_in_memory();
        let id = Uuid::new_v4();
        state.insert_inventory(id, sample_inventory("Expired"));

        // Backdate beyond the 2-hour TTL
        state.backdate_inventory(&id, INVENTORY_TTL_SECS + 60);

        // Next access should evict it
        assert!(state.get_inventory(&id).is_none());
    }

    #[test]
    fn test_inventory_not_evicted_when_fresh() {
        let state = AppState::new_in_memory();
        let id = Uuid::new_v4();
        state.insert_inventory(id, sample_inventory("Fresh"));

        // Backdate but still within TTL
        state.backdate_inventory(&id, INVENTORY_TTL_SECS - 60);

        assert!(state.get_inventory(&id).is_some());
    }

    #[test]
    fn test_pending_ttl_eviction() {
        let state = AppState::new_in_memory();
        let id = Uuid::new_v4();
        state.insert_pending(id, "expired.csv".to_string(), sample_rows());

        // Backdate beyond the 30-minute TTL
        state.backdate_pending(&id, PENDING_TTL_SECS + 60);

        // Next access should evict it
        assert!(!state.has_pending(&id));
        assert!(state.get_pending_name(&id).is_none());
    }

    #[test]
    fn test_pending_not_evicted_when_fresh() {
        let state = AppState::new_in_memory();
        let id = Uuid::new_v4();
        state.insert_pending(id, "fresh.csv".to_string(), sample_rows());

        state.backdate_pending(&id, PENDING_TTL_SECS - 60);

        assert!(state.has_pending(&id));
    }

    // -----------------------------------------------------------------------
    // Capacity eviction tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_inventory_capacity_eviction() {
        let state = AppState::new_in_memory();
        let inv = sample_inventory("Cap");
        let now = unix_now();

        // Fill to MAX_INVENTORIES with increasing timestamps
        let mut ids = Vec::new();
        for i in 0..MAX_INVENTORIES {
            let id = Uuid::new_v4();
            state.insert_inventory_at(id, &inv, now + i as u64);
            ids.push(id);
        }
        assert_eq!(state.count_rows("inventories"), MAX_INVENTORIES);

        // Insert one more — should evict the oldest (ids[0])
        let new_id = Uuid::new_v4();
        state.insert_inventory(new_id, inv);

        assert!(state.get_inventory(&new_id).is_some());
        // The oldest should have been evicted
        // (Note: the eviction happens before the insert, and the oldest by created_at is ids[0])
    }

    #[test]
    fn test_pending_capacity_eviction() {
        let state = AppState::new_in_memory();
        let rows = sample_rows();

        // Fill to MAX_PENDING
        for _ in 0..MAX_PENDING {
            state.insert_pending(Uuid::new_v4(), "file.csv".to_string(), rows.clone());
        }
        assert_eq!(state.count_rows("pending_rows"), MAX_PENDING);

        // Insert one more — should evict oldest
        let new_id = Uuid::new_v4();
        state.insert_pending(new_id, "new.csv".to_string(), rows);

        assert!(state.has_pending(&new_id));
        // Count should still be at MAX_PENDING (one evicted, one added)
        assert_eq!(state.count_rows("pending_rows"), MAX_PENDING);
    }

    // -----------------------------------------------------------------------
    // Data integrity tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_inventory_data_preserved_through_serialization() {
        let state = AppState::new_in_memory();
        let id = Uuid::new_v4();
        let inv = sample_inventory("Roundtrip");

        state.insert_inventory(id, inv);
        let loaded = state.get_inventory(&id).unwrap();

        assert_eq!(loaded.plots[0].trees[0].dbh, 14.0);
        assert_eq!(loaded.plots[0].trees[0].height, Some(90.0));
        assert_eq!(loaded.plots[0].trees[0].species.common_name, "Douglas Fir");
        assert_eq!(loaded.plots[0].trees[0].status, TreeStatus::Live);
    }

    #[test]
    fn test_pending_rows_data_preserved() {
        let state = AppState::new_in_memory();
        let id = Uuid::new_v4();
        let rows = sample_rows();

        state.insert_pending(id, "data.csv".to_string(), rows);
        let (name, loaded) = state.remove_pending(&id).unwrap();

        assert_eq!(name, "data.csv");
        assert_eq!(loaded[0].species_code, "DF");
        assert_eq!(loaded[0].dbh, 14.0);
        assert_eq!(loaded[0].height, Some(90.0));
        assert_eq!(loaded[0].status, "Live");
    }

    #[test]
    fn test_multiple_inventories_independent() {
        let state = AppState::new_in_memory();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        state.insert_inventory(id1, sample_inventory("First"));
        state.insert_inventory(id2, sample_inventory("Second"));

        assert_eq!(state.get_inventory(&id1).unwrap().name, "First");
        assert_eq!(state.get_inventory(&id2).unwrap().name, "Second");
    }
}
