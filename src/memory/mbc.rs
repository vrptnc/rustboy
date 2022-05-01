pub trait Loadable {
  fn load_byte(&mut self, address: usize, value: u8);
  fn load_bytes(&mut self, address: usize, values: &[u8]);
}
