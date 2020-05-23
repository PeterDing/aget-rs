use crate::common::errors::Result;

pub trait Runnable {
    fn run(&mut self) -> Result<()>;
}
