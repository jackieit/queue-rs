//! # queue-rs
//! A simple queue library for rust which execute delay and sync jobs.
//! Now ,it's only support redis now,may be support other queue in the future such as db,file and so on
//! ## Usage
//!
//! 1. how to add a job to queue
//!
//! ```rust
//! use queue_rs::queue::Queue;
//! use queue_rs::job::JobTrait;
//! use serde::{Deserialize, Serialize};
//! use queue_rs::{QResult,MakeJob};
//! // define a job struct
//! #[derive(Serialize, Deserialize)]
//! pub struct TestJob {
//!     pub name: String,
//!     //add some other attributes
//! }
//! impl TestJob {
//!    fn new(name: String) -> Self {
//!       TestJob { title }
//!    }
//! }
//! // impl JobTrait
//! #[MakeJob]
//! impl JobTrait for TestJob {
//!     fn execute(&self) -> QResult<()> {
//!        println!("test job [{}] executed", self.name);
//!        Ok(())
//!     }
//! }
//! let queue = Queue::new("queue-test", redis::Client::open("redis://127.0.0.1/").unwrap());
//! let _job_id = queue.push(TestJob::new("first job".to_string()));
//!
//! ```
//! 2. how add a delay job to queue
//! ```rust
//! let mut queue = Queue::new("queue-test", redis::Client::open("redis://127.0.0.1/").unwrap());
//! // will execute after 10 seconds
//! queue.delay(10)
//! let _job_id = queue.push(TestJob::new("first job".to_string()));
//!
//! ```
//! 3. how to listen the queue
//! ```rust
//! let queue = Queue::new("queue-test", redis::Client::open("redis://127.0.0.1/").unwrap());
//! let task  = QueueTask::new(queue);
//! task.listen(0);
//! ```
//! 4. how to run all jobs in queue, this will exit after all jobs executed
//! ```rust
//! let queue = Queue::new("queue-test", redis::Client::open("redis://127.0.0.1/").unwrap());
//! let task  = QueueTask::new(queue);
//! task.run(0);
//! ```
//! 5. tracing logs
//! add tracing-subscriber to cargo.toml
//! ```
//! tracing-subscriber="0.3"
//! ```
//! add tracing_subscriber::fmt::init();` to your main function, more info about (tracing)[https://github.com/tokio-rs/tracing/tree/master/tracing-subscriber]

use crate::error::QError;

use std::time::{SystemTime, UNIX_EPOCH};
pub use typetag::serde as MakeJob;
pub mod error;
pub mod job;
pub mod queue;
pub mod task;

pub type QResult<T> = Result<T, QError>;
//pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = QResult<T>> + Send + 'a>>;

/// get current unix timestamp
pub fn timestamp() -> QResult<u64> {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH)?;
    let timestamp = since_the_epoch.as_secs();
    Ok(timestamp)
}
// create a macro to convert error to QError
#[macro_export]
macro_rules! err {
    ( $msg:expr) => {
        Err(crate::error::QError("".to_string(), $msg.to_string()))
    };
}
