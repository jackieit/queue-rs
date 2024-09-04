use crate::job::JobTrait;
use crate::{err, timestamp, QResult};
use redis::{Commands, ExistenceCheck, SetExpiry, SetOptions};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument, span, Level};
/// task is waiting to be executed
const STATUS_WAITING: u8 = 1;
/// task is reserved
const STATUS_RESERVED: u8 = 2;
/// task has done
const STATUS_DONE: u8 = 3;
/// (message_id, message, ttr, attempts)
type JobMessage = (u64, String, u32, u32);

#[derive(Debug)]
pub struct Queue {
    /// The name of the queue
    channel: String,
    /// The redis client
    redis: redis::Client,
    /// The seconds to live of the job
    ttr: u32,
    /// The delay of the job
    delay: u32,
    /// The number of attempts default value 1
    attempts: u32,
}

impl Queue {
    /// Create a new queue
    /// # Arguments
    /// * `channel` - The name of the queue, used as the redis key prefix
    /// * `redis` - The redis client
    pub fn new(channel: impl Into<String>, redis: redis::Client) -> Self {
        Queue {
            channel: channel.into(),
            redis,
            ttr: 300,
            delay: 0,
            attempts: 1,
        }
    }
    /// Push a job to the queue
    pub fn push<'a, T: JobTrait + Serialize + Deserialize<'a>>(&self, job: T) -> QResult<u64> {
        //let mut conn = self.redis.get_connection()?;
        //conn.lpush(self.channel.clone(), job)?;
        let job = &job as &dyn JobTrait;
        let message = serde_json::to_string(job)?;
        //println!("Pushing message: {}", &message);
        let job_id = self.push_message(message)?;
        Ok(job_id)
    }
    /// push a message to redis queue
    fn push_message(&self, message: String) -> QResult<u64> {
        let mut conn = self.redis.get_connection()?;

        let id: u64 = conn.incr(self.k("message_id"), 1)?;

        conn.hset(self.k("messages"), id, format!("{};{}", self.ttr, message))?;
        let now = timestamp()?;
        if self.delay > 0 {
            conn.zadd(self.k("delayed"), id, now + self.delay as u64)?;
        } else {
            conn.lpush(self.k("waiting"), id)?;
        }
        Ok(id)
    }
    /// handle a message to execute
    #[instrument]
    pub fn handle_message(&self, job: JobMessage) -> QResult<()> {
        let (id, message, ttr, attempts) = job;
        let job: Box<dyn JobTrait> = serde_json::from_str(&message)?;
        let result = job.execute();
        match result {
            Err(e) => {
                info!(
                    "Executed job failed with error: [{}] , id:[{}],message:[{}],ttr:[{}],attampts:[{}]",
                    e.to_string(), id, &message, ttr, attempts
                );
            }
            Ok(_) => {
                info!(
                    "Executed job successed, id:[{}],message:[{}],ttr:[{}],attampts:[{}]",
                    id, &message, ttr, attempts
                );
            }
        }

        //self.delete(id)?;
        Ok(())
    }
    /// reserve a job, fetch the job from redis queue
    /// 1st Moves delayed and reserved jobs into waiting list with lock for one second
    /// 2nd find the job in waiting list
    /// return the job id, message, ttr, attempts as unit type
    #[instrument]
    pub fn reserve(&self, timeout: u64) -> QResult<JobMessage> {
        let span = span!(Level::TRACE, "Run Job ");
        let _enter = span.enter();
        let mut conn = self.redis.get_connection()?;
        let opts = SetOptions::default()
            .conditional_set(ExistenceCheck::NX)
            .with_expiration(SetExpiry::EX(1));
        let has_set: bool = conn.set_options(self.k("moving_lock"), true, opts)?;
        if has_set {
            info!("Moving delayed jobs into waiting list");
            self.move_expired("delayed")?;
            info!("Moving reserved jobs into waiting list");
            self.move_expired("reserved")?;
        }
        info!("Fetching job from waiting list");
        let id: u64 = if timeout == 0 {
            let id: Option<u64> = conn.rpop(self.k("waiting"), None)?;
            id.unwrap_or(0)
        } else {
            let id: Option<(String, u64)> = conn.brpop(self.k("waiting"), timeout as f64)?;
            match id {
                Some((_, id)) => id,
                None => 0,
            }
        };
        if id == 0 {
            error!("No job fetched from waiting list");
            return err!("No job found");
        }
        info!("Fetched job ID:[{}]", id);
        let payload: String = conn.hget(self.k("messages"), id)?;
        info!(
            "Fetched job ID:[{}] with Message:[{}] from waiting list",
            id, &payload
        );
        // split the payload as ttr and message
        let payload: Vec<&str> = payload.split(";").collect();
        let ttr: u32 = match payload[0].parse::<u32>() {
            Ok(ttr) => ttr,
            Err(_) => {
                error!(
                    "Parsed message ttr from payload ,Invalid ttr:[{}]",
                    payload[0]
                );
                return err!("Invalid ttr");
            }
        };
        let message: String = payload[1].to_string();
        let now = timestamp()?;

        conn.zadd(self.k("reserved"), id, now + ttr as u64)?;

        let attampts: u32 = conn.hincr(self.k("attempts"), id, 1)?;
        info!(
            "Fetched message successed id:[{}],message:[{}],ttr:[{}],attampts:[{}]",
            id, &message, ttr, attampts
        );
        //self.handle_message((id, message, ttr, attampts))?;
        Ok((id, message, ttr, attampts))
    }
    /// clear the queue
    pub fn clear(&self) -> QResult<()> {
        let mut conn = self.redis.get_connection()?;
        let pattern = self.k("*");
        let keys: Vec<String> = conn.scan_match(pattern)?.collect();
        //println!("=====Clearing queue: {:?}", keys);
        if !keys.is_empty() {
            conn.del(keys)?;
        }
        Ok(())
    }

