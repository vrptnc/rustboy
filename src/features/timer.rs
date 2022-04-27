use crate::context::context::{Context, Executable};
use crate::time::duration::Duration;

pub struct Timer {
  total: Duration,

}

impl Executable for Timer {
  fn execute(&mut self, context: &mut Context) {

  }
}