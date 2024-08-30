use crate::job::JobTrait;
use crate::{timestamp, QResult};
use redis::Commands;
use serde::{Deserialize, Serialize};
pub struct Queue {
    /// The name of the queue
    channel: String,
    /// The redis client
    redis: redis::Client,
    /// The seconds to live of the job
    ttr: i32,
    /// The delay of the job
    delay: i32,
    /// The number of attempts default value 1
    attempts: i32,
}

impl Queue {
    /// Create a new queue
    pub fn new(channel: String, redis: redis::Client) -> Self {
        Queue {
            channel,
            redis,
            ttr: 300,
            delay: 0,
            attempts: 1,
            //priority: 0,
        }
    }
    /// Push a job to the queue
    pub fn push<'a, T: JobTrait + Serialize + Deserialize<'a> + Send>(
        &self,
        job: T,
    ) -> QResult<()> {
        //let mut conn = self.redis.get_connection()?;
        //conn.lpush(self.channel.clone(), job)?;
        let message = serde_json::to_string(&job)?;
        self.push_message(message)?;
        Ok(())
    }
    /// push a message to redis queue
    fn push_message(&self, message: String) -> QResult<i32> {
        let mut conn = self.redis.get_connection()?;

        let id: i32 = conn.incr(self.k("message_id"), 1)?;

        conn.hset(self.k("messages"), id, format!("{}:{}", self.ttr, message))?;
        let now = timestamp()?;
        if self.delay > 0 {
            conn.zadd(self.k("delayed"), id, now + self.delay as u64)?;
        } else {
            conn.lpush(self.k("waiting"), id)?;
        }
        Ok(id)
    }
    /// short for get redis key
    fn k(&self, key: &str) -> String {
        format!("{}.{}", self.channel, key)
    }

    /// Set the time to live of the job
    pub fn ttl(mut self, ttr: i32) -> Self {
        self.ttr = ttr;
        self
    }
    /// Set the delay of the job
    pub fn delay(mut self, delay: i32) -> Self {
        self.delay = delay;
        self
    }
    /// Set the number of attempts
    pub fn attempts(mut self, attempts: i32) -> Self {
        self.attempts = attempts;
        self
    }
}