    /// remove a job by id, if a job is runing it will be retried after 5 seconds
    pub fn remove(&self, message_id: u64) -> QResult<bool> {
        let mut conn = self.redis.get_connection()?;
        let opts = SetOptions::default()
            .conditional_set(ExistenceCheck::NX)
            .with_expiration(SetExpiry::EX(1));
        loop {
            let has_set: bool = conn.set_options(self.k("moving_lock"), true, opts)?;
            if has_set {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
        }

        let has_del: bool = conn.hdel(self.k("messages"), message_id)?;
        if has_del {
            conn.zrem(self.k("reserved"), message_id)?;
            conn.zrem(self.k("delayed"), message_id)?;
            conn.lrem(self.k("waiting"), 0, message_id)?;
            conn.hdel(self.k("attempts"), message_id)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// delete a job from redis queue
    #[instrument]
    pub fn delete(&self, message_id: u64) -> QResult<()> {
        let mut conn = self.redis.get_connection()?;
        conn.hdel(self.k("messages"), message_id)?;
        conn.hdel(self.k("attempts"), message_id)?;
        conn.zrem(self.k("reserved"), message_id)?;
        info!("Deleted message successed id:[{}]", message_id);
        Ok(())
    }
    /// move expired jobs [from] to waiting list
    fn move_expired(&self, from: &str) -> QResult<()> {
        let mut conn = self.redis.get_connection()?;
        let now = timestamp()?;
        let expired: Vec<u64> = conn.zrevrangebyscore(self.k(from), now, "-inf")?;
        conn.zrembyscore(self.k(from), "-inf", now)?;
        for id in expired {
            conn.rpush(self.k("waiting"), id)?;
        }
        Ok(())
    }

    /// get the status by message_id
    pub fn status(&self, message_id: u64) -> QResult<u8> {
        let mut conn = self.redis.get_connection()?;
        let status: bool = conn.hexists(self.k("attempts"), message_id)?;
        if status {
            return Ok(STATUS_RESERVED);
        }
        let status: bool = conn.hexists(self.k("messages"), message_id)?;
        if status {
            return Ok(STATUS_WAITING);
        }
        Ok(STATUS_DONE)
    }
    /// short for get redis key
    fn k(&self, key: &str) -> String {
        format!("{}.{}", self.channel, key)
    }
    /// set the channel for queue
    pub fn channel(&mut self, channel: impl Into<String>) -> &mut Self {
        self.channel = channel.into();
        self
    }
    /// set the redis client for queue
    pub fn redis(&mut self, redis: redis::Client) -> &mut Self {
        self.redis = redis;
        self
    }
    /// Set the time to live of the job
    pub fn ttl(&mut self, ttr: u32) -> &mut Self {
        self.ttr = ttr;
        self
    }
    /// Set the delay of the job
    pub fn delay(&mut self, delay: u32) -> &mut Self {
        self.delay = delay;
        self
    }
    /// Set the number of attempts
    pub fn attempts(&mut self, attempts: u32) -> &mut Self {
        self.attempts = attempts;
        self
    }
}

// test queue
#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    pub use typetag::serde as ThisJob;
    #[derive(Serialize, Deserialize)]
    struct TestJob {
        title: String,
    }
    impl TestJob {
        fn new(title: String) -> Self {
            TestJob { title }
        }
    }
    #[ThisJob]
    impl JobTrait for TestJob {
        fn execute(&self) -> QResult<()> {
            println!("test job [{}] executed", self.title);
            Ok(())
        }
    }

    // test queue init work
    #[test]
    fn test_queue_init() {
        let mut queue = Queue::new("test", redis::Client::open("redis://127.0.0.1/").unwrap());
        assert_eq!(queue.channel, "test");
        assert_eq!(queue.ttr, 300);
        assert_eq!(queue.delay, 0);
        assert_eq!(queue.attempts, 1);
        queue.delay(300);
        assert_eq!(queue.delay, 300);
    }
    // test redis option set work
    #[test]
    fn test_redis_option_set() {
        use redis::{Commands, ExistenceCheck, SetExpiry, SetOptions};
        let mut conn = redis::Client::open("redis://127.0.0.1/")
            .unwrap()
            .get_connection()
            .unwrap();
        let opts = SetOptions::default()
            .conditional_set(ExistenceCheck::NX)
            //  .get(true)
            .with_expiration(SetExpiry::EX(1));
        let has_set: bool = conn.set_options("test.lock", true, opts).unwrap();
        assert_eq!(has_set, true);
        let has_set: bool = conn.set_options("test.lock", true, opts).unwrap();
        assert_eq!(has_set, false);
    }

    // test add jobs work
    #[test]
    fn test_add_jobs() {
        let mut queue = Queue::new("test", redis::Client::open("redis://127.0.0.1/").unwrap());
        queue.delay(10);
        let job = queue.push(TestJob::new("first job".to_string()));
        assert_eq!(job.is_ok(), true);
    }
    // test clear all keys
    #[test]
    fn test_clear_all_keys() {
        let queue = Queue::new("test", redis::Client::open("redis://127.0.0.1/").unwrap());
        //queue.remove(1).unwrap();
        queue.clear().unwrap();
    }
    // test struct to json work
    #[test]
    fn test_struct_to_json() {
        let job = TestJob::new("first job".to_string());
        let event = &job as &dyn JobTrait;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "{\"type\":\"TestJob\",\"title\":\"first job\"}");
        let de: Box<dyn JobTrait> = serde_json::from_str(&json).unwrap();
        assert_eq!(de.execute().is_ok(), true);
    }
}
