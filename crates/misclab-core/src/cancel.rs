//! Cooperative cancellation. One token per analysis job; analyzers check it at
//! loop / rayon-chunk boundaries so long jobs (cracking, carving) stop promptly.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::error::CoreError;

#[derive(Clone, Default)]
pub struct CancellationToken {
    flag: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// Returns `Err(CoreError::Cancelled)` if cancellation was requested.
    pub fn check(&self) -> Result<(), CoreError> {
        if self.is_cancelled() {
            Err(CoreError::Cancelled)
        } else {
            Ok(())
        }
    }
}
