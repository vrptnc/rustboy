use super::memory::Memory;

pub struct LinearMemory<const Size: usize> {
  bytes: [u8; Size],
}

impl<const Size: usize> Memory for LinearMemory<Size> {
  fn read(&self, address: u16) -> u8 {
    self.bytes[address as usize]
  }

  fn write(&mut self, address: u16, value: u8) {
    self.bytes[address as usize] = value
  }
}

impl<const Size: usize> LinearMemory<Size> {
  pub fn new() -> LinearMemory<Size> {
    LinearMemory {
      bytes: [0; Size],
    }
  }
}