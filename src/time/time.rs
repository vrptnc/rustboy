pub trait ClockAware {
  fn handle_tick(&mut self, double_speed: bool);

  fn tick(&mut self) {
    self.handle_tick(false);
  }

  fn ticks(&mut self, number_of_ticks: u32) {
    for _ in 0..number_of_ticks {
      self.handle_tick(false);
    }
  }

  fn double_tick(&mut self) {
    self.handle_tick(true);
  }
}