use crate::memory::memory::Memory;

pub trait ClockAware {
  fn handle_tick(&mut self, memory: &mut dyn Memory, double_speed: bool);

  fn tick(&mut self, memory: &mut dyn Memory) {
    self.handle_tick(memory, false);
  }

  fn ticks(&mut self, memory: &mut dyn Memory, number_of_ticks: u32) {
    for _ in 0..number_of_ticks {
      self.handle_tick(memory, false);
    }
  }

  fn double_tick(&mut self, memory: &mut dyn Memory) {
    self.handle_tick(memory, true);
  }
}