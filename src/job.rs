use crate::QResult;
pub trait JobTrait {
    fn execute(&self) -> QResult<()>;
}
