pub trait Memory {
  fn read(&self, address: usize) -> u8;
  fn write(&mut self, address: usize, value: u8);
}

#[cfg(test)]
pub mod test {
  use crate::memory::memory::Memory;

  pub struct MockMemory {
    bytes: [u8; 0x10000],
  }

  impl MockMemory {
    pub fn new() -> MockMemory {
      MockMemory {
        bytes: [0; 0x10000]
      }
    }
  }

  impl Memory for MockMemory {
    fn read(&self, address: usize) -> u8 {
      self.bytes[address]
    }

    fn write(&mut self, address: usize, value: u8) {
      self.bytes[address] = value
    }
  }
}
