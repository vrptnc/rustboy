use std::ops::{BitAnd, Shr};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crate::cpu::alu::ALU;
use crate::memory::memory::Memory;
use crate::util::bit_util::BitUtil;
use super::alu;

struct Opcode {
  opcode: u8,
}

// Opcode bit structure: xxyy yzzz
// Opcode bit structure: xxdd xxxx
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

  fn dd_bits(&self) -> u8 {
    self.opcode >> 4 & 3
  }

  fn qq_bits(&self) -> u8 {
    self.opcode >> 4 & 3
  }
}

#[derive(Copy, Clone)]
pub enum Register {
  A,
  F, // Z | N | H | CY | x | x | x | x    Z: 1 if result was 0, N: 1 if previous op was subtraction, H: carry from bit 3, CY: carry from bit 7
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
  registers: [u8; 12],
}


impl CPU {
  fn new() -> CPU {
    CPU {
      registers: [0; 12],
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
      0x01 => CPU::immediate_to_reg_pair_ld,
      0x02 => CPU::reg_A_to_indirect_BC_ld,
      0x06 => CPU::immediate_to_reg_ld,
      0x08 => CPU::reg_SP_to_immediate_indirect_ld,
      0x0A => CPU::indirect_BC_to_reg_A_ld,
      0x0E => CPU::immediate_to_reg_ld,
      0x11 => CPU::immediate_to_reg_pair_ld,
      0x12 => CPU::reg_A_to_indirect_DE_ld,
      0x16 => CPU::immediate_to_reg_ld,
      0x1A => CPU::indirect_DE_to_reg_A_ld,
      0x1E => CPU::immediate_to_reg_ld,
      0x21 => CPU::immediate_to_reg_pair_ld,
      0x22 => CPU::reg_A_to_indirect_HL_ld_and_increment,
      0x26 => CPU::immediate_to_reg_ld,
      0x2A => CPU::indirect_HL_to_reg_A_ld_and_increment,
      0x2E => CPU::immediate_to_reg_ld,
      0x31 => CPU::immediate_to_reg_pair_ld,
      0x32 => CPU::reg_A_to_indirect_HL_ld_and_decrement,
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
      0x80..=0x85 => CPU::add_reg_to_reg_A_and_write_to_reg_A,
      0x86 => CPU::add_indirect_HL_to_reg_A_and_write_to_reg_A,
      0x87 => CPU::add_reg_to_reg_A_and_write_to_reg_A,
      0x88..=0x8D => CPU::add_reg_with_carry_to_reg_A_and_write_to_reg_A,
      0x8E => CPU::add_indirect_HL_with_carry_to_reg_A_and_write_to_reg_A,
      0x8F => CPU::add_reg_with_carry_to_reg_A_and_write_to_reg_A,
      0x90..=0x95 => CPU::subtract_reg_from_reg_A_and_write_to_reg_A,
      0x96 => CPU::subtract_indirect_HL_from_reg_A_and_write_to_reg_A,
      0x97 => CPU::subtract_reg_from_reg_A_and_write_to_reg_A,
      0x98..=0x9D => CPU::subtract_reg_with_carry_from_reg_A_and_write_to_reg_A,
      0x9E => CPU::subtract_indirect_HL_with_carry_from_reg_A_and_write_to_reg_A,
      0xC1 => CPU::pop_stack_to_reg_pair,
      0xC5 => CPU::push_reg_pair_to_stack,
      0xC6 => CPU::add_immediate_to_reg_A_and_write_to_reg_A,
      0xCE => CPU::add_immediate_with_carry_to_reg_A_and_write_to_reg_A,
      0xD1 => CPU::pop_stack_to_reg_pair,
      0xD5 => CPU::push_reg_pair_to_stack,
      0xD6 => CPU::subtract_immediate_from_reg_A_and_write_to_reg_A,
      0xDE => CPU::subtract_immediate_with_carry_from_reg_A_and_write_to_reg_A,
      0xE0 => CPU::reg_A_to_immediate_indirect_with_offset_ld,
      0xE1 => CPU::pop_stack_to_reg_pair,
      0xE2 => CPU::reg_A_to_indirect_C_ld,
      0xE5 => CPU::push_reg_pair_to_stack,
      0xEA => CPU::reg_A_to_immediate_indirect_ld,
      0xF0 => CPU::immediate_indirect_with_offset_to_reg_A_ld,
      0xF1 => CPU::pop_stack_to_reg_pair,
      0xF2 => CPU::indirect_C_with_offset_to_reg_A_ld,
      0xF5 => CPU::push_reg_pair_to_stack,
      0xF8 => CPU::reg_SP_plus_signed_immediate_to_HL_ld,
      0xF9 => CPU::reg_HL_to_reg_SP_ld,
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

  fn reg_A_to_indirect_HL_ld_and_increment(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    memory.write(self.read_and_increment_register_pair(Register::HL) as usize, self.read_register(Register::A));
  }

  fn reg_A_to_indirect_HL_ld_and_decrement(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    memory.write(self.read_and_decrement_register_pair(Register::HL) as usize, self.read_register(Register::A));
  }

  fn immediate_to_reg_pair_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let lower_bits = self.read_next_instruction(memory);
    let upper_bits = self.read_next_instruction(memory);
    let value = (&[upper_bits, lower_bits][..]).read_u16::<BigEndian>().unwrap();
    self.write_register_pair(Register::from_dd_bits(opcode.dd_bits()), value);
  }

  fn reg_HL_to_reg_SP_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = self.read_register_pair(Register::HL);
    self.write_register_pair(Register::SP, value)
  }

