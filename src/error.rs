use std::fmt;

#[derive(Debug)]
pub struct QError(pub String, pub String);

impl QError {
    pub fn new(kind: impl Into<String>, message: String) -> Self {
        QError(kind.into(), message)
    }
}

impl fmt::Display for QError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "QueueError: {} {}", self.0, self.1)
    }
}
/// impl redis error
impl From<redis::RedisError> for QError {
    fn from(err: redis::RedisError) -> Self {
        QError::new("Redis error", err.to_string())
    }
}
/// impl serde_json error
impl From<serde_json::Error> for QError {
    fn from(err: serde_json::Error) -> Self {
        QError::new("JsonConvert", err.to_string())
    }
}

/// impl SystemTimeError
impl From<std::time::SystemTimeError> for QError {
    fn from(err: std::time::SystemTimeError) -> Self {
        QError::new("SystemTimeError", err.to_string())
    }
}
impl std::error::Error for QError {}
