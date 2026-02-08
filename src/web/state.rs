use std::collections::HashMap;
use std::sync::Mutex;

use uuid::Uuid;

use crate::models::ForestInventory;

pub struct AppState {
    pub inventories: Mutex<HashMap<Uuid, ForestInventory>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inventories: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_inventory(&self, id: &Uuid) -> Option<ForestInventory> {
        self.inventories.lock().unwrap().get(id).cloned()
    }
}
