use super::memory::Memory;

pub struct LinearMemory<const Size: usize> {
  bytes: [u8; Size],
}

impl<const Size: usize> Memory for LinearMemory<Size> {
  fn read(&self, address: usize) -> u8 {
    self.bytes[address]
  }

  fn write(&mut self, address: usize, value: u8) {
    self.bytes[address] = value
  }
}

impl<const Size: usize> LinearMemory<Size> {
  pub fn new() -> LinearMemory<Size> {
    LinearMemory {
      bytes: [0; Size],
    }
  }
}