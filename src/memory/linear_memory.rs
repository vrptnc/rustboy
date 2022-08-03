use super::memory::Memory;

pub struct LinearMemory<const Size: usize, const StartAddress: u16> {
  bytes: [u8; Size],
}

impl<const Size: usize, const StartAddress: u16> Memory for LinearMemory<Size, StartAddress> {
  fn read(&self, address: u16) -> u8 {
    self.bytes[address as usize - StartAddress as usize]
  }

  fn write(&mut self, address: u16, value: u8) {
    self.bytes[address as usize - StartAddress as usize] = value
  }
}

impl<const Size: usize, const StartAddress: u16> LinearMemory<Size, StartAddress> {
  pub fn new() -> LinearMemory<Size, StartAddress> {
    LinearMemory {
      bytes: [0; Size],
    }
  }
}