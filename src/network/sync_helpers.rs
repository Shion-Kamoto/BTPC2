// Helper functions for async-safe locking

use std::sync::{RwLock, RwLockWriteGuard};
use tokio::time::Duration;

pub async fn with_write_lock<T, F, R>(lock: &RwLock<T>, operation: F) -> R
where
    F: FnOnce(RwLockWriteGuard<'_, T>) -> R,
{
    let guard = lock.write().unwrap();
    let result = operation(guard);
    result
}

pub async fn sleep_without_lock(duration: Duration) {
    tokio::time::sleep(duration).await;
}
