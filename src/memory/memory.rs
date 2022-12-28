pub trait Memory {
  fn read(&self, address: u16) -> u8;
  fn write(&mut self, address: u16, value: u8);
}

pub struct MemoryAddress {}

impl MemoryAddress {
  const P1: u16 = 0xFF00; // Port P15-10
  const SB: u16 = 0xFF01; // Serial transfer register
  const SC: u16 = 0xFF02; // Serial control

  // Timer control
  const DIV: u16 = 0xFF04; // Divider
  const TIMA: u16 = 0xFF05; // Timer
  const TMA: u16 = 0xFF06; // Timer modulo
  const TAC: u16 = 0xFF07; // Timer control

  // LCD control
  const LCDC: u16 = 0xFF40; // LCDC control
  const STAT: u16 = 0xFF40; // LCDC control
  const SCY: u16 = 0xFF40; // LCDC control
  const SCX: u16 = 0xFF40; // LCDC control
  const WX: u16 = 0xFF40; // LCDC control
  const WY: u16 = 0xFF40; // LCDC control
  const LY: u16 = 0xFF40; // LCDC control
  const LYC: u16 = 0xFF40; // LCDC control

  // Palette control
  const BGP: u16 = 0xFF40; // LCDC control
  const OBP0: u16 = 0xFF40; // LCDC control
  const OBP1: u16 = 0xFF40; // LCDC control

  // DMA control
  const DMA: u16 = 0xFF40; // LCDC control


  // Interrupt control
  const IF: u16 = 0xFF0F; // Interrupt request flag
  const IE: u16 = 0xFFFF; // Interrupt enable flag
}

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
  pub fn from_byte(byte: u8) -> ROMSize {
    match byte {
      0x00 => ROMSize::KB32,
      0x01 => ROMSize::KB64,
      0x02 => ROMSize::KB128,
      0x03 => ROMSize::KB256,
      0x04 => ROMSize::KB512,
      0x05 => ROMSize::MB1,
      0x06 => ROMSize::MB2,
      0x07 => ROMSize::MB4,
      0x08 => ROMSize::MB8,
      _ => panic!("Byte {} does not correspond to any known ROM size", byte)
    }
  }

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
      ROMSize::MB8 => 0x800000
    }
  }
}

pub enum RAMSize {
  Unavailable,
  KB8,
  KB32,
  KB64,
  KB128,
}

impl RAMSize {
  pub fn from_byte(byte: u8) -> RAMSize {
    match byte {
      0x00 => RAMSize::Unavailable,
      0x01 => RAMSize::Unavailable,
      0x02 => RAMSize::KB8,
      0x03 => RAMSize::KB32,
      0x04 => RAMSize::KB128,
      0x05 => RAMSize::KB64,
      _ => panic!("Byte {} does not correspond to any known RAM size", byte)
    }
  }

  pub fn bytes(&self) -> usize {
    match self {
      RAMSize::Unavailable => 0,
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
