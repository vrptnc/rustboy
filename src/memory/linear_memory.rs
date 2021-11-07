use super::memory::Memory;

pub struct LinearMemory<const Size: usize> {
  start_address: u16,
  bytes: [u8; Size],
}

impl<const Size: usize> Memory for LinearMemory<Size> {
  fn read(&self, address: u16) -> u8 {
    self.bytes[address - self.start_address]
  }

  fn write(&mut self, address: u16, value: u8) {
    self.bytes[address - self.start_address] = value
  }
}

impl<const Size: usize> LinearMemory<Size> {
  pub fn new(start_address: u16) -> LinearMemory<Size> {
    LinearMemory {
      start_address,
      bytes: [0; Size],
    }
  }
}