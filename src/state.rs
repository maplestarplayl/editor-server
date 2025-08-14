use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
#[allow(unused)]
pub struct AppState {
    // Add shared state fields here if needed
}

pub type _SharedState = Arc<Mutex<AppState>>;
