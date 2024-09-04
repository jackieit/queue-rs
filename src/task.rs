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
    /// run all jobs in queue, loop until an error occur
    pub fn run(&self, timeout: u64) -> Result<(), QError> {
        let inner = Arc::clone(&self.inner);
        thread::spawn(move || -> QResult<()> {
            loop {
                let inner = inner.lock().unwrap();
                let job = inner.reserve(timeout)?;
                let message_id = job.0;
                inner.handle_message(job)?;
                inner.delete(message_id)?;
            }
        })
        .join()
        .unwrap()?;
        Ok(())
    }
    /// run a task to fetch all jobs and execute them
    /// timeout: the timeout of the job
    /// todo : the error msg should write to log file, not print to stdout now because of loop without a break
    pub fn listen(&self, timeout: u64) {
        let inner = Arc::clone(&self.inner);

        let _ = thread::spawn(move || loop {
            let inner = inner.lock().unwrap();
            let job = inner.reserve(timeout);
            match job {
                Ok(job) => {
                    let message_id = job.0;
                    let result = inner.handle_message(job);
                    if result.is_err() {
                        thread::sleep(Duration::from_millis(1000));
                        continue;
                    }
                    let result = inner.delete(message_id);
                    if result.is_err() {
                        thread::sleep(Duration::from_millis(1000));
                        continue;
                    }
                }
                Err(e) => {
                    println!("{:?}", e);
                    thread::sleep(Duration::from_millis(1000));
                    continue;
                }
            };
            thread::sleep(Duration::from_millis(1000));
        })
        .join()
        .unwrap();
    }
}

// test
#[cfg(test)]
mod tests {
    use tracing_subscriber;

    // test listen should work
    #[test]
    fn test_run() {
        use super::QueueTask;
        use crate::queue::Queue;

        let queue = Queue::new("test", redis::Client::open("redis://127.0.0.1/").unwrap());
        let task = QueueTask::new(queue);
        let _ = task.run(0);
    }
    // test run should work
    #[test]
    fn test_listen() {
        use super::QueueTask;
        use crate::queue::Queue;
        tracing_subscriber::fmt::init();
        let queue = Queue::new("test", redis::Client::open("redis://127.0.0.1/").unwrap());
        let task = QueueTask::new(queue);
        let _ = task.listen(1);
    }
}
