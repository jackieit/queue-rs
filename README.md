# queue-rs
 A simple queue library for rust which execute delay and sync jobs.
 Now ,it's only support redis ,may be support other queue later such as db,file and so on.
 ## Usage
 
 1. how to add a job to queue
 
 ```rust
 use queue_rs::queue::Queue;
 use serde::{Deserialize, Serialize};
 use queue_rs::{QResult,MakeJob};
 // define a job struct
 #[derive(Serialize, Deserialize)]
 pub struct TestJob {
     pub name: String,
     //add some other attributes
 }
 impl TestJob {
    fn new(name: String) -> Self {
       TestJob { title }
    }
 }
 // impl JobTrait
 #[MakeJob]
 impl JobTrait for TestJob {
     fn execute(&self) -> QResult<()> {
        println!("test job [{}] executed", self.name);
        Ok(())
     }
 }
 let queue = Queue::new("queue-test", redis::Client::open("redis://127.0.0.1/").unwrap());
 let _job_id = queue.push(TestJob::new("first job".to_string()));
//!
 ```
 2. how add a delay job to queue
 ```rust
 let mut queue = Queue::new("queue-test", redis::Client::open("redis://127.0.0.1/").unwrap());
 // will execute after 10 seconds
 queue.delay(10)
 let _job_id = queue.push(TestJob::new("first job".to_string()));
//!
 ```
 3. how to listen the queue
 ```rust
 let queue = Queue::new("queue-test", redis::Client::open("redis://127.0.0.1/").unwrap());
 let task  = QueueTask::new(queue);
 task.listen(0);
 ```
 4. how to run all jobs in queue, this will exit after all jobs executed
 ```rust
 let queue = Queue::new("queue-test", redis::Client::open("redis://127.0.0.1/").unwrap());
 let task  = QueueTask::new(queue);
 task.run(0);
 ```
 5. tracing logs
 add tracing-subscriber to cargo.toml
 ```
 tracing-subscriber="0.3"
 ```
 add tracing_subscriber::fmt::init();` to your main function, more info about [tracing](https://github.com/tokio-rs/tracing/tree/master/tracing-subscriber)