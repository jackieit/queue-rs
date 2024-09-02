use crate::queue::Queue;
use crate::{QError, QResult};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub struct QueueTask {
    pub inner: Arc<Mutex<Queue>>,
}
impl QueueTask {
    /// init a queue by channel and redis client
    pub fn new(queue: Queue) -> Self {
        QueueTask {
            inner: Arc::new(Mutex::new(queue)),
        }
    }
    /// run a task to fetch all jobs and execute them
    /// timeout: the timeout of the job
    /// todo : the error msg should write to log file, not print to stdout now because of loop without a break
    pub fn listen(&self, timeout: u64) -> Result<(), QError> {
        let inner = Arc::clone(&self.inner);

        thread::spawn(move || -> QResult<()> {
            loop {
                thread::sleep(Duration::from_millis(300));
                let inner = inner.lock().unwrap();
                let job = inner.reserve(timeout);
                match job {
                    Ok(job) => {
                        let message_id = job.0;
                        let result = inner.handle_message(job);
                        if result.is_err() {
                            println!("{:?}", result.err());
                            continue;
                        }
                        let result = inner.delete(message_id);
                        if result.is_err() {
                            println!("{:?}", result.err());
                            continue;
                        }
                    }
                    Err(e) => {
                        println!("{:?}", e);
                        continue;
                    }
                }
            }
        })
        .join()
        .unwrap()
    }
}

// test
#[cfg(test)]
mod tests {
    // test run should work
    #[test]
    fn test_run() -> crate::QResult<()> {
        use super::QueueTask;
        use crate::queue::Queue;

        let queue = Queue::new("test", redis::Client::open("redis://127.0.0.1/").unwrap());
        let task = QueueTask::new(queue);
        task.listen(0)
    }
}
