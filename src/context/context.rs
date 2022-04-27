use crate::memory::memory::Memory;
use crate::time::duration::Duration;

pub trait Executable {
  fn execute(&mut self, context: &mut Context);
}

pub struct Context<'a> {
  pub memory: &'a mut dyn Memory,
  pub delta: Duration
}

impl<'a> Context<'a> {
  pub fn new(memory: &'a mut dyn Memory) -> Context<'a> {
    Context {
      memory,
      delta: Duration::new()
    }
  }
}