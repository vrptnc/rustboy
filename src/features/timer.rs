use crate::time::duration::Duration;
use crate::time::time::TimingAware;

pub struct Timer {

}

impl TimingAware for Timer {
  fn tick(&mut self, delta: Duration) {

  }
}