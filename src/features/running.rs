use crate::common::errors::Result;

pub trait Runnable {
    fn run(self) -> Result<()>;
}
