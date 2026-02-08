use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use uuid::Uuid;

use crate::io::EditableTreeRow;
use crate::models::ForestInventory;

/// Maximum number of entries per map before oldest entries are evicted.
const MAX_INVENTORIES: usize = 100;
const MAX_PENDING: usize = 50;
/// Time-to-live for pending rows (30 minutes).
const PENDING_TTL_SECS: u64 = 30 * 60;
/// Time-to-live for stored inventories (2 hours).
const INVENTORY_TTL_SECS: u64 = 2 * 60 * 60;

pub struct AppState {
    pub inventories: Mutex<HashMap<Uuid, (Instant, ForestInventory)>>,
    /// Rows awaiting validation fixes before they can be stored as an inventory.
    pub pending_rows: Mutex<HashMap<Uuid, (Instant, String, Vec<EditableTreeRow>)>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inventories: Mutex::new(HashMap::new()),
            pending_rows: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_inventory(&self, id: &Uuid) -> Option<ForestInventory> {
        let mut map = self.inventories.lock().expect("inventories mutex poisoned");
        evict_expired_inventories(&mut map);
        map.get(id).map(|(_, inv)| inv.clone())
    }

    pub fn insert_inventory(&self, id: Uuid, inventory: ForestInventory) {
        let mut map = self.inventories.lock().expect("inventories mutex poisoned");
        evict_expired_inventories(&mut map);
        if map.len() >= MAX_INVENTORIES {
            evict_oldest_inventory(&mut map);
        }
        map.insert(id, (Instant::now(), inventory));
    }

    pub fn get_pending_name(&self, id: &Uuid) -> Option<String> {
        let mut map = self.pending_rows.lock().expect("pending_rows mutex poisoned");
        evict_expired_pending(&mut map);
        map.get(id).map(|(_, name, _)| name.clone())
    }

    pub fn has_pending(&self, id: &Uuid) -> bool {
        let mut map = self.pending_rows.lock().expect("pending_rows mutex poisoned");
        evict_expired_pending(&mut map);
        map.contains_key(id)
    }

    pub fn insert_pending(&self, id: Uuid, name: String, rows: Vec<EditableTreeRow>) {
        let mut map = self.pending_rows.lock().expect("pending_rows mutex poisoned");
        evict_expired_pending(&mut map);
        if map.len() >= MAX_PENDING {
            evict_oldest_pending(&mut map);
        }
        map.insert(id, (Instant::now(), name, rows));
    }

    pub fn remove_pending(&self, id: &Uuid) -> Option<(String, Vec<EditableTreeRow>)> {
        let mut map = self.pending_rows.lock().expect("pending_rows mutex poisoned");
        evict_expired_pending(&mut map);
        map.remove(id).map(|(_, name, rows)| (name, rows))
    }
}

fn evict_expired_inventories(map: &mut HashMap<Uuid, (Instant, ForestInventory)>) {
    let cutoff = Instant::now() - std::time::Duration::from_secs(INVENTORY_TTL_SECS);
    map.retain(|_, (created, _)| *created > cutoff);
}

fn evict_oldest_inventory(map: &mut HashMap<Uuid, (Instant, ForestInventory)>) {
    if let Some(oldest_id) = map.iter().min_by_key(|(_, (t, _))| *t).map(|(id, _)| *id) {
        map.remove(&oldest_id);
    }
}

fn evict_expired_pending(map: &mut HashMap<Uuid, (Instant, String, Vec<EditableTreeRow>)>) {
    let cutoff = Instant::now() - std::time::Duration::from_secs(PENDING_TTL_SECS);
    map.retain(|_, (created, _, _)| *created > cutoff);
}

fn evict_oldest_pending(map: &mut HashMap<Uuid, (Instant, String, Vec<EditableTreeRow>)>) {
    if let Some(oldest_id) = map.iter().min_by_key(|(_, (t, _, _))| *t).map(|(id, _)| *id) {
        map.remove(&oldest_id);
    }
}
