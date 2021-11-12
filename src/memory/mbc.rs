#[cfg(test)]
pub mod test {
  use crate::memory::memory::Memory;

  pub struct MockMBC {
    lower_bytes: [u8; 0x8000],
    upper_bytes: [u8; 0x2000],
  }

  impl MockMBC {
    pub fn new() -> MockMBC {
      MockMBC {
        lower_bytes: [0; 0x8000],
        upper_bytes: [0; 0x2000],
      }
    }
  }

  impl Memory for MockMBC {
    fn read(&self, address: usize) -> u8 {
      match address {
        0x0000..=0x7FFF => self.lower_bytes[address],
        0xA000..=0xBFFF => self.upper_bytes[address],
        _ => panic!("Outside of MockMBC address space")
      }
    }

    fn write(&mut self, address: usize, value: u8) {
      match address {
        0x0000..=0x7FFF => self.lower_bytes[address] = value,
        0xA000..=0xBFFF => self.upper_bytes[address] = value,
        _ => panic!("Outside of MockMBC address space")
      }
    }
  }
}
