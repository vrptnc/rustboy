use super::memory::Memory;

pub struct LinearMemory<const Size: usize> {
  start_address: usize,
  bytes: [u8; Size],
}

impl<const Size: usize> Memory for LinearMemory<Size> {
  fn read(&self, address: usize) -> u8 {
    self.bytes[address - self.start_address]
  }

  fn write(&mut self, address: usize, value: u8) {
    self.bytes[address - self.start_address] = value
  }
}

impl<const Size: usize> LinearMemory<Size> {
  pub fn new(start_address: usize) -> LinearMemory<Size> {
    LinearMemory {
      start_address,
      bytes: [0; Size],
    }
  }
}