  fn push_reg_pair_to_stack(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = self.read_register_pair(Register::from_qq_bits(opcode.qq_bits())).to_be_bytes();
    memory.write(self.read_and_decrement_register_pair(Register::SP) as usize, value[0]);
    memory.write(self.read_and_decrement_register_pair(Register::SP) as usize, value[1]);
  }

  fn pop_stack_to_reg_pair(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let lower_bits = memory.read(self.read_and_increment_register_pair(Register::SP) as usize);
    let upper_bits = memory.read(self.read_and_increment_register_pair(Register::SP) as usize);
    let value = (&[upper_bits, lower_bits][..]).read_u16::<BigEndian>().unwrap();
    self.write_register_pair(Register::from_qq_bits(opcode.qq_bits()), value);
  }

  // TODO: Do a more thorough check to see if this is correct. There seems to be a lot of confusion surrounding the (half) carry bits
  fn reg_SP_plus_signed_immediate_to_HL_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let signed_value = self.read_next_instruction(memory) as i8 as u16;
    let reg_sp = self.read_register_pair(Register::SP);
    let result = reg_sp.wrapping_add(signed_value);
    let temp = ((!result & (reg_sp | signed_value)) | (reg_sp & signed_value)).to_be_bytes()[0];
    self.write_register(Register::F, (temp & 0x80).wrapping_shr(3) | ((temp & 0x08).wrapping_shl(2)));
    self.write_register_pair(Register::HL, result);
  }

  fn reg_SP_to_immediate_indirect_ld(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let lower_address = self.read_next_instruction(memory);
    let upper_address = self.read_next_instruction(memory);
    let address = (&[upper_address, lower_address][..]).read_u16::<BigEndian>().unwrap() as usize;
    let sp = self.read_register_pair(Register::SP).to_be_bytes();
    memory.write(address, sp[1]);
    memory.write(address + 1, sp[0]);
  }

  fn add_value_to_reg_A_and_write_to_reg_A(&mut self, value: u8) {
    let reg_a = self.read_register(Register::A);
    let result = ALU::add(reg_a, value);
    self.write_register(Register::F, u8::compose(&[(result.zero, 7), (result.half_carry, 5), (result.carry, 4)]));
    self.write_register(Register::A, result.value);
  }

  fn add_reg_to_reg_A_and_write_to_reg_A(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = self.read_register(Register::from_r_bits(opcode.z_bits()));
    self.add_value_to_reg_A_and_write_to_reg_A(value);
  }

  fn add_immediate_to_reg_A_and_write_to_reg_A(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = self.read_next_instruction(memory);
    self.add_value_to_reg_A_and_write_to_reg_A(value);
  }

  fn add_indirect_HL_to_reg_A_and_write_to_reg_A(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let address = self.read_register_pair(Register::HL) as usize;
    self.add_value_to_reg_A_and_write_to_reg_A(memory.read(address));
  }

  fn subtract_value_from_reg_A_and_write_to_reg_A(&mut self, value: u8) {
    let reg_a = self.read_register(Register::A);
    let result = ALU::subtract(reg_a, value);
    self.write_register(Register::F, u8::compose(&[(result.zero, 7), (true, 6), (result.half_carry, 5), (result.carry, 4)]));
    self.write_register(Register::A, result.value);
  }

  fn subtract_reg_from_reg_A_and_write_to_reg_A(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = self.read_register(Register::from_r_bits(opcode.z_bits()));
    self.subtract_value_from_reg_A_and_write_to_reg_A(value);
  }

  fn subtract_immediate_from_reg_A_and_write_to_reg_A(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = self.read_next_instruction(memory);
    self.subtract_value_from_reg_A_and_write_to_reg_A(value);
  }

  fn subtract_indirect_HL_from_reg_A_and_write_to_reg_A(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let address = self.read_register_pair(Register::HL) as usize;
    self.subtract_value_from_reg_A_and_write_to_reg_A(memory.read(address));
  }

  fn subtract_value_with_carry_from_reg_A_and_write_to_reg_A(&mut self, value: u8) {
    let reg_a = self.read_register(Register::A);
    let result = ALU::subtract_with_carry(reg_a, value, self.read_register(Register::F).get_bit(4));
    self.write_register(Register::F, u8::compose(&[(result.zero, 7), (true, 6), (result.half_carry, 5), (result.carry, 4)]));
    self.write_register(Register::A, result.value);
  }

  fn subtract_reg_with_carry_from_reg_A_and_write_to_reg_A(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = self.read_register(Register::from_r_bits(opcode.z_bits()));
    self.subtract_value_with_carry_from_reg_A_and_write_to_reg_A(value);
  }

  fn subtract_immediate_with_carry_from_reg_A_and_write_to_reg_A(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = self.read_next_instruction(memory);
    self.subtract_value_with_carry_from_reg_A_and_write_to_reg_A(value);
  }

  fn subtract_indirect_HL_with_carry_from_reg_A_and_write_to_reg_A(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let address = self.read_register_pair(Register::HL) as usize;
    self.subtract_value_with_carry_from_reg_A_and_write_to_reg_A(memory.read(address));
  }

  fn add_value_with_carry_to_reg_A_and_write_to_reg_A(&mut self, value: u8) {
    let reg_a = self.read_register(Register::A);
    let carry = self.read_register(Register::F).get_bit(4);
    let result = ALU::add_with_carry(reg_a, value, carry);
    self.write_register(Register::F, u8::compose(&[(result.zero, 7), (result.half_carry, 5), (result.carry, 4)]));
    self.write_register(Register::A, result.value);
  }

  fn add_reg_with_carry_to_reg_A_and_write_to_reg_A(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = self.read_register(Register::from_r_bits(opcode.z_bits()));
    self.add_value_with_carry_to_reg_A_and_write_to_reg_A(value);
  }

  fn add_immediate_with_carry_to_reg_A_and_write_to_reg_A(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let value = self.read_next_instruction(memory);
    self.add_value_with_carry_to_reg_A_and_write_to_reg_A(value);
  }

  fn add_indirect_HL_with_carry_to_reg_A_and_write_to_reg_A(&mut self, opcode: Opcode, memory: &mut dyn Memory) {
    let address = self.read_register_pair(Register::HL) as usize;
    self.add_value_with_carry_to_reg_A_and_write_to_reg_A(memory.read(address));
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::memory::memory::test::MockMemory;
  use test_case::test_case;

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

  #[test]
  fn reg_A_to_indirect_HL_ld_and_increment() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, 0x5A);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x22);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
    assert_eq!(cpu.read_register_pair(Register::HL), 0xABCE);
  }

  #[test]
  fn reg_A_to_indirect_HL_ld_and_decrement() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, 0x5A);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x32);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
    assert_eq!(cpu.read_register_pair(Register::HL), 0xABCC);
  }


  #[test]
  fn immediate_to_reg_pair_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, 0x5A);
    memory.write(0x0000, 0x21);
    memory.write(0x0001, 0x5A);
    memory.write(0x0002, 0x7B);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::HL), 0x7B5A);
  }

  #[test]
  fn reg_HL_to_reg_SP_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xF9);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::SP), 0xABCD);
  }

  #[test]
  fn push_reg_pair_to_stack() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register_pair(Register::SP, 0xFFFE);
    cpu.write_register_pair(Register::DE, 0xABCD);
    memory.write(0x0000, 0xD5);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xFFFE), 0xAB);
    assert_eq!(memory.read(0xFFFD), 0xCD);
    assert_eq!(cpu.read_register_pair(Register::SP), 0xFFFC);
  }

  #[test]
  fn pop_stack_to_reg_pair() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register_pair(Register::SP, 0xFFFC);
    memory.write(0x0000, 0xD1);
    memory.write(0xFFFC, 0xCD);
    memory.write(0xFFFD, 0xAB);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::DE), 0xABCD);
    assert_eq!(cpu.read_register_pair(Register::SP), 0xFFFE);
  }

  #[test]
  fn reg_SP_plus_signed_immediate_to_HL_ld_writes_correct_result() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    // Check if carry flag is set correctly
    cpu.write_register_pair(Register::SP, 0x0005);
    memory.write(0x0000, 0xF8);
    memory.write(0x0001, 0xFD);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::HL), 0x0002);
  }

  #[test_case(0x0FF8, 0x07, 0x00 ; "no flags")]
  #[test_case(0x0FF8, 0x08, 0x20 ; "only half carry")]
  #[test_case(0xFFF8, 0x08, 0x30 ; "both carry flags")]
  fn reg_SP_plus_signed_immediate_to_HL_ld_writes_correct_flags(sp: u16, e: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register_pair(Register::SP, sp);
    memory.write(0x0000, 0xF8);
    memory.write(0x0001, e);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test]
  fn reg_SP_to_immediate_indirect_ld() {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register_pair(Register::SP, 0x7B5A);
    memory.write(0x0000, 0x08);
    memory.write(0x0001, 0xCD);
    memory.write(0x0002, 0xAB);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
    assert_eq!(memory.read(0xABCE), 0x7B);
  }

  #[test_case(0xFC, 0x04, 0x00, 0xB0 ; "zero flag set correctly")]
  #[test_case(0xF0, 0xF0, 0xE0, 0x10 ; "carry set correctly")]
  #[test_case(0x08, 0x08, 0x10, 0x20 ; "half carry set correctly")]
  fn add_reg_to_reg_A_and_write_to_reg_A(a: u8, value: u8, result: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0x82);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0x04, 0x00, 0xB0 ; "zero flag set correctly")]
  #[test_case(0xF0, 0xF0, 0xE0, 0x10 ; "carry set correctly")]
  #[test_case(0x08, 0x08, 0x10, 0x20 ; "half carry set correctly")]
  fn add_immediate_to_reg_A_and_write_to_reg_A(a: u8, value: u8, result: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, a);
    memory.write(0x0000, 0xC6);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0x04, 0x00, 0xB0 ; "zero flag set correctly")]
  #[test_case(0xF0, 0xF0, 0xE0, 0x10 ; "carry set correctly")]
  #[test_case(0x08, 0x08, 0x10, 0x20 ; "half carry set correctly")]
  fn add_indirect_HL_to_reg_A_and_write_to_reg_A(a: u8, value: u8, result: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, a);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x86);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0x03, 0x00, 0xB0 ; "zero flag set correctly")]
  #[test_case(0xF0, 0xEF, 0xE0, 0x30 ; "carry set correctly")]
  #[test_case(0x08, 0x07, 0x10, 0x20 ; "half carry set correctly")]
  fn add_reg_with_carry_to_reg_A_and_write_to_reg_A(a: u8, value: u8, result: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::F, 0x10);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0x8A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0x03, 0x00, 0xB0 ; "zero flag set correctly")]
  #[test_case(0xF0, 0xEF, 0xE0, 0x30 ; "carry set correctly")]
  #[test_case(0x08, 0x07, 0x10, 0x20 ; "half carry set correctly")]
  fn add_immediate_with_carry_to_reg_A_and_write_to_reg_A(a: u8, value: u8, result: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    memory.write(0x0000, 0xCE);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0x03, 0x00, 0xB0 ; "zero flag set correctly")]
  #[test_case(0xF0, 0xEF, 0xE0, 0x10 ; "carry set correctly")]
  #[test_case(0x08, 0x07, 0x10, 0x20 ; "half carry set correctly")]
  fn add_indirect_HL_with_carry_to_reg_A_and_write_to_reg_A(a: u8, value: u8, result: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x8E);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFC, 0x00, 0xC0 ; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0xE0, 0x50 ; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x0E, 0x60 ; "half carry set correctly")]
  fn subtract_reg_from_reg_A_and_write_to_reg_A(a: u8, value: u8, result: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0x92);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFC, 0x00, 0xC0 ; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0xE0, 0x50 ; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x0E, 0x60 ; "half carry set correctly")]
  fn subtract_immediate_from_reg_A_and_write_to_reg_A(a: u8, value: u8, result: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, a);
    memory.write(0x0000, 0xD6);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFC, 0x00, 0xC0 ; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0xE0, 0x50 ; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x0E, 0x60 ; "half carry set correctly")]
  fn subtract_indirect_HL_from_reg_A_and_write_to_reg_A(a: u8, value: u8, result: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, a);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x96);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFB, 0x00, 0xC0 ; "zero flag set correctly")]
  #[test_case(0x1F, 0x3E, 0xE0, 0x50 ; "carry set correctly")]
  #[test_case(0xF1, 0xE2, 0x0E, 0x60 ; "half carry set correctly")]
  fn subtract_reg_with_carry_from_reg_A_and_write_to_reg_A(a: u8, value: u8, result: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::F, 0x10);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0x9A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFB, 0x00, 0xC0 ; "zero flag set correctly")]
  #[test_case(0x1F, 0x3E, 0xE0, 0x50 ; "carry set correctly")]
  #[test_case(0xF1, 0xE2, 0x0E, 0x60 ; "half carry set correctly")]
  fn subtract_immediate_with_carry_from_reg_A_and_write_to_reg_A(a: u8, value: u8, result: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    memory.write(0x0000, 0xDE);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFB, 0x00, 0xC0 ; "zero flag set correctly")]
  #[test_case(0x1F, 0x3E, 0xE0, 0x50 ; "carry set correctly")]
  #[test_case(0xF1, 0xE2, 0x0E, 0x60 ; "half carry set correctly")]
  fn subtract_indirect_HL_with_carry_from_reg_A_and_write_to_reg_A(a: u8, value: u8, result: u8, f: u8) {
    let mut cpu = CPU::new();
    let mut memory = MockMemory::new();
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x9E);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }
}
