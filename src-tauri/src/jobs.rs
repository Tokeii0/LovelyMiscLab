//! Tracks in-flight graph runs so they can be cancelled by id. Each run gets a
//! `CancellationToken` (shared with the executor); `cancel` flips it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use misclab_core::cancel::CancellationToken;

#[derive(Default)]
pub struct JobManager {
    counter: AtomicU64,
    tokens: Mutex<HashMap<String, CancellationToken>>,
}

impl JobManager {
    /// Register a new job and return its id.
    pub fn start(&self, token: CancellationToken) -> String {
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        let id = format!("job-{n}");
        self.tokens.lock().unwrap().insert(id.clone(), token);
        id
    }

    pub fn cancel(&self, id: &str) {
        if let Some(token) = self.tokens.lock().unwrap().get(id) {
            token.cancel();
        }
    }

    pub fn finish(&self, id: &str) {
        self.tokens.lock().unwrap().remove(id);
    }
}
