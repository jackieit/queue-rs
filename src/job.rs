use crate::QResult;
use serde::{Deserialize, Serialize};

#[typetag::serde(tag = "type")]
pub trait JobTrait {
    fn execute(&self) -> QResult<()>;
}
pub trait SerializeJob: JobTrait + Serialize + Sized + for<'de> Deserialize<'de> + Send {}
pub trait DeserializeJob: JobTrait + for<'de> Deserialize<'de> + Sized {}
