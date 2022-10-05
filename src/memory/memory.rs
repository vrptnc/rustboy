use std::cell::RefCell;
use std::rc::Rc;
use crate::MemoryBus;

pub trait Memory {
  fn read(&self, address: u16) -> u8;
  fn write(&mut self, address: u16, value: u8);
}

pub type MemoryRef = Rc<RefCell<MemoryBus>>;

pub enum ROMSize {
  KB32,
  KB64,
  KB128,
  KB256,
  KB512,
  MB1,
  MB2,
  MB4,
  MB8,
}

impl ROMSize {
  pub fn bytes(&self) -> usize {
    match self {
      ROMSize::KB32 => 0x8000,
      ROMSize::KB64 => 0x10000,
      ROMSize::KB128 => 0x20000,
      ROMSize::KB256 => 0x40000,
      ROMSize::KB512 => 0x80000,
      ROMSize::MB1 => 0x100000,
      ROMSize::MB2 => 0x200000,
      ROMSize::MB4 => 0x400000,
      ROMSize::MB8 => 0x800000,
    }
  }
}

pub enum RAMSize {
  NotAvailable,
  KB8,
  KB32,
  KB64,
  KB128,
}

impl RAMSize {
  pub fn bytes(&self) -> usize {
    match self {
      RAMSize::NotAvailable => 0,
      RAMSize::KB8 => 0x8000,
      RAMSize::KB32 => 0x8000,
      RAMSize::KB64 => 0x10000,
      RAMSize::KB128 => 0x20000,
    }
  }
}

#[derive(Copy, Clone)]
pub enum CGBMode {
  Monochrome,
  Color,
  PGB,
}

impl CGBMode {
  pub fn from_byte(byte: u8) -> CGBMode {
    match byte & 0xBF {
      0x00 => CGBMode::Monochrome,
      0x80 => CGBMode::Color,
      0x82 => CGBMode::PGB,
      0x84 => CGBMode::PGB,
      _ => panic!("Invalid CGB byte:  {:#x}", byte)
    }
  }
}

#[cfg(test)]
pub mod test {
  use crate::memory::memory::Memory;

  pub struct MockMemory {
    bytes: Vec<u8>,
  }

  impl MockMemory {
    pub fn new(bytes: usize) -> MockMemory {
      MockMemory {
        bytes: vec![0; bytes]
      }
    }
  }

  impl Memory for MockMemory {
    fn read(&self, address: u16) -> u8 {
      self.bytes[address as usize]
    }

    fn write(&mut self, address: u16, value: u8) {
      self.bytes[address as usize] = value
    }
  }
}
