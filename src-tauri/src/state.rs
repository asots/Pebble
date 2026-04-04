use pebble_store::Store;
use std::sync::Arc;

pub struct AppState {
    pub store: Arc<Store>,
}

impl AppState {
    pub fn new(store: Store) -> Self {
        Self { store: Arc::new(store) }
    }
}
