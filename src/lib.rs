use crate::error::QError;
use std::future::Future;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};
pub mod error;
pub mod job;
pub mod queue;

pub type QResult<T> = Result<T, QError>;
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = QResult<T>> + Send + 'a>>;

/// get current unix timestamp
pub fn timestamp() -> QResult<u64> {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH)?;
    let timestamp = since_the_epoch.as_secs();
    Ok(timestamp)
}
