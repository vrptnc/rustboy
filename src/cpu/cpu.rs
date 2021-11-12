use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crate::memory::memory::Memory;

struct Opcode {
  opcode: u8,
}

// Opcode bit structure: xxyy yzzz
impl Opcode {
  fn new(opcode: u8) -> Opcode {
    Opcode {
      opcode
    }
  }

  fn value(&self) -> u8 {
    self.opcode
  }

  fn x_bits(&self) -> u8 {
    self.opcode >> 6 & 3
  }

  fn y_bits(&self) -> u8 {
    self.opcode >> 3 & 7
  }

  fn z_bits(&self) -> u8 {
    self.opcode & 7
  }
}

#[derive(Copy, Clone)]
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

struct CPU {
  registers: [u8; 10],
}


impl CPU {
  fn new() -> CPU {
    CPU {
      registers: [0;10],
    }
  }

  fn read_and_increment_register_pair(&mut self, register: Register) -> u16 {
    let value = self.read_register_pair(register);
    self.write_register_pair(register, value + 1);
    value
  }

  fn read_and_decrement_register_pair(&mut self, register: Register) -> u16 {
    let value = self.read_register_pair(register);
    self.write_register_pair(register, value - 1);
    value
  }

  pub fn read_next_instruction(&mut self, memory: &dyn Memory) -> u8 {
    memory.read(self.read_and_increment_register_pair(Register::PC) as usize)
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

  pub fn execute(&mut self, memory: &mut dyn Memory) {
    let opcode = Opcode::new(self.read_next_instruction(memory));
    let operation = match opcode.value() {
      0x00 => CPU::noop,
      0x02 => CPU::reg_A_to_indirect_BC_ld,
      0x06 => CPU::immediate_to_reg_ld,
      0x0A => CPU::indirect_BC_to_reg_A_ld,
      0x0E => CPU::immediate_to_reg_ld,
      0x12 => CPU::reg_A_to_indirect_DE_ld,
      0x16 => CPU::immediate_to_reg_ld,
      0x1A => CPU::indirect_DE_to_reg_A_ld,
      0x1E => CPU::immediate_to_reg_ld,
      0x26 => CPU::immediate_to_reg_ld,
      0x2A => CPU::indirect_HL_to_reg_A_ld_and_increment,
      0x2E => CPU::immediate_to_reg_ld,
      0x36 => CPU::immediate_to_indirect_ld,
      0x3A => CPU::indirect_HL_to_reg_A_ld_and_decrement,
      0x3E => CPU::immediate_to_reg_ld,
      0x40..=0x45 => CPU::reg_to_reg_ld,
      0x46 => CPU::indirect_to_reg_ld,
      0x47..=0x4D => CPU::reg_to_reg_ld,
      0x4E => CPU::indirect_to_reg_ld,
      0x4F => CPU::reg_to_reg_ld,
      0x50..=0x55 => CPU::reg_to_reg_ld,
      0x56 => CPU::indirect_to_reg_ld,
      0x57..=0x5D => CPU::reg_to_reg_ld,
      0x5E => CPU::indirect_to_reg_ld,
      0x5F => CPU::reg_to_reg_ld,
      0x60..=0x65 => CPU::reg_to_reg_ld,
      0x66 => CPU::indirect_to_reg_ld,
      0x67..=0x6D => CPU::reg_to_reg_ld,
      0x6E => CPU::indirect_to_reg_ld,
      0x6F => CPU::reg_to_reg_ld,
      0x70..=0x75 => CPU::reg_to_indirect_ld,
      0x77 => CPU::reg_to_indirect_ld,
      0x78..=0x7D => CPU::reg_to_reg_ld,
      0x7E => CPU::indirect_to_reg_ld,
      0x7F => CPU::reg_to_reg_ld,
      0xE0 => CPU::reg_A_to_immediate_indirect_with_offset_ld,
      0xE2 => CPU::reg_A_to_indirect_C_ld,
      0xEA => CPU::reg_A_to_immediate_indirect_ld,
      0xF0 => CPU::immediate_indirect_with_offset_to_reg_A_ld,
      0xF2 => CPU::indirect_C_with_offset_to_reg_A_ld,
      0xFA => CPU::immediate_indirect_to_reg_A_ld,
      _ => panic!("Unknown opcode"),
    };
    operation(self, opcode, memory)
  }

  fn noop(&mut self, _opcode: Opcode, _memory: &mut dyn Memory) {}

  fn reg_to_reg_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let src = Register::from_r_bits(opcode.z_bits());
    let dest = Register::from_r_bits(opcode.y_bits());
    self.write_register(dest, self.read_register(src));
  }

