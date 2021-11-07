use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crate::memory::bank_memory::BankMemory;
use crate::memory::linear_memory::LinearMemory;
use crate::memory::memory::Memory;
use super::memory;

pub struct MainMemory<T> where T: Memory {
  rom: T,
  vram: BankMemory<0x2000, 2>, // Two banks of 8k VRAM memory, switched by VBK register (0xFF4F)
  ram: LinearMemory<0x1000>, //Bank 0 of RAM
  ram_banks: BankMemory<0x1000,7>, // Seven banks of switchable 4k RAM, switched by SVBK register (0xFF70),
  reserved_area_1: LinearMemory<0x1E00>, // In theory, this area is prohibited, but let's map it anyway
  oam: LinearMemory<0xA0>,
  reserved_area_2: LinearMemory<0x60>, // In theory, this area is prohibited, but let's map it anyway
  control_registers: LinearMemory<0x80>,
  stack: LinearMemory<127>,
  registers: [u8; 10],
  interrupt_enable: u8
}

impl<T> Memory for MainMemory<T> where T: Memory {
  fn read(&self, address: u16) -> u8 {
    match address {
      0x0000..=0x7FFF => self.rom.read(address),
      0x8000..=0x9FFF => self.vram.read(address),
      0xA000..=0xBFFF => self.rom.read(address),
      0xC000..=0xCFFF => self.ram.read(address),
      0xD000..=0xDFFF => self.ram_banks.read(address),
      0xE000..=0xFDFF => self.reserved_area_1.read(address),
      0xFE00..=0xFEBF => self.oam.read(address),
      0xFEA0..=0xFEFF => self.reserved_area_2.read(address),
      0xFF00..=0xFF7F => self.control_registers.read(address),
      0xFF80..=0xFFFE => self.stack.read(address),
      0xFFFF => self.interrupt_enable
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    todo!()
  }
}

impl<T> MainMemory<T> where T: Memory {
  pub fn new(rom: T) -> MainMemory<T> {
    return MainMemory {
      rom,
      ram: LinearMemory::<0x8000>::new(0xC000),
      registers: [0; 10],
    };
  }

  pub fn read_register(&self, register: Register) -> u8 {
    self.registers[register.offset()]
  }

  pub fn read_register_pair(&self, register: Register) -> u16 {
    (&self.registers[register.offset()..]).read_u16::<BigEndian>().unwrap()
  }

  pub fn write_register(&mut self, register: Register, value: u8) {
    self.registers[register.offset()] = value;
  }

  pub fn write_register_pair(&mut self, register: Register, value: u16) {
    (&mut self.registers[register.offset()..]).write_u16::<BigEndian>(value).unwrap();
  }
}

pub enum Register {
  A,
  F,
  AF,
  B,
  C,
  BC,
  D,
  E,
  DE,
  H,
  L,
  HL,
  PC,
  SP,
}

impl Register {
  fn offset(&self) -> usize {
    match self {
      Register::A => 0,
      Register::F => 1,
      Register::AF => 0,
      Register::B => 2,
      Register::C => 3,
      Register::BC => 2,
      Register::D => 4,
      Register::E => 5,
      Register::DE => 4,
      Register::H => 6,
      Register::L => 7,
      Register::HL => 6,
      Register::PC => 8,
      Register::SP => 10
    }
  }

  pub fn from_r_bits(bits: u8) -> Register {
    match bits {
      0b111 => Register::A,
      0b000 => Register::B,
      0b001 => Register::C,
      0b010 => Register::D,
      0b011 => Register::E,
      0b100 => Register::H,
      0b101 => Register::L,
      _ => panic!("{} doesn't map to a register", bits)
    }
  }

  // Also works for ss bits
  pub fn from_dd_bits(bits: u8) -> Register {
    match bits {
      0b00 => Register::BC,
      0b01 => Register::DE,
      0b10 => Register::HL,
      0b11 => Register::SP,
      _ => panic!("{} doesn't map to a register pair", bits)
    }
  }

  pub fn from_qq_bits(bits: u8) -> Register {
    match bits {
      0b00 => Register::BC,
      0b01 => Register::DE,
      0b10 => Register::HL,
      0b11 => Register::AF,
      _ => panic!("{} doesn't map to a register pair", bits)
    }
  }
}

#[cfg(test)]
mod test {
  use crate::memory::main::{MainMemory, Register};

  #[test]
  fn read_register() {
    let mut memory = MainMemory::new();
    memory.registers[2] = 0xAB;
    assert_eq!(memory.read_register(Register::B), 0xAB);
  }

  #[test]
  fn read_register_pair() {
    let mut memory = MainMemory::new();
    memory.registers[2] = 0xAB;
    memory.registers[3] = 0xCD;
    assert_eq!(memory.read_register_pair(Register::BC), 0xABCD);
  }

  #[test]
  fn write_register() {
    let mut memory = MainMemory::new();
    memory.write_register(Register::B, 0xAB);
    assert_eq!(memory.registers[2], 0xAB);
  }

  #[test]
  fn write_register_pair() {
    let mut memory = MainMemory::new();
    memory.write_register_pair(Register::BC, 0xABCD);
    assert_eq!(memory.registers[2], 0xAB);
    assert_eq!(memory.registers[3], 0xCD);
  }
}