  fn immediate_to_reg_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let dest = Register::from_r_bits(opcode.y_bits());
    let value = self.read_next_instruction(memory);
    self.write_register(dest, value);
  }

  fn immediate_to_indirect_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = self.read_next_instruction(memory);
    memory.write(self.read_register_pair(Register::HL) as usize, value);
  }

  fn indirect_to_reg_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let dest = Register::from_r_bits(opcode.y_bits());
    let value = memory.read(self.read_register_pair(Register::HL) as usize);
    self.write_register(dest, value);
  }

  fn reg_to_indirect_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let src = Register::from_r_bits(opcode.z_bits());
    memory.write(self.read_register_pair(Register::HL) as usize, self.read_register(src));
  }

  fn indirect_BC_to_reg_A_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    self.write_register(Register::A, memory.read(self.read_register_pair(Register::BC) as usize));
  }

  fn indirect_DE_to_reg_A_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    self.write_register(Register::A, memory.read(self.read_register_pair(Register::DE) as usize));
  }

  fn indirect_C_with_offset_to_reg_A_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    self.write_register(Register::A, memory.read(0xFF00 + self.read_register(Register::C) as usize));
  }

  fn reg_A_to_indirect_C_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    memory.write(0xFF00 + self.read_register(Register::C) as usize, self.read_register(Register::A));
  }

  fn immediate_indirect_with_offset_to_reg_A_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let offset = self.read_next_instruction(memory) as usize;
    self.write_register(Register::A, memory.read(0xFF00 + offset));
  }

  fn reg_A_to_immediate_indirect_with_offset_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let offset = self.read_next_instruction(memory) as usize;
    memory.write(0xFF00 + offset, self.read_register(Register::A));
  }

  fn immediate_indirect_to_reg_A_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let lower_address = self.read_next_instruction(memory);
    let upper_address = self.read_next_instruction(memory);
    self.write_register(Register::A, memory.read((&[upper_address, lower_address][..]).read_u16::<BigEndian>().unwrap() as usize));
  }

  fn reg_A_to_immediate_indirect_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let lower_address = self.read_next_instruction(memory);
    let upper_address = self.read_next_instruction(memory);
    memory.write((&[upper_address, lower_address][..]).read_u16::<BigEndian>().unwrap() as usize, self.read_register(Register::A));
  }

  fn indirect_HL_to_reg_A_ld_and_increment(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = {
      memory.read(self.read_and_increment_register_pair(Register::HL) as usize)
    };
    self.write_register(Register::A, value);
  }

  fn indirect_HL_to_reg_A_ld_and_decrement(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = {
      memory.read(self.read_and_decrement_register_pair(Register::HL) as usize)
    };
    self.write_register(Register::A, value);
  }

  fn reg_A_to_indirect_BC_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    memory.write(self.read_register_pair(Register::BC) as usize, self.read_register(Register::A));
  }

  fn reg_A_to_indirect_DE_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    memory.write(self.read_register_pair(Register::DE) as usize, self.read_register(Register::A));
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::memory::memory::test::MockMemory;


  #[test]
  fn read_register() {
    let mut cpu = CPU::new();
    cpu.registers[2] = 0xAB;
    assert_eq!(cpu.read_register(Register::B), 0xAB);
  }

  #[test]
  fn read_register_pair() {
    let mut cpu = CPU::new();
    cpu.registers[2] = 0xAB;
    cpu.registers[3] = 0xCD;
    assert_eq!(cpu.read_register_pair(Register::BC), 0xABCD);
  }

  #[test]
  fn write_register() {
    let mut cpu = CPU::new();
    cpu.write_register(Register::B, 0xAB);
    assert_eq!(cpu.registers[2], 0xAB);
  }

  #[test]
  fn write_register_pair() {
    let mut cpu = CPU::new();
    cpu.write_register_pair(Register::BC, 0xABCD);
    assert_eq!(cpu.registers[2], 0xAB);
    assert_eq!(cpu.registers[3], 0xCD);
  }

  #[test]
  fn reg_to_reg_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    memory.write(0x0000, 0x45);
    cpu.write_register(Register::L, 0xAB);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::B), 0xAB);
  }

  #[test]
  fn immediate_to_reg_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    memory.write(0x0000, 0x06);
    memory.write(0x0001, 0xAB);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::B), 0xAB);
  }

  #[test]
  fn indirect_to_reg_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    memory.write(0x0000, 0x6E);
    memory.write(0xABCD, 0xEF);
    cpu.write_register_pair(Register::HL, 0xABCD);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::L), 0xEF);
  }

  #[test]
  fn reg_to_indirect_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register_pair(Register::HL, 0xABCD);
    cpu.write_register(Register::A, 0xEF);
    memory.write(0x0000, 0x77);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0xEF);
  }

  #[test]
  fn immediate_to_indirect_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x36);
    memory.write(0x0001, 0xEF);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0xEF);
  }

  #[test]
  fn indirect_BC_to_reg_A_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register_pair(Register::BC, 0xABCD);
    memory.write(0x0000, 0x0A);
    memory.write(0xABCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0x5A);
  }

  #[test]
  fn indirect_DE_to_reg_A_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register_pair(Register::DE, 0xABCD);
    memory.write(0x0000, 0x1A);
    memory.write(0xABCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0x5A);
  }

  #[test]
  fn indirect_C_with_offset_to_reg_A_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::C, 0xCD);
    memory.write(0x0000, 0xF2);
    memory.write(0xFFCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0x5A);
  }

  #[test]
  fn reg_A_to_indirect_C_with_offset_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, 0x5A);
    cpu.write_register(Register::C, 0xCD);
    memory.write(0x0000, 0xE2);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xFFCD), 0x5A);
  }

  #[test]
  fn immediate_indirect_with_offset_to_reg_A_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    memory.write(0x0000, 0xF0);
    memory.write(0x0001, 0xCD);
    memory.write(0xFFCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0x5A);
  }

  #[test]
  fn reg_A_to_immediate_indirect_with_offset_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, 0x5A);
    memory.write(0x0000, 0xE0);
    memory.write(0x0001, 0xCD);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xFFCD), 0x5A);
  }

  #[test]
  fn immediate_indirect_to_reg_A_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    memory.write(0x0000, 0xFA);
    memory.write(0x0001, 0xCD);
    memory.write(0x0002, 0xAB);
    memory.write(0xABCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0x5A);
  }

  #[test]
  fn reg_A_to_immediate_indirect_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, 0x5A);
    memory.write(0x0000, 0xEA);
    memory.write(0x0001, 0xCD);
    memory.write(0x0002, 0xAB);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
  }


  #[test]
  fn indirect_HL_to_reg_A_ld_and_increment() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x2A);
    memory.write(0xABCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
    assert_eq!(cpu.read_register_pair(Register::HL), 0xABCE);
  }

  #[test]
  fn indirect_HL_to_reg_A_ld_and_decrement() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x3A);
    memory.write(0xABCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
    assert_eq!(cpu.read_register_pair(Register::HL), 0xABCC);
  }

  #[test]
  fn reg_A_to_indirect_BC_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, 0x5A);
    cpu.write_register_pair(Register::BC, 0xABCD);
    memory.write(0x0000, 0x02);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
  }

  #[test]
  fn reg_A_to_indirect_DE_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, 0x5A);
    cpu.write_register_pair(Register::DE, 0xABCD);
    memory.write(0x0000, 0x12);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
  }

}
