use core::panicking::panic;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ops::{BitAnd, Shr};
use std::rc::Rc;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::cpu::alu::ALU;
use crate::memory::memory::{Memory, MemoryRef};
use crate::time::time::ClockAware;
use crate::util::bit_util::BitUtil;
use super::alu;

#[derive(Copy, Clone)]
struct Opcode {
  opcode: u8,
}

// Opcode bit structure: xxyy yzzz
// Opcode bit structure: xxdd xxxx
// Opcode bit structure: xxxc cxxx
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

  fn cc_bits(&self) -> u8 {
    self.opcode >> 3 & 3
  }

  fn dd_bits(&self) -> u8 {
    self.opcode >> 4 & 3
  }

  fn qq_bits(&self) -> u8 {
    self.opcode >> 4 & 3
  }
}

#[derive(Copy, Clone, Debug)]
pub enum Register {
  A,
  F,
  // Z | N | H | CY | x | x | x | x    Z: 1 if result was 0, N: 1 if previous op was subtraction, H: carry from bit 3, CY: carry from bit 7
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
  S,
  P,
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
      Register::S => 10,
      Register::P => 11,
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

  pub fn get_upper_register(&self) -> Register {
    match self {
      Register::A => Register::A,
      Register::F => Register::F,
      Register::AF => Register::A,
      Register::B => Register::B,
      Register::C => Register::C,
      Register::BC => Register::B,
      Register::D => Register::D,
      Register::E => Register::E,
      Register::DE => Register::D,
      Register::H => Register::H,
      Register::L => Register::L,
      Register::HL => Register::H,
      Register::S => Register::S,
      Register::P => Register::P,
      Register::SP => Register::S,
      _ => panic!("Can't be accessed as separate registers"),
    }
  }

  pub fn get_lower_register(&self) -> Register {
    match self {
      Register::A => Register::A,
      Register::F => Register::F,
      Register::AF => Register::F,
      Register::B => Register::B,
      Register::C => Register::C,
      Register::BC => Register::C,
      Register::D => Register::D,
      Register::E => Register::E,
      Register::DE => Register::E,
      Register::H => Register::H,
      Register::L => Register::L,
      Register::HL => Register::L,
      Register::S => Register::S,
      Register::P => Register::P,
      Register::SP => Register::P,
      _ => panic!("Can't be accessed as separate registers"),
    }
  }
}

#[derive(Copy, Clone)]
enum ByteLocation {
  Value(u8),
  Register(Register),
  ByteBuffer,
  LowerAddressBuffer,
  UpperAddressBuffer,
  LowerWordBuffer,
  UpperWordBuffer,
  NextMemoryByte,
  MemoryReferencedByAddressBuffer,
  MemoryReferencedByRegister(Register),
}

#[derive(Copy, Clone)]
enum WordLocation {
  Value(u16),
  Register(Register),
  WordBuffer,
  AddressBuffer,
}

#[derive(Copy, Clone, Default)]
struct ByteArithmeticParams {
  first: ByteLocation,
  second: ByteLocation,
  destination: ByteLocation,
  use_carry: bool,
  flag_mask: u8,
}

#[derive(Copy, Clone, Default)]
struct WordArithmeticParams {
  first: WordLocation,
  second: WordLocation,
  destination: WordLocation,
  flag_mask: u8,
}

#[derive(Copy, Clone, Default)]
struct ByteLogicParams {
  first: ByteLocation,
  second: ByteLocation,
  destination: ByteLocation,
}

#[derive(Copy, Clone, Default)]
struct ByteRotationParams {
  source: ByteLocation,
  destination: ByteLocation,
  unset_zero: bool,
}

impl Default for ByteArithmeticParams {
  fn default() -> Self {
    ByteArithmeticParams {
      first: ByteLocation::Value(0),
      second: ByteLocation::Value(0),
      destination: ByteLocation::ByteBuffer,
      use_carry: false,
      flag_mask: 0,
    }
  }
}

struct InstructionContext {
  opcode: Opcode,
  byte_buffer: u8,
  word_buffer: u16,
  address_buffer: u16,
}

impl InstructionContext {
  pub fn new() -> InstructionContext {
    InstructionContext {
      opcode: Opcode::new(0),
      byte_buffer: 0u8,
      word_buffer: 0u16,
      address_buffer: 0u16,
    }
  }
}

type Operation = Box<dyn FnOnce()>;

pub struct CPU {
  context: InstructionContext,
  operations: VecDeque<Operation>,
  memory: MemoryRef,
  registers: [u8; 12],
  ime: bool,
}

impl CPU {
  pub fn new(memory: MemoryRef) -> CPU {
    CPU {
      memory,
      context: InstructionContext::new(),
      operations: VecDeque::with_capacity(5),
      registers: [0; 12],
      ime: true,
    }
  }

  fn check_interrupt(&mut self) {
    if !self.ime {
      return;
    }
    let memory = self.memory.borrow();
    let interrupt_enables = memory.read(0xFFFF);
    let interrupt_flags = memory.read(0xFF0F);
    let interrupts_to_process = interrupt_enables & interrupt_flags;
  }

  fn fetch_and_decode_instruction(&mut self) {
    macro_rules! add_operations {
        ($($x:expr),*) => {
          {
            $(
              self.operations.push_back($x);
            )*
          }
        };
    }




    self.context.opcode = Opcode::new(self.read_next_instruction());
    match opcode.value() {
      0x00 => {}
      0x01 => self.immediate_to_reg_pair_ld(),
      0x02 => self.reg_a_to_indirect_bc_ld(),
      0x03 => self.increment_reg_pair(),
      0x04 => self.increment_reg(),
      0x05 => self.decrement_reg(),
      0x06 => self.immediate_to_reg_ld(),
      0x07 => self.rotate_reg_a_left(),
      0x08 => self.reg_sp_to_immediate_indirect_ld(),
      0x09 => self.add_reg_pair_to_reg_hl(),
      0x0A => self.indirect_bc_to_reg_a_ld(),
      0x0B => self.decrement_reg_pair(),
      0x0C => self.increment_reg(),
      0x0D => self.decrement_reg(),
      0x0E => self.immediate_to_reg_ld(),
      0x0F => self.rotate_reg_a_right(),
      0x10 => self.stop(),
      0x11 => self.immediate_to_reg_pair_ld(),
      0x12 => self.reg_a_to_indirect_de_ld(),
      0x13 => self.increment_reg_pair(),
      0x14 => self.increment_reg(),
      0x15 => self.decrement_reg(),
      0x16 => self.immediate_to_reg_ld(),
      0x17 => self.rotate_reg_a_left_through_carry(),
      0x18 => self.jump_relative(),
      0x19 => self.add_reg_pair_to_reg_hl(),
      0x1A => self.indirect_de_to_reg_a_ld(),
      0x1B => self.decrement_reg_pair(),
      0x1C => self.increment_reg(),
      0x1D => self.decrement_reg(),
      0x1E => self.immediate_to_reg_ld(),
      0x1F => self.rotate_reg_a_right_through_carry(),
      0x20 => self.jump_conditional_relative(),
      0x21 => self.immediate_to_reg_pair_ld(),
      0x22 => self.reg_a_to_indirect_hl_ld_and_increment(),
      0x23 => self.increment_reg_pair(),
      0x24 => self.increment_reg(),
      0x25 => self.decrement_reg(),
      0x26 => self.immediate_to_reg_ld(),
      0x27 => CPU::decimal_adjust_reg_a,
      0x28 => CPU::jump_conditional_relative,
      0x29 => self.add_reg_pair_to_reg_hl(),
      0x2A => CPU::indirect_hl_to_reg_a_ld_and_increment,
      0x2B => self.decrement_reg_pair(),
      0x2C => self.increment_reg(),
      0x2D => self.decrement_reg(),
      0x2E => self.immediate_to_reg_ld(),
      0x2F => CPU::ones_complement_reg_a,
      0x30 => CPU::jump_conditional_relative,
      0x31 => self.immediate_to_reg_pair_ld(),
      0x32 => CPU::reg_a_to_indirect_hl_ld_and_decrement,
      0x33 => self.increment_reg_pair(),
      0x34 => CPU::increment_indirect_hl,
      0x35 => CPU::decrement_indirect_hl,
      0x36 => CPU::immediate_to_indirect_ld,
      0x37 => CPU::set_carry_flag,
      0x38 => CPU::jump_conditional_relative,
      0x39 => self.add_reg_pair_to_reg_hl(),
      0x3A => CPU::indirect_hl_to_reg_a_ld_and_decrement,
      0x3B => self.decrement_reg_pair(),
      0x3C => self.increment_reg(),
      0x3D => self.decrement_reg(),
      0x3E => self.immediate_to_reg_ld(),
      0x3F => CPU::flip_carry_flag,
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
      0x76 => CPU::halt,
      0x77 => CPU::reg_to_indirect_ld,
      0x78..=0x7D => CPU::reg_to_reg_ld,
      0x7E => CPU::indirect_to_reg_ld,
      0x7F => CPU::reg_to_reg_ld,
      0x80..=0x85 => self.add_reg_to_reg_a_and_write_to_reg_a(false),
      0x86 => self.add_indirect_hl_to_reg_a_and_write_to_reg_a(false),
      0x87 => self.add_reg_to_reg_a_and_write_to_reg_a(false),
      0x88..=0x8D => self.add_reg_to_reg_a_and_write_to_reg_a(true),
      0x8E => self.add_indirect_hl_to_reg_a_and_write_to_reg_a(true),
      0x8F => self.add_reg_to_reg_a_and_write_to_reg_a(true),
      0x90..=0x95 => self.subtract_reg_from_reg_a_and_write_to_reg_a(false),
      0x96 => self.subtract_indirect_hl_from_reg_a_and_write_to_reg_a(false),
      0x97 => self.subtract_reg_from_reg_a_and_write_to_reg_a(false),
      0x98..=0x9D => self.subtract_reg_with_carry_from_reg_a_and_write_to_reg_a(true),
      0x9E => self.subtract_indirect_hl_with_carry_from_reg_a_and_write_to_reg_a(true),
      0x9F => self.subtract_reg_with_carry_from_reg_a_and_write_to_reg_a(true),
      0xA0..=0xA5 => CPU::and_reg_with_reg_a_and_write_to_reg_a,
      0xA6 => CPU::and_indirect_hl_with_reg_a_and_write_to_reg_a,
      0xA7 => CPU::and_reg_with_reg_a_and_write_to_reg_a,
      0xA8..=0xAD => CPU::xor_reg_with_reg_a_and_write_to_reg_a,
      0xAE => CPU::xor_indirect_hl_with_reg_a_and_write_to_reg_a,
      0xAF => CPU::xor_reg_with_reg_a_and_write_to_reg_a,
      0xB0..=0xB5 => CPU::or_reg_with_reg_a_and_write_to_reg_a,
      0xB6 => CPU::or_indirect_hl_with_reg_a_and_write_to_reg_a,
      0xB7 => CPU::or_reg_with_reg_a_and_write_to_reg_a,
      0xB8..=0xBD => CPU::compare_reg_with_reg_a,
      0xBE => CPU::compare_indirect_hl_with_reg_a,
      0xBF => CPU::compare_reg_with_reg_a,
      0xC0 => CPU::return_conditionally,
      0xC1 => CPU::pop_stack_to_reg_pair,
      0xC2 => CPU::jump_conditional,
      0xC3 => CPU::jump,
      0xC4 => CPU::call_conditional,
      0xC5 => CPU::push_reg_pair_to_stack,
      0xC7 => CPU::restart,
      0xC6 => CPU::add_immediate_to_reg_a_and_write_to_reg_a,
      0xC8 => CPU::return_conditionally,
      0xC9 => CPU::return_from_call,
      0xCA => CPU::jump_conditional,
      0xCB => CPU::execute_cb,
      0xCC => CPU::call_conditional,
      0xCD => CPU::call,
      0xCE => CPU::add_immediate_with_carry_to_reg_a_and_write_to_reg_a,
      0xCF => CPU::restart,
      0xD0 => CPU::return_conditionally,
      0xD1 => CPU::pop_stack_to_reg_pair,
      0xD2 => CPU::jump_conditional,
      0xD4 => CPU::call_conditional,
      0xD5 => CPU::push_reg_pair_to_stack,
      0xD6 => CPU::subtract_immediate_from_reg_a_and_write_to_reg_a,
      0xD7 => CPU::restart,
      0xD8 => CPU::return_conditionally,
      0xD9 => CPU::return_from_interrupt,
      0xDA => CPU::jump_conditional,
      0xDC => CPU::call_conditional,
      0xDE => CPU::subtract_immediate_with_carry_from_reg_a_and_write_to_reg_a,
      0xDF => CPU::restart,
      0xE0 => CPU::reg_a_to_immediate_indirect_with_offset_ld,
      0xE1 => CPU::pop_stack_to_reg_pair,
      0xE2 => CPU::reg_a_to_indirect_c_ld,
      0xE5 => CPU::push_reg_pair_to_stack,
      0xE6 => CPU::and_immediate_with_reg_a_and_write_to_reg_a,
      0xE7 => CPU::restart,
      0xE8 => CPU::add_immediate_to_reg_sp,
      0xE9 => CPU::jump_to_indirect_hl,
      0xEA => CPU::reg_a_to_immediate_indirect_ld,
      0xEE => CPU::xor_immediate_with_reg_a_and_write_to_reg_a,
      0xEF => CPU::restart,
      0xF0 => CPU::immediate_indirect_with_offset_to_reg_a_ld,
      0xF1 => CPU::pop_stack_to_reg_pair,
      0xF2 => CPU::indirect_c_with_offset_to_reg_a_ld,
      0xF3 => CPU::disable_interrupts,
      0xF5 => CPU::push_reg_pair_to_stack,
      0xF6 => CPU::or_immediate_with_reg_a_and_write_to_reg_a,
      0xF7 => CPU::restart,
      0xF8 => CPU::reg_sp_plus_signed_immediate_to_hl_ld,
      0xF9 => CPU::reg_hl_to_reg_sp_ld,
      0xFA => CPU::immediate_indirect_to_reg_a_ld,
      0xFB => CPU::enable_interrupts,
      0xFE => CPU::compare_immediate_with_reg_a,
      0xFF => CPU::restart,
      _ => panic!("Unknown opcode"),
    };
  }

  fn execute_next_instruction(&mut self) {
    let operation = self.operations.pop_front().unwrap_or(CPU::fetch_and_decode_instruction);
    operation(self)


    let opcode = Opcode::new(self.read_next_instruction());
    match opcode.value() {
      0x00 => CPU::noop,
      0x01 => self.immediate_to_reg_pair_ld(),
      0x02 => CPU::reg_a_to_indirect_bc_ld,
      0x03 => self.increment_reg_pair(),
      0x04 => self.increment_reg(),
      0x05 => self.decrement_reg(),
      0x06 => self.immediate_to_reg_ld(),
      0x07 => CPU::rotate_reg_a_left,
      0x08 => CPU::reg_sp_to_immediate_indirect_ld,
      0x09 => self.add_reg_pair_to_reg_hl(),
      0x0A => CPU::indirect_bc_to_reg_a_ld,
      0x0B => self.decrement_reg_pair(),
      0x0C => self.increment_reg(),
      0x0D => self.decrement_reg(),
      0x0E => self.immediate_to_reg_ld(),
      0x0F => CPU::rotate_reg_a_right,
      0x10 => CPU::stop,
      0x11 => self.immediate_to_reg_pair_ld(),
      0x12 => CPU::reg_a_to_indirect_de_ld,
      0x13 => self.increment_reg_pair(),
      0x14 => self.increment_reg(),
      0x15 => self.decrement_reg(),
      0x16 => self.immediate_to_reg_ld(),
      0x17 => CPU::rotate_reg_a_left_through_carry,
      0x18 => CPU::jump_relative,
      0x19 => self.add_reg_pair_to_reg_hl(),
      0x1A => CPU::indirect_de_to_reg_a_ld,
      0x1B => self.decrement_reg_pair(),
      0x1C => self.increment_reg(),
      0x1D => self.decrement_reg(),
      0x1E => self.immediate_to_reg_ld(),
      0x1F => CPU::rotate_reg_a_right_through_carry,
      0x20 => CPU::jump_conditional_relative,
      0x21 => self.immediate_to_reg_pair_ld(),
      0x22 => CPU::reg_a_to_indirect_hl_ld_and_increment,
      0x23 => self.increment_reg_pair(),
      0x24 => self.increment_reg(),
      0x25 => self.decrement_reg(),
      0x26 => self.immediate_to_reg_ld(),
      0x27 => CPU::decimal_adjust_reg_a,
      0x28 => CPU::jump_conditional_relative,
      0x29 => self.add_reg_pair_to_reg_hl(),
      0x2A => CPU::indirect_hl_to_reg_a_ld_and_increment,
      0x2B => self.decrement_reg_pair(),
      0x2C => self.increment_reg(),
      0x2D => self.decrement_reg(),
      0x2E => self.immediate_to_reg_ld(),
      0x2F => CPU::ones_complement_reg_a,
      0x30 => CPU::jump_conditional_relative,
      0x31 => self.immediate_to_reg_pair_ld(),
      0x32 => CPU::reg_a_to_indirect_hl_ld_and_decrement,
      0x33 => self.increment_reg_pair(),
      0x34 => CPU::increment_indirect_hl,
      0x35 => CPU::decrement_indirect_hl,
      0x36 => CPU::immediate_to_indirect_ld,
      0x37 => CPU::set_carry_flag,
      0x38 => CPU::jump_conditional_relative,
      0x39 => self.add_reg_pair_to_reg_hl(),
      0x3A => CPU::indirect_hl_to_reg_a_ld_and_decrement,
      0x3B => self.decrement_reg_pair(),
      0x3C => self.increment_reg(),
      0x3D => self.decrement_reg(),
      0x3E => self.immediate_to_reg_ld(),
      0x3F => CPU::flip_carry_flag,
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
      0x76 => CPU::halt,
      0x77 => CPU::reg_to_indirect_ld,
      0x78..=0x7D => CPU::reg_to_reg_ld,
      0x7E => CPU::indirect_to_reg_ld,
      0x7F => CPU::reg_to_reg_ld,
      0x80..=0x85 => CPU::add_reg_to_reg_a_and_write_to_reg_a,
      0x86 => CPU::add_indirect_hl_to_reg_a_and_write_to_reg_a,
      0x87 => CPU::add_reg_to_reg_a_and_write_to_reg_a,
      0x88..=0x8D => CPU::add_reg_with_carry_to_reg_a_and_write_to_reg_a,
      0x8E => CPU::add_indirect_hl_with_carry_to_reg_a_and_write_to_reg_a,
      0x8F => CPU::add_reg_with_carry_to_reg_a_and_write_to_reg_a,
      0x90..=0x95 => CPU::subtract_reg_from_reg_a_and_write_to_reg_a,
      0x96 => CPU::subtract_indirect_hl_from_reg_a_and_write_to_reg_a,
      0x97 => CPU::subtract_reg_from_reg_a_and_write_to_reg_a,
      0x98..=0x9D => CPU::subtract_reg_with_carry_from_reg_a_and_write_to_reg_a,
      0x9E => CPU::subtract_indirect_hl_with_carry_from_reg_a_and_write_to_reg_a,
      0x9F => CPU::subtract_reg_with_carry_from_reg_a_and_write_to_reg_a,
      0xA0..=0xA5 => CPU::and_reg_with_reg_a_and_write_to_reg_a,
      0xA6 => CPU::and_indirect_hl_with_reg_a_and_write_to_reg_a,
      0xA7 => CPU::and_reg_with_reg_a_and_write_to_reg_a,
      0xA8..=0xAD => CPU::xor_reg_with_reg_a_and_write_to_reg_a,
      0xAE => CPU::xor_indirect_hl_with_reg_a_and_write_to_reg_a,
      0xAF => CPU::xor_reg_with_reg_a_and_write_to_reg_a,
      0xB0..=0xB5 => CPU::or_reg_with_reg_a_and_write_to_reg_a,
      0xB6 => CPU::or_indirect_hl_with_reg_a_and_write_to_reg_a,
      0xB7 => CPU::or_reg_with_reg_a_and_write_to_reg_a,
      0xB8..=0xBD => CPU::compare_reg_with_reg_a,
      0xBE => CPU::compare_indirect_hl_with_reg_a,
      0xBF => CPU::compare_reg_with_reg_a,
      0xC0 => CPU::return_conditionally,
      0xC1 => CPU::pop_stack_to_reg_pair,
      0xC2 => CPU::jump_conditional,
      0xC3 => CPU::jump,
      0xC4 => CPU::call_conditional,
      0xC5 => CPU::push_reg_pair_to_stack,
      0xC7 => CPU::restart,
      0xC6 => CPU::add_immediate_to_reg_a_and_write_to_reg_a,
      0xC8 => CPU::return_conditionally,
      0xC9 => CPU::return_from_call,
      0xCA => CPU::jump_conditional,
      0xCB => CPU::execute_cb,
      0xCC => CPU::call_conditional,
      0xCD => CPU::call,
      0xCE => CPU::add_immediate_with_carry_to_reg_a_and_write_to_reg_a,
      0xCF => CPU::restart,
      0xD0 => CPU::return_conditionally,
      0xD1 => CPU::pop_stack_to_reg_pair,
      0xD2 => CPU::jump_conditional,
      0xD4 => CPU::call_conditional,
      0xD5 => CPU::push_reg_pair_to_stack,
      0xD6 => CPU::subtract_immediate_from_reg_a_and_write_to_reg_a,
      0xD7 => CPU::restart,
      0xD8 => CPU::return_conditionally,
      0xD9 => CPU::return_from_interrupt,
      0xDA => CPU::jump_conditional,
      0xDC => CPU::call_conditional,
      0xDE => CPU::subtract_immediate_with_carry_from_reg_a_and_write_to_reg_a,
      0xDF => CPU::restart,
      0xE0 => CPU::reg_a_to_immediate_indirect_with_offset_ld,
      0xE1 => CPU::pop_stack_to_reg_pair,
      0xE2 => CPU::reg_a_to_indirect_c_ld,
      0xE5 => CPU::push_reg_pair_to_stack,
      0xE6 => CPU::and_immediate_with_reg_a_and_write_to_reg_a,
      0xE7 => CPU::restart,
      0xE8 => CPU::add_immediate_to_reg_sp,
      0xE9 => CPU::jump_to_indirect_hl,
      0xEA => CPU::reg_a_to_immediate_indirect_ld,
      0xEE => CPU::xor_immediate_with_reg_a_and_write_to_reg_a,
      0xEF => CPU::restart,
      0xF0 => CPU::immediate_indirect_with_offset_to_reg_a_ld,
      0xF1 => CPU::pop_stack_to_reg_pair,
      0xF2 => CPU::indirect_c_with_offset_to_reg_a_ld,
      0xF3 => CPU::disable_interrupts,
      0xF5 => CPU::push_reg_pair_to_stack,
      0xF6 => CPU::or_immediate_with_reg_a_and_write_to_reg_a,
      0xF7 => CPU::restart,
      0xF8 => CPU::reg_sp_plus_signed_immediate_to_hl_ld,
      0xF9 => CPU::reg_hl_to_reg_sp_ld,
      0xFA => CPU::immediate_indirect_to_reg_a_ld,
      0xFB => CPU::enable_interrupts,
      0xFE => CPU::compare_immediate_with_reg_a,
      0xFF => CPU::restart,
      _ => panic!("Unknown opcode"),
    };
    operation(self, opcode);
  }

  fn write_low_byte_buffer_to_register(&mut self) {
    self.registers[self.context.register.offset()] = self.context.low_byte_buffer;
  }

  fn write_high_byte_buffer_to_register(&mut self) {
    self.registers[self.context.register.offset()] = self.context.high_byte_buffer;
  }

  fn write_register_to_low_byte_buffer(&mut self) {
    self.context.low_byte_buffer = self.registers[self.context.register.offset()];
  }

  fn write_register_to_high_byte_buffer(&mut self) {
    self.context.high_byte_buffer = self.registers[self.context.register.offset()];
  }

  fn write_byte_buffers_to_register_pair(&mut self) {
    let offset = self.context.register.offset();
    self.registers[offset] = self.context.high_byte_buffer;
    self.registers[offset + 1] = self.context.low_byte_buffer;
  }

  fn write_word_buffer_to_register_pair(&mut self) {
    (&mut self.registers[register.offset()..]).write_u16::<BigEndian>(self.context.word_buffer).unwrap();
  }

  fn read_into_low_byte_buffer_from_memory_at_address(&mut self) {
    self.context.low_byte_buffer = self.memory.borrow().read(self.context.address_buffer);
  }

  fn read_into_high_byte_buffer_from_memory_at_address(&mut self) {
    self.context.high_byte_buffer = self.memory.borrow().read(self.context.address_buffer);
  }

  fn read_into_low_byte_buffer_from_next_memory_byte(&mut self) {
    self.context.low_byte_buffer = self.read_next_byte();
  }

  fn read_into_high_byte_buffer_from_next_memory_byte(&mut self) {
    self.context.high_byte_buffer = self.read_next_byte();
  }


  fn write_low_byte_buffer_to_memory_at_address(&mut self) {
    self.memory.borrow_mut().write(self.context.address_buffer, self.context.low_byte_buffer);
  }

  fn write_high_byte_buffer_to_memory_at_address(&mut self) {
    self.memory.borrow_mut().write(self.context.address_buffer, self.context.high_byte_buffer);
  }

  fn write_low_byte_buffer_to_next_memory_byte(&mut self) {
    self.memory.borrow_mut().write(self.read_and_increment_register_pair(Register::PC), self.context.low_byte_buffer);
  }

  fn write_high_byte_buffer_to_next_memory_byte(&mut self) {
    self.memory.borrow_mut().write(self.read_and_increment_register_pair(Register::PC), self.context.high_byte_buffer);
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

  fn decrement_and_read_register_pair(&mut self, register: Register) -> u16 {
    let value = self.read_register_pair(register) - 1;
    self.write_register_pair(register, value);
    value
  }

  fn read_next_instruction(&mut self) -> u8 {
    self.memory.borrow().read(self.read_and_increment_register_pair(Register::PC))
  }

  fn read_next_byte(&mut self) -> u8 {
    self.memory.borrow().read(self.read_and_increment_register_pair(Register::PC))
  }

  fn read_low_from_memory(&mut self) {
    self.context.low = self.read_next_byte();
  }

  fn read_high_from_memory(&mut self) {
    self.context.low = self.read_next_byte();
  }

  fn write_memory(&mut self, address: u16, value: u8) {
    self.memory.borrow_mut().write(address, value);
  }

  fn read_register(&self, register: Register) -> u8 {
    self.registers[register.offset()]
  }

  fn read_register_pair(&self, register: Register) -> u16 {
    (&self.registers[register.offset()..]).read_u16::<BigEndian>().unwrap()
  }

  fn write_register(&mut self, register: Register, value: u8) {
    self.registers[register.offset()] = value;
  }

  fn write_register_masked(&mut self, register: Register, value: u8, mask: u8) {
    self.registers[register.offset()] = (!mask & self.registers[register.offset()]) | (mask & value);
  }

  fn write_register_pair(&mut self, register: Register, value: u16) {
    (&mut self.registers[register.offset()..]).write_u16::<BigEndian>(value).unwrap();
  }

  fn execute_cb(&mut self, _opcode: Opcode) {
    let opcode = Opcode::new(self.read_next_instruction());
    let operation = match opcode.value() {
      0x00..=0x05 => CPU::rotate_reg_left,
      0x06 => CPU::rotate_indirect_hl_left,
      0x07 => CPU::rotate_reg_left,
      0x08..=0x0D => CPU::rotate_reg_right,
      0x0E => CPU::rotate_indirect_hl_right,
      0x0F => CPU::rotate_reg_right,
      0x10..=0x15 => CPU::rotate_reg_left_through_carry,
      0x16 => CPU::rotate_indirect_hl_left_through_carry,
      0x17 => CPU::rotate_reg_left_through_carry,
      0x18..=0x1D => CPU::rotate_reg_right_through_carry,
      0x1E => CPU::rotate_indirect_hl_right_through_carry,
      0x1F => CPU::rotate_reg_right_through_carry,
      0x20..=0x25 => CPU::shift_reg_left,
      0x26 => CPU::shift_indirect_hl_left,
      0x27 => CPU::shift_reg_left,
      0x28..=0x2D => CPU::shift_reg_right_arithmetic,
      0x2E => CPU::shift_indirect_hl_right_arithmetic,
      0x2F => CPU::shift_reg_right_arithmetic,
      0x30..=0x35 => CPU::swap_reg,
      0x36 => CPU::swap_indirect_hl,
      0x37 => CPU::swap_reg,
      0x38..=0x3D => CPU::shift_reg_right,
      0x3E => CPU::shift_indirect_hl_right,
      0x3F => CPU::shift_reg_right,
      0x40..=0x45 => CPU::get_reg_bit,
      0x46 => CPU::get_indirect_hl_bit,
      0x47..=0x4D => CPU::get_reg_bit,
      0x4E => CPU::get_indirect_hl_bit,
      0x4F..=0x55 => CPU::get_reg_bit,
      0x56 => CPU::get_indirect_hl_bit,
      0x57..=0x5D => CPU::get_reg_bit,
      0x5E => CPU::get_indirect_hl_bit,
      0x5F..=0x65 => CPU::get_reg_bit,
      0x66 => CPU::get_indirect_hl_bit,
      0x67..=0x6D => CPU::get_reg_bit,
      0x6E => CPU::get_indirect_hl_bit,
      0x6F..=0x75 => CPU::get_reg_bit,
      0x76 => CPU::get_indirect_hl_bit,
      0x77..=0x7D => CPU::get_reg_bit,
      0x7E => CPU::get_indirect_hl_bit,
      0x7F => CPU::get_reg_bit,
      0x80..=0x85 => CPU::reset_reg_bit,
      0x86 => CPU::reset_indirect_hl_bit,
      0x87..=0x8D => CPU::reset_reg_bit,
      0x8E => CPU::reset_indirect_hl_bit,
      0x8F..=0x95 => CPU::reset_reg_bit,
      0x96 => CPU::reset_indirect_hl_bit,
      0x97..=0x9D => CPU::reset_reg_bit,
      0x9E => CPU::reset_indirect_hl_bit,
      0x9F..=0xA5 => CPU::reset_reg_bit,
      0xA6 => CPU::reset_indirect_hl_bit,
      0xA7..=0xAD => CPU::reset_reg_bit,
      0xAE => CPU::reset_indirect_hl_bit,
      0xAF..=0xB5 => CPU::reset_reg_bit,
      0xB6 => CPU::reset_indirect_hl_bit,
      0xB7..=0xBD => CPU::reset_reg_bit,
      0xBE => CPU::reset_indirect_hl_bit,
      0xBF => CPU::reset_reg_bit,
      0xC0..=0xC5 => CPU::set_reg_bit,
      0xC6 => CPU::set_indirect_hl_bit,
      0xC7..=0xCD => CPU::set_reg_bit,
      0xCE => CPU::set_indirect_hl_bit,
      0xCF..=0xD5 => CPU::set_reg_bit,
      0xD6 => CPU::set_indirect_hl_bit,
      0xD7..=0xDD => CPU::set_reg_bit,
      0xDE => CPU::set_indirect_hl_bit,
      0xDF..=0xE5 => CPU::set_reg_bit,
      0xE6 => CPU::set_indirect_hl_bit,
      0xE7..=0xED => CPU::set_reg_bit,
      0xEE => CPU::set_indirect_hl_bit,
      0xEF..=0xF5 => CPU::set_reg_bit,
      0xF6 => CPU::set_indirect_hl_bit,
      0xF7..=0xFD => CPU::set_reg_bit,
      0xFE => CPU::set_indirect_hl_bit,
      0xFF => CPU::set_reg_bit,
      _ => panic!("Unknown opcode"),
    };
    operation(self, opcode);
  }

  fn combine_operations(operation1: Operation, operation2: Operation) -> Operation {
    Box::new(|| {
      operation1();
      operation2();
    })
  }

  fn read_byte(&mut self, location: ByteLocation) -> u8 {
    match location {
      ByteLocation::Value(value) => value,
      ByteLocation::Register(register) => self.read_register(register),
      ByteLocation::ByteBuffer => self.context.byte_buffer,
      ByteLocation::LowerAddressBuffer => self.context.address_buffer as u8,
      ByteLocation::UpperAddressBuffer => (self.context.address_buffer >> 8) as u8,
      ByteLocation::LowerWordBuffer => self.context.word_buffer as u8,
      ByteLocation::UpperWordBuffer => (self.context.word_buffer >> 8) as u8,
      ByteLocation::MemoryReferencedByAddressBuffer => self.memory.borrow().read(self.context.address_buffer),
      ByteLocation::MemoryReferencedByRegister(register) => self.memory.borrow().read(self.read_register_pair(register)),
      ByteLocation::NextMemoryByte => self.read_next_byte(),
    }
  }

  fn write_byte(&mut self, location: ByteLocation, value: u8) {
    match location {
      ByteLocation::Register(register) => self.write_register(register, value),
      ByteLocation::ByteBuffer => self.context.byte_buffer = value,
      ByteLocation::LowerAddressBuffer => self.context.address_buffer = (self.context.address_buffer & 0xFF00) + (value as u16),
      ByteLocation::UpperAddressBuffer => self.context.address_buffer = (self.context.address_buffer & 0x00FF) + ((value as u16) << 8),
      ByteLocation::LowerWordBuffer => self.context.word_buffer = (self.context.word_buffer & 0xFF00) + (value as u16),
      ByteLocation::UpperWordBuffer => self.context.word_buffer = (self.context.word_buffer & 0x00FF) + ((value as u16) << 8),
      ByteLocation::MemoryReferencedByAddressBuffer => self.memory.borrow_mut().write(self.context.address_buffer, value),
      ByteLocation::MemoryReferencedByRegister(register) => self.memory.borrow_mut().write(self.read_register_pair(register), value),
      ByteLocation::NextMemoryByte => panic!("Can't write byte to next memory location"),
      ByteLocation::Value(_) => panic!("Can't write to passed value")
    }
  }

  fn read_word(&mut self, location: WordLocation) -> u16 {
    match location {
      WordLocation::Value(value) => value,
      WordLocation::Register(register) => self.read_register_pair(register),
      WordLocation::WordBuffer => self.context.word_buffer,
      WordLocation::AddressBuffer => self.context.address_buffer,
    }
  }

  fn write_word(&mut self, location: WordLocation, value: u16) {
    match location {
      WordLocation::Register(register) => self.write_register_pair(register, value),
      WordLocation::WordBuffer => self.context.word_buffer = value,
      WordLocation::AddressBuffer => self.context.address_buffer = value,
      WordLocation::Value(_) => panic!("Can't write to passed value")
    }
  }

  fn move_byte(&mut self, source: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(|| {
      self.write_byte(destination, self.read_byte(source));
    })
  }

  fn move_word(&mut self, source: WordLocation, destination: WordLocation) -> Operation {
    Box::new(|| {
      self.write_word(destination, self.read_word(source));
    })
  }

  fn add_bytes(&mut self, params: ByteArithmeticParams) -> Operation {
    Box::new(|| {
      let first_value = self.read_byte(params.first);
      let second_value = self.read_byte(params.second);
      let carry = if params.use_carry { self.read_register(Register::F).get_bit(4) as u16 } else { 0u16 };
      let result = (first_value as u16) + (second_value as u16) + carry;
      let carry_result = (operand1 as u16) ^ (operand2 as u16) ^ result;
      let truncated_result = result as u8;
      let zero = truncated_result == 0;
      if params.flag_mask != 0 {
        let flag =
          ((zero as u8) << 7) &
            ((carry_result.get_bit(4) as u8) << 5) &
            ((carry_result.get_bit(8) as u8) << 4);
        self.write_register_masked(Register::F, flag, params.flag_mask);
      }
      self.write_byte(params.destination, truncated_result);
    })
  }

  fn add_words(&mut self, params: WordArithmeticParams) -> Operation {
    Box::new(|| {
      let first_value = self.read_word(params.first);
      let second_value = self.read_word(params.second);
      let le_bytes1 = first_value.to_le_bytes();
      let le_bytes2 = second_value.to_le_bytes();
      let (result1, carry1) = le_bytes1[0].overflowing_add(le_bytes2[0]);
      let result2 = (le_bytes1[1] as u16) + (le_bytes2[1] as u16) + (carry1 as u16);
      let carry_result2 = (operand1 as u16) ^ (operand2 as u16) ^ result;
      let result = (&[result1, result2 as u8][..]).read_u16::<LittleEndian>().unwrap();
      let zero = result == 0;
      if params.flag_mask != 0 {
        let flag =
          ((zero as u8) << 7) &
            ((carry_result2.get_bit(4) as u8) << 5) &
            ((carry_result2.get_bit(8) as u8) << 4);
        self.write_register_masked(Register::F, flag, params.flag_mask);
      }
      self.write_word(params.destination, result);
    })
  }

  fn subtract_bytes(&mut self, params: ByteArithmeticParams) -> Operation {
    Box::new(|| {
      let first_value = self.read_byte(params.first);
      let second_value = self.read_byte(params.second);
      let borrow = if params.use_carry { self.read_register(Register::F).get_bit(4) as u16 } else { 0u16 };
      let result = 0x100u16 + (first_value as u16) - (second_value as u16) - borrow;
      let borrow_result = (0x100u16 + first_value as u16) ^ (second_value as u16) ^ result;
      let truncated_result = result as u8;
      let zero = truncated_result == 0;
      if params.flag_mask != 0 {
        let flag =
          ((zero as u8) << 7) &
            (1u8 << 6) &
            ((borrow_result.get_bit(4) as u8) << 5) &
            ((borrow_result.get_bit(8) as u8) << 4);
        self.write_register_masked(Register::F, flag, params.flag_mask);
      }
      self.write_byte(params.destination, truncated_result);
    })
  }

  fn and_bytes(&mut self, params: ByteLogicParams) -> Operation {
    Box::new(|| {
      let first_value = self.read_byte(params.first);
      let second_value = self.read_byte(params.second);
      let result = first_value & second_value;
      let zero = result == 0;
      let flag = ((zero as u8) << 7) & (1u8 << 5);
      self.write_register(Register::F, flag);
      self.write_byte(params.destination, result);
    })
  }

  fn or_bytes(&mut self, params: ByteLogicParams) -> Operation {
    Box::new(|| {
      let first_value = self.read_byte(params.first);
      let second_value = self.read_byte(params.second);
      let result = first_value | second_value;
      let flag = if result == 0 { 0x80u8 } else { 0x00u8 };
      self.write_register(Register::F, flag);
      self.write_byte(params.destination, result);
    })
  }

  fn xor_bytes(&mut self, params: ByteLogicParams) -> Operation {
    Box::new(|| {
      let first_value = self.read_byte(params.first);
      let second_value = self.read_byte(params.second);
      let result = first_value ^ second_value;
      let flag = if result == 0 { 0x80u8 } else { 0x00u8 };
      self.write_register(Register::F, flag);
      self.write_byte(params.destination, result);
    })
  }

  fn rotate_byte_left(&mut self, params: ByteRotationParams) -> Operation {
    Box::new(|| {
      let value = self.read_byte(source);
      let result = value.rotate_left(1);
      let zero = !params.unset_zero && result == 0;
      let flag =
        ((zero as u8) << 7) & ((value.get_bit(7) as u8) << 4);
      self.write_register(Register::F, flag);
      self.write_byte(params.destination, result);
    })
  }

  fn rotate_byte_left_through_carry(&mut self, params: ByteRotationParams) -> Operation {
    Box::new(|| {
      let value = self.read_byte(source);
      let carry = self.read_register(Register::F).get_bit(4);
      let result = (value << 1) | (carry as u8);
      let zero = !params.unset_zero && result == 0;
      let flag =
        ((zero as u8) << 7) & ((value.get_bit(7) as u8) << 4);
      self.write_register(Register::F, flag);
      self.write_byte(params.destination, result);
    })
  }

  fn rotate_byte_right(&mut self, params: ByteRotationParams) -> Operation {
    Box::new(|| {
      let value = self.read_byte(source);
      let result = value.rotate_right(1);
      let zero = !params.unset_zero && result == 0;
      let flag =
        ((zero as u8) << 7) & ((value.get_bit(0) as u8) << 4);
      self.write_register(Register::F, flag);
      self.write_byte(params.destination, result);
    })
  }

  fn rotate_byte_right_through_carry(&mut self, params: ByteRotationParams) -> Operation {
    Box::new(|| {
      let value = self.read_byte(source);
      let carry = self.read_register(Register::F).get_bit(4);
      let result = (value >> 1) | (if carry { 0x80u8 } else { 0x00u8 });
      let zero = !params.unset_zero && result == 0;
      let flag =
        ((zero as u8) << 7) & ((value.get_bit(0) as u8) << 4);
      self.write_register(Register::F, flag);
      self.write_byte(params.destination, result);
    })
  }

  pub fn shift_byte_left(&mut self, source: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(|| {
      let value = self.read_byte(source);
      let result = value << 1;
      let zero = result == 0;
      let flag =
        ((zero as u8) << 7) & ((value.get_bit(7) as u8) << 4);
      self.write_register(Register::F, flag);
      self.write_byte(destination, result);
    })
  }

  pub fn shift_byte_right(&mut self, source: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(|| {
      let value = self.read_byte(source);
      let result = value >> 1;
      let zero = result == 0;
      let flag =
        ((zero as u8) << 7) & ((value.get_bit(0) as u8) << 4);
      self.write_register(Register::F, flag);
      self.write_byte(destination, result);
    })
  }

  pub fn shift_byte_right_arithmetic(&mut self, source: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(|| {
      let value = self.read_byte(source);
      let result = (value >> 1) | (value & 0x80);
      let zero = result == 0;
      let flag =
        ((zero as u8) << 7) & ((value.get_bit(0) as u8) << 4);
      self.write_register(Register::F, flag);
      self.write_byte(destination, result);
    })
  }

  pub fn swap_byte(&mut self, source: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(|| {
      let value = self.read_byte(source);
      let result = value.rotate_left(4);
      let flag = if result == 0 { 0x80u8 } else { 0x00u8 };
      ;
      self.write_register(Register::F, flag);
      self.write_byte(destination, result);
    })
  }

  fn increment_word(&mut self, location: WordLocation) -> Operation {
    Box::new(|| {
      self.write_word(location, self.read_word(location).wrapping_add(1));
    })
  }

  fn decrement_word(&mut self, location: WordLocation) -> Operation {
    Box::new(|| {
      self.write_word(location, self.read_word(location).wrapping_sub(1));
    })
  }

  fn increment_register_pair(&mut self, register_pair: Register) -> Operation {
    Box::new(|| {
      self.write_register_pair(register_pair, self.read_register_pair(register_pair).wrapping_add(1));
    })
  }

  fn decrement_register_pair(&mut self, register_pair: Register) -> Operation {
    Box::new(|| {
      self.write_register_pair(register_pair, self.read_register_pair(register_pair).wrapping_sub(1));
    })
  }

  fn noop(&mut self, _opcode: Opcode) {}

  fn reg_to_reg_ld(&mut self, opcode: Opcode) {
    let src = Register::from_r_bits(opcode.z_bits());
    let dest = Register::from_r_bits(opcode.y_bits());
    self.write_register(dest, self.read_register(src));
  }

  fn immediate_to_reg_ld(&mut self) {
    let register = Register::from_r_bits(self.context.opcode.y_bits());
    self.operations.push_back(
      self.move_byte(ByteLocation::NextMemoryByte, ByteLocation::Register(register))
    );
  }

  fn immediate_to_indirect_ld(&mut self, _opcode: Opcode) {
    self.operations.push_back(self.write_next_byte_to_byte_buffer());
    self.operations.push_back(self.write_byte_buffer_to_memory_referenced_by_register_pair(Register::HL));
  }

  fn indirect_to_reg_ld(&mut self, opcode: Opcode) {
    let dest = Register::from_r_bits(opcode.y_bits());
    self.operations.push_back(self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::HL), ByteLocation::Register(dest)));
  }

  fn reg_to_indirect_ld(&mut self) {
    let src = Register::from_r_bits(self.context.opcode.z_bits());
    self.operations.push_back(self.move_byte(ByteLocation::Register(src), ByteLocation::MemoryReferencedByRegister(Register::HL)));
  }

  fn indirect_bc_to_reg_a_ld(&mut self) {
    self.operations.push_back(self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::BC), ByteLocation::Register(Register::A)));
  }

  fn indirect_de_to_reg_a_ld(&mut self) {
    self.operations.push_back(self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::DE), ByteLocation::Register(Register::A)));
  }

  fn indirect_c_with_offset_to_reg_a_ld(&mut self, _opcode: Opcode) {
    self.write_register(Register::A, self.memory.borrow().read(0xFF00 + self.read_register(Register::C) as u16));
  }

  fn reg_a_to_indirect_c_ld(&mut self, _opcode: Opcode) {
    self.memory.borrow_mut().write(0xFF00 + self.read_register(Register::C) as u16, self.read_register(Register::A));
  }

  fn immediate_indirect_with_offset_to_reg_a_ld(&mut self, _opcode: Opcode) {
    let offset = self.read_next_instruction() as u16;
    self.write_register(Register::A, self.memory.borrow().read(0xFF00 + offset));
  }

  fn reg_a_to_immediate_indirect_with_offset_ld(&mut self, _opcode: Opcode) {
    let offset = self.read_next_instruction() as u16;
    self.memory.borrow_mut().write(0xFF00 + offset, self.read_register(Register::A));
  }

  fn immediate_indirect_to_reg_a_ld(&mut self, _opcode: Opcode) {
    let lower_address = self.read_next_instruction();
    let upper_address = self.read_next_instruction();
    self.write_register(Register::A, self.memory.borrow().read((&[upper_address, lower_address][..]).read_u16::<BigEndian>().unwrap()));
  }

  fn reg_a_to_immediate_indirect_ld(&mut self, _opcode: Opcode) {
    let lower_address = self.read_next_instruction();
    let upper_address = self.read_next_instruction();
    self.memory.borrow_mut().write((&[upper_address, lower_address][..]).read_u16::<BigEndian>().unwrap(), self.read_register(Register::A));
  }

  fn indirect_hl_to_reg_a_ld_and_increment(&mut self, _opcode: Opcode) {
    self.operations.push_back(Operation::Composite(
      Operation::MoveByte {
        src: ByteLocation::MemoryAtRegisterAddress(Register::BC),
        dest: ByteLocation::Register(Register::A),
      },
      Operation::IncrementWord(WordLocation::Register(Register::HL)),
    ));
  }

  fn indirect_hl_to_reg_a_ld_and_decrement(&mut self, _opcode: Opcode) {
    self.operations.push_back(Operation::Composite(
      Operation::MoveByte {
        src: ByteLocation::MemoryAtRegisterAddress(Register::BC),
        dest: ByteLocation::Register(Register::A),
      },
      Operation::DecrementWord(WordLocation::Register(Register::HL)),
    ));
  }

  fn reg_a_to_indirect_bc_ld(&mut self) {
    self.operations.push_back(self.move_byte(ByteLocation::Register(Register::A), ByteLocation::MemoryReferencedByRegister(Register::BC)));
  }

  fn reg_a_to_indirect_de_ld(&mut self) {
    self.operations.push_back(self.move_byte(ByteLocation::Register(Register::A), ByteLocation::MemoryReferencedByRegister(Register::DE)));
  }

  fn reg_a_to_indirect_hl_ld_and_increment(&mut self) {
    self.operations.push_back(CPU::combine_operations(
      self.move_byte(ByteLocation::Register(Register::A), ByteLocation::MemoryReferencedByRegister(Register::HL)),
      self.increment_register_pair(Register::HL),
    ));
  }

  fn reg_a_to_indirect_hl_ld_and_decrement(&mut self, _opcode: Opcode) {
    self.operations.push_back(CPU::combine_operations(
      self.write_register_to_memory_referenced_by_register_pair(Register::A, Register::HL),
      self.increment_register_pair(Register::HL),
    ));
  }

  fn immediate_to_reg_pair_ld(&mut self) {
    let register = Register::from_dd_bits(self.context.opcode.dd_bits());
    self.operations.push_back(self.move_byte(ByteLocation::NextMemoryByte, ByteLocation::Register(register.get_lower_register())));
    self.operations.push_back(self.move_byte(ByteLocation::NextMemoryByte, ByteLocation::Register(register.get_upper_register())));
  }

  fn reg_hl_to_reg_sp_ld(&mut self, _opcode: Opcode) {
    let value = self.read_register_pair(Register::HL);
    self.write_register_pair(Register::SP, value);
  }

  fn push_reg_pair_to_stack(&mut self, opcode: Opcode) {
    let mut memory = self.memory.borrow_mut();
    let value = self.read_register_pair(Register::from_qq_bits(opcode.qq_bits())).to_be_bytes();
    memory.write(self.read_and_decrement_register_pair(Register::SP), value[0]);
    memory.write(self.read_and_decrement_register_pair(Register::SP), value[1]);
  }

  fn pop_stack_to_reg_pair(&mut self, opcode: Opcode) {
    let memory = self.memory.borrow();
    let lower_bits = memory.read(self.read_and_increment_register_pair(Register::SP));
    let upper_bits = memory.read(self.read_and_increment_register_pair(Register::SP));
    let value = (&[upper_bits, lower_bits][..]).read_u16::<BigEndian>().unwrap();
    self.write_register_pair(Register::from_qq_bits(opcode.qq_bits()), value);
  }

  // TODO: Do a more thorough check to see if this is correct. There seems to be a lot of confusion surrounding the (half) carry bits
  fn reg_sp_plus_signed_immediate_to_hl_ld(&mut self, _opcode: Opcode) {
    let signed_value = self.read_next_instruction() as i8 as u16;
    let reg_sp = self.read_register_pair(Register::SP);
    let result = reg_sp.wrapping_add(signed_value);
    let temp = ((!result & (reg_sp | signed_value)) | (reg_sp & signed_value)).to_be_bytes()[0];
    self.write_register(Register::F, (temp & 0x80).wrapping_shr(3) | ((temp & 0x08).wrapping_shl(2)));
    self.write_register_pair(Register::HL, result);
  }

  fn reg_sp_to_immediate_indirect_ld(&mut self) {
    self.operations.push_back(self.move_byte(ByteLocation::NextMemoryByte, ByteLocation::LowerAddressBuffer));
    self.operations.push_back(self.move_byte(ByteLocation::NextMemoryByte, ByteLocation::UpperAddressBuffer));
    self.operations.push_back(
      self.move_byte(ByteLocation::Register(Register::SP.get_lower_register()), ByteLocation::MemoryReferencedByAddressBuffer)
    );
    self.operations.push_back(CPU::combine_operations(
      self.increment_word(WordLocation::AddressBuffer),
      self.move_byte(ByteLocation::Register(Register::SP.get_upper_register()), ByteLocation::MemoryReferencedByAddressBuffer)),
    );
  }

  fn add_reg_to_reg_a_and_write_to_reg_a(&mut self, use_carry: bool) {
    self.add_bytes(ByteArithmeticParams {
      first: ByteLocation::Register(Register::A),
      second: ByteLocation::Register(Register::from_r_bits(self.context.opcode.z_bits())),
      destination: ByteLocation::Register(Register::A),
      use_carry: use_carry,
      flag_mask: 0xF0,
    })();
  }

  fn add_immediate_to_reg_a_and_write_to_reg_a(&mut self, use_carry: bool) {
    self.operations.push_back(
      self.add_bytes(ByteArithmeticParams {
        first: ByteLocation::Register(Register::A),
        second: ByteLocation::NextMemoryByte,
        destination: ByteLocation::Register(Register::A),
        use_carry: use_carry,
        flag_mask: 0xF0,
      })
    );
  }

  fn add_indirect_hl_to_reg_a_and_write_to_reg_a(&mut self, use_carry: bool) {
    self.operations.push_back(
      self.add_bytes(ByteArithmeticParams {
        first: ByteLocation::Register(Register::A),
        second: ByteLocation::MemoryReferencedByRegister(Register::HL),
        destination: ByteLocation::Register(Register::A),
        use_carry: use_carry,
        flag_mask: 0xF0,
      })
    );
  }

  fn subtract_reg_from_reg_a_and_write_to_reg_a(&mut self, use_carry: bool) {
    self.subtract_bytes(ByteArithmeticParams {
      first: ByteLocation::Register(Register::A),
      second: ByteLocation::Register(Register::from_r_bits(self.context.opcode.z_bits())),
      destination: ByteLocation::Register(Register::A),
      use_carry: use_carry,
      flag_mask: 0xF0,
    })();
  }

  fn subtract_immediate_from_reg_a_and_write_to_reg_a(&mut self, use_carry: bool) {
    self.operations.push_back(
      self.subtract_bytes(ByteArithmeticParams {
        first: ByteLocation::Register(Register::A),
        second: ByteLocation::NextMemoryByte,
        destination: ByteLocation::Register(Register::A),
        use_carry: use_carry,
        flag_mask: 0xF0,
      })
    );
  }

  fn subtract_indirect_hl_from_reg_a_and_write_to_reg_a(&mut self, use_carry: bool) {
    self.operations.push_back(
      self.subtract_bytes(ByteArithmeticParams {
        first: ByteLocation::Register(Register::A),
        second: ByteLocation::MemoryReferencedByRegister(Register::HL),
        destination: ByteLocation::Register(Register::A),
        use_carry: use_carry,
        flag_mask: 0xF0,
      })
    );
  }

  fn and_value_with_reg_a_and_write_to_reg_a(&mut self, value: u8) {
    let reg_a = self.read_register(Register::A);
    let result = ALU::and(reg_a, value);
    self.write_register(Register::F, u8::compose(&[(result.zero, 7), (result.half_carry, 5), (result.carry, 4)]));
    self.write_register(Register::A, result.value);
  }

  fn and_reg_with_reg_a_and_write_to_reg_a(&mut self, opcode: Opcode) {
    let value = self.read_register(Register::from_r_bits(opcode.z_bits()));
    self.and_value_with_reg_a_and_write_to_reg_a(value);
  }

  fn and_immediate_with_reg_a_and_write_to_reg_a(&mut self, _opcode: Opcode) {
    let value = self.read_next_instruction();
    self.and_value_with_reg_a_and_write_to_reg_a(value);
  }

  fn and_indirect_hl_with_reg_a_and_write_to_reg_a(&mut self, _opcode: Opcode) {
    let address = self.read_register_pair(Register::HL);
    self.and_value_with_reg_a_and_write_to_reg_a(self.memory.borrow().read(address));
  }

  fn or_value_with_reg_a_and_write_to_reg_a(&mut self, value: u8) {
    let reg_a = self.read_register(Register::A);
    let result = ALU::or(reg_a, value);
    self.write_register(Register::F, u8::compose(&[(result.zero, 7), (result.half_carry, 5), (result.carry, 4)]));
    self.write_register(Register::A, result.value);
  }

  fn or_reg_with_reg_a_and_write_to_reg_a(&mut self, opcode: Opcode) {
    let value = self.read_register(Register::from_r_bits(opcode.z_bits()));
    self.or_value_with_reg_a_and_write_to_reg_a(value);
  }

  fn or_immediate_with_reg_a_and_write_to_reg_a(&mut self, _opcode: Opcode) {
    let value = self.read_next_instruction();
    self.or_value_with_reg_a_and_write_to_reg_a(value);
  }

  fn or_indirect_hl_with_reg_a_and_write_to_reg_a(&mut self, _opcode: Opcode) {
    let address = self.read_register_pair(Register::HL);
    self.or_value_with_reg_a_and_write_to_reg_a(self.memory.borrow().read(address));
  }

  fn xor_value_with_reg_a_and_write_to_reg_a(&mut self, value: u8) {
    let reg_a = self.read_register(Register::A);
    let result = ALU::xor(reg_a, value);
    self.write_register(Register::F, u8::compose(&[(result.zero, 7), (result.half_carry, 5), (result.carry, 4)]));
    self.write_register(Register::A, result.value);
  }

  fn xor_reg_with_reg_a_and_write_to_reg_a(&mut self, opcode: Opcode) {
    let value = self.read_register(Register::from_r_bits(opcode.z_bits()));
    self.xor_value_with_reg_a_and_write_to_reg_a(value);
  }

  fn xor_immediate_with_reg_a_and_write_to_reg_a(&mut self, _opcode: Opcode) {
    let value = self.read_next_instruction();
    self.xor_value_with_reg_a_and_write_to_reg_a(value);
  }

  fn xor_indirect_hl_with_reg_a_and_write_to_reg_a(&mut self, _opcode: Opcode) {
    let address = self.read_register_pair(Register::HL);
    self.xor_value_with_reg_a_and_write_to_reg_a(self.memory.borrow().read(address));
  }

  fn compare_value_with_reg_a(&mut self, value: u8) {
    let reg_a = self.read_register(Register::A);
    let result = ALU::subtract(reg_a, value);
    self.write_register(Register::F, u8::compose(&[(result.zero, 7), (true, 6), (result.half_carry, 5), (result.carry, 4)]));
  }

  fn compare_reg_with_reg_a(&mut self, opcode: Opcode) {
    let value = self.read_register(Register::from_r_bits(opcode.z_bits()));
    self.compare_value_with_reg_a(value);
  }

  fn compare_immediate_with_reg_a(&mut self, _opcode: Opcode) {
    let value = self.read_next_instruction();
    self.compare_value_with_reg_a(value);
  }

  fn compare_indirect_hl_with_reg_a(&mut self, _opcode: Opcode) {
    let address = self.read_register_pair(Register::HL);
    self.compare_value_with_reg_a(self.memory.borrow().read(address));
  }

  fn increment_reg(&mut self) {
    let register = Register::from_r_bits(self.context.opcode.y_bits());
    self.add_bytes(ByteArithmeticParams {
      first: ByteLocation::Register(register),
      second: ByteLocation::Value(1),
      destination: ByteLocation::Register(register),
      use_carry: false,
      flag_mask: 0xE0,
    })();
  }

  fn increment_indirect_hl(&mut self, _opcode: Opcode) {
    self.operations.push_back(
      self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::HL), ByteLocation::ByteBuffer)
    );
    self.operations.push_back(
      self.add_bytes(ByteArithmeticParams {
        first: ByteLocation::ByteBuffer,
        second: ByteLocation::Value(1),
        destination: ByteLocation::MemoryReferencedByRegister(Register::HL),
        use_carry: false,
        flag_mask: 0xE0,
      })
    );
  }

  fn decrement_reg(&mut self) {
    let register = Register::from_r_bits(self.context.opcode.y_bits());
    self.subtract_bytes(ByteArithmeticParams {
      first: ByteLocation::Register(register),
      second: ByteLocation::Value(1),
      destination: ByteLocation::Register(register),
      use_carry: false,
      flag_mask: 0xE0,
    })();
  }

  fn decrement_indirect_hl(&mut self, _opcode: Opcode) {
    self.operations.push_back(
      self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::HL), ByteLocation::ByteBuffer)
    );
    self.operations.push_back(
      self.subtract_bytes(ByteArithmeticParams {
        first: ByteLocation::ByteBuffer,
        second: ByteLocation::Value(1),
        destination: ByteLocation::MemoryReferencedByRegister(Register::HL),
        use_carry: false,
        flag_mask: 0xE0,
      })
    );
  }

  fn add_reg_pair_to_reg_hl(&mut self) {
    let register = Register::from_dd_bits(self.context.opcode.dd_bits());
    self.add_words(WordArithmeticParams {
      first: WordLocation::Register(register),
      second: WordLocation::Register(Register::HL),
      destination: WordLocation::WordBuffer,
      flag_mask: 0x70,
    })();
    self.move_byte(ByteLocation::LowerWordBuffer, ByteLocation::Register(Register::L))();
    self.operations.push_back(
      self.move_byte(ByteLocation::UpperWordBuffer, ByteLocation::Register(Register::H))
    );
  }

  //TODO: Check whether the flags are set correctly
  fn add_immediate_to_reg_sp(&mut self, _opcode: Opcode) {
    let value = self.read_next_instruction();
    let sp_value = self.read_register_pair(Register::SP);
    let result = ALU::add_pair(sp_value, value as u16);
    self.write_register(Register::F, u8::compose(&[(result.half_carry, 5), (result.carry, 4)]));
    self.write_register_pair(Register::SP, result.value);
  }

  fn increment_reg_pair(&mut self) {
    let register = Register::from_dd_bits(self.context.opcode.dd_bits());
    self.move_word(WordLocation::Register(register), WordLocation::WordBuffer)();
    self.increment_word(WordLocation::Register(register))();
    self.move_byte(ByteLocation::LowerWordBuffer, ByteLocation::Register(register.get_lower_register()))();
    self.operations.push_back(
      self.move_byte(ByteLocation::UpperWordBuffer, ByteLocation::Register(register.get_upper_register()))
    );
  }

  fn decrement_reg_pair(&mut self) {
    let register = Register::from_dd_bits(self.context.opcode.dd_bits());
    self.move_word(WordLocation::Register(register), WordLocation::WordBuffer)();
    self.decrement_word(WordLocation::Register(register))();
    self.move_byte(ByteLocation::LowerWordBuffer, ByteLocation::Register(register.get_lower_register()))();
    self.operations.push_back(
      self.move_byte(ByteLocation::UpperWordBuffer, ByteLocation::Register(register.get_upper_register()))
    );
  }

  fn rotate_reg_a_left(&mut self) {
    self.rotate_byte_left(ByteRotationParams {
      source: ByteLocation::Register(Register::A),
      destination: ByteLocation::Register(Register::A),
      unset_zero: true,
    })();
  }

  fn rotate_reg_left(&mut self) {
    let register = Register::from_r_bits(self.context.opcode.z_bits());
    self.rotate_byte_left(ByteRotationParams {
      source: ByteLocation::Register(register),
      destination: ByteLocation::Register(register),
      unset_zero: false,
    })();
  }

  fn rotate_indirect_hl_left(&mut self) {
    self.operations.push_back(
      self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::HL), ByteLocation::ByteBuffer)
    );
    self.operations.push_back(
      self.rotate_byte_left(ByteRotationParams {
        source: ByteLocation::ByteBuffer,
        destination: ByteLocation::MemoryReferencedByRegister(Register::HL),
        unset_zero: false,
      })
    );
  }

  fn rotate_reg_a_left_through_carry(&mut self) {
    self.rotate_byte_left_through_carry(ByteRotationParams {
      source: ByteLocation::Register(Register::A),
      destination: ByteLocation::Register(Register::A),
      unset_zero: true,
    })();
  }

  fn rotate_reg_left_through_carry(&mut self, opcode: Opcode) {
    let register = Register::from_r_bits(self.context.opcode.z_bits());
    self.rotate_byte_left_through_carry(ByteRotationParams {
      source: ByteLocation::Register(register),
      destination: ByteLocation::Register(register),
      unset_zero: false,
    })();
  }

  fn rotate_indirect_hl_left_through_carry(&mut self, _opcode: Opcode) {
    self.operations.push_back(
      self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::HL), ByteLocation::ByteBuffer)
    );
    self.operations.push_back(
      self.rotate_byte_left_through_carry(ByteRotationParams {
        source: ByteLocation::ByteBuffer,
        destination: ByteLocation::MemoryReferencedByRegister(Register::HL),
        unset_zero: false,
      })
    );
  }

  fn rotate_reg_a_right(&mut self) {
    self.rotate_byte_right(ByteRotationParams {
      source: ByteLocation::Register(Register::A),
      destination: ByteLocation::Register(Register::A),
      unset_zero: true,
    })();
  }

  fn rotate_reg_right(&mut self, opcode: Opcode) {
    let register = Register::from_r_bits(opcode.z_bits());
    self.rotate_byte_right(ByteRotationParams {
      source: ByteLocation::Register(register),
      destination: ByteLocation::Register(register),
      unset_zero: false,
    })();
  }

  fn rotate_indirect_hl_right(&mut self, _opcode: Opcode) {
    self.operations.push_back(
      self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::HL), ByteLocation::ByteBuffer)
    );
    self.operations.push_back(
      self.rotate_byte_right(ByteRotationParams {
        source: ByteLocation::ByteBuffer,
        destination: ByteLocation::MemoryReferencedByRegister(Register::HL),
        unset_zero: false,
      })
    );
  }

  fn rotate_reg_a_right_through_carry(&mut self) {
    self.rotate_byte_right_through_carry(ByteRotationParams {
      source: ByteLocation::Register(Register::A),
      destination: ByteLocation::Register(Register::A),
      unset_zero: true,
    })();
  }

  fn rotate_reg_right_through_carry(&mut self, opcode: Opcode) {
    let register = Register::from_r_bits(opcode.z_bits());
    self.rotate_byte_right_through_carry(ByteRotationParams {
      source: ByteLocation::Register(register),
      destination: ByteLocation::Register(register),
      unset_zero: false,
    })();
  }

  fn rotate_indirect_hl_right_through_carry(&mut self, _opcode: Opcode) {
    self.operations.push_back(
      self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::HL), ByteLocation::ByteBuffer)
    );
    self.operations.push_back(
      self.rotate_byte_right_through_carry(ByteRotationParams {
        source: ByteLocation::ByteBuffer,
        destination: ByteLocation::MemoryReferencedByRegister(Register::HL),
        unset_zero: false,
      })
    );
  }

  fn shift_reg_left(&mut self, opcode: Opcode) {
    let register = Register::from_r_bits(opcode.z_bits());
    self.shift_byte_left(ByteLocation::Register(register), ByteLocation::Register(register))();
  }

  fn shift_reg_right(&mut self, opcode: Opcode) {
    let register = Register::from_r_bits(opcode.z_bits());
    self.shift_byte_right(ByteLocation::Register(register), ByteLocation::Register(register))();
  }

  fn shift_reg_right_arithmetic(&mut self, opcode: Opcode) {
    let register = Register::from_r_bits(opcode.z_bits());
    let register = Register::from_r_bits(opcode.z_bits());
    self.shift_byte_right_arithmetic(ByteLocation::Register(register), ByteLocation::Register(register))();
  }

  fn shift_indirect_hl_left(&mut self, _opcode: Opcode) {
    self.operations.push_back(
      self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::HL), ByteLocation::ByteBuffer)
    );
    self.operations.push_back(
      self.shift_byte_left(ByteLocation::ByteBuffer, ByteLocation::MemoryReferencedByRegister(Register::HL))
    );
  }

  fn shift_indirect_hl_right(&mut self, _opcode: Opcode) {
    self.operations.push_back(
      self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::HL), ByteLocation::ByteBuffer)
    );
    self.operations.push_back(
      self.shift_byte_right(ByteLocation::ByteBuffer, ByteLocation::MemoryReferencedByRegister(Register::HL))
    );
  }

  fn shift_indirect_hl_right_arithmetic(&mut self, _opcode: Opcode) {
    self.operations.push_back(
      self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::HL), ByteLocation::ByteBuffer)
    );
    self.operations.push_back(
      self.shift_byte_right_arithmetic(ByteLocation::ByteBuffer, ByteLocation::MemoryReferencedByRegister(Register::HL))
    );
  }

  fn swap_reg(&mut self, opcode: Opcode) {
    let register = Register::from_r_bits(opcode.z_bits());
    self.swap_byte(ByteLocation::Register(register), ByteLocation::Register(register))();
  }

  fn swap_indirect_hl(&mut self, _opcode: Opcode) {
    self.operations.push_back(
      self.move_byte(ByteLocation::MemoryReferencedByRegister(Register::HL), ByteLocation::ByteBuffer)
    );
    self.operations.push_back(
      self.swap_byte(ByteLocation::ByteBuffer, ByteLocation::MemoryReferencedByRegister(Register::HL))
    );
  }

  fn get_reg_bit(&mut self, opcode: Opcode) {
    let value = self.read_register(Register::from_r_bits(opcode.z_bits()));
    let bit = opcode.y_bits();
    self.write_register_masked(Register::F, u8::compose(&[(!value.get_bit(bit), 7), (true, 5)]), 0xE0);
  }

  fn get_indirect_hl_bit(&mut self, opcode: Opcode) {
    let address = self.read_register_pair(Register::HL);
    let value = self.memory.borrow().read(address);
    let bit = opcode.y_bits();
    self.write_register_masked(Register::F, u8::compose(&[(!value.get_bit(bit), 7), (true, 5)]), 0xE0);
  }

  fn set_reg_bit(&mut self, opcode: Opcode) {
    let register = Register::from_r_bits(opcode.z_bits());
    let value = self.read_register(register);
    let bit = opcode.y_bits();
    self.write_register(register, value.set_bit(bit));
  }

  fn set_indirect_hl_bit(&mut self, opcode: Opcode) {
    let address = self.read_register_pair(Register::HL);
    let value = self.memory.borrow().read(address);
    let bit = opcode.y_bits();
    self.memory.borrow_mut().write(address, value.set_bit(bit));
  }

  fn reset_reg_bit(&mut self, opcode: Opcode) {
    let register = Register::from_r_bits(opcode.z_bits());
    let value = self.read_register(register);
    let bit = opcode.y_bits();
    self.write_register(register, value.reset_bit(bit));
  }

  fn reset_indirect_hl_bit(&mut self, opcode: Opcode) {
    let address = self.read_register_pair(Register::HL);
    let value = self.memory.borrow().read(address);
    let bit = opcode.y_bits();
    self.memory.borrow_mut().write(address, value.reset_bit(bit));
  }

  fn jump(&mut self, _opcode: Opcode) {
    let lower_address = self.read_next_instruction();
    let upper_address = self.read_next_instruction();
    let address = (&[upper_address, lower_address][..]).read_u16::<BigEndian>().unwrap();
    self.write_register_pair(Register::PC, address);
  }

  fn satisfies_condition(&self, opcode: Opcode) -> bool {
    let condition = opcode.cc_bits();
    match condition {
      0x00 => !self.read_register(Register::F).get_bit(7),
      0x01 => self.read_register(Register::F).get_bit(7),
      0x02 => !self.read_register(Register::F).get_bit(4),
      0x03 => self.read_register(Register::F).get_bit(4),
      _ => panic!("{} doesn't map to a condition value", condition)
    }
  }

  fn jump_conditional(&mut self, opcode: Opcode) {
    if self.satisfies_condition(opcode) {
      let lower_address = self.read_next_instruction();
      let upper_address = self.read_next_instruction();
      let address = (&[upper_address, lower_address][..]).read_u16::<BigEndian>().unwrap();
      self.write_register_pair(Register::PC, address);
    } else {
      self.write_register_pair(Register::PC, self.read_register_pair(Register::PC) + 2);
    }
  }

  fn jump_relative(&mut self) {
    self.operations.push_back(self.move_byte(ByteLocation::NextMemoryByte, ByteLocation::ByteBuffer));
    self.operations.push_back(Box::new(|| {
      self.write_register_pair(Register::PC, self.read_register_pair(Register::PC).wrapping_add(self.context.byte_buffer as i8 as u16));
    }));
  }

  fn jump_conditional_relative(&mut self) {
    self.operations.push_back(self.move_byte(ByteLocation::NextMemoryByte, ByteLocation::ByteBuffer));
    if self.satisfies_condition(self.context.opcode) {
      self.operations.push_back(Box::new(|| {
        self.write_register_pair(Register::PC, self.read_register_pair(Register::PC).wrapping_add(self.context.byte_buffer as i8 as u16));
      }));
    }
  }

  fn jump_to_indirect_hl(&mut self, _opcode: Opcode) {
    self.write_register_pair(Register::PC, self.read_register_pair(Register::HL));
  }

  fn call_address(&mut self, address: u16) {
    let mut memory = self.memory.borrow_mut();
    let pc_bytes = (self.read_register_pair(Register::PC)).to_be_bytes();
    memory.write(self.decrement_and_read_register_pair(Register::SP), pc_bytes[0]);
    memory.write(self.decrement_and_read_register_pair(Register::SP), pc_bytes[1]);
    self.write_register_pair(Register::PC, address);
  }

  fn call(&mut self, _opcode: Opcode) {
    let lower_address = self.read_next_instruction();
    let upper_address = self.read_next_instruction();
    self.call_address((&[upper_address, lower_address][..]).read_u16::<BigEndian>().unwrap());
  }

  fn call_conditional(&mut self, opcode: Opcode) {
    if self.satisfies_condition(opcode) {
      self.call(opcode);
    } else {
      self.write_register_pair(Register::PC, self.read_register_pair(Register::PC) + 2);
    }
  }

  fn return_from_call(&mut self, _opcode: Opcode) {
    let memory = self.memory.borrow();
    let lower_pc = memory.read(self.read_and_increment_register_pair(Register::SP));
    let upper_pc = memory.read(self.read_and_increment_register_pair(Register::SP));
    self.write_register_pair(Register::PC, (&[upper_pc, lower_pc][..]).read_u16::<BigEndian>().unwrap());
  }

  fn return_from_interrupt(&mut self, opcode: Opcode) {
    self.return_from_call(opcode);
    self.ime = true;
  }

  fn return_conditionally(&mut self, opcode: Opcode) {
    if self.satisfies_condition(opcode) {
      self.return_from_call(opcode);
    } else {}
  }

  fn restart(&mut self, opcode: Opcode) {
    let t = opcode.y_bits();
    let address = match t {
      0 => 0x0000u16,
      1 => 0x0008u16,
      2 => 0x0010u16,
      3 => 0x0018u16,
      4 => 0x0020u16,
      5 => 0x0028u16,
      6 => 0x0030u16,
      7 => 0x0038u16,
      _ => panic!("{} is not a valid restart code", t)
    };
    self.call_address(address);
  }

  fn decimal_adjust_reg_a(&mut self, _opcode: Opcode) {
    let f = self.read_register(Register::F);
    let n = f.get_bit(6);
    let carry = f.get_bit(4);
    let half_carry = f.get_bit(5);
    let result = if n {
      let lower = if half_carry { 6u8 } else { 0u8 };
      let upper = if carry { 0x60u8 } else { 0u8 };
      ALU::subtract(a, upper | lower)
    } else {
      let lower = if half_carry || ((a & 0x0F) >= 0x0A) { 6u8 } else { 0u8 };
      let upper = if carry || (a > 0x99) { 0x60u8 } else { 0u8 };
      ALU::add(a, upper | lower)
    };
    self.write_register_masked(Register::F, u8::compose(&[(result.zero, 7), (result.carry | carry, 4)]), 0xB0);
    self.write_register(Register::A, result.value);
  }

  fn ones_complement_reg_a(&mut self, _opcode: Opcode) {
    self.write_register(Register::A, !self.read_register(Register::A));
    self.write_register_masked(Register::F, 0x60, 0x60);
  }

  fn flip_carry_flag(&mut self, _opcode: Opcode) {
    self.write_register_masked(Register::F, (self.read_register(Register::F) ^ 0x10) & 0x90, 0x70);
  }

  fn set_carry_flag(&mut self, _opcode: Opcode) {
    self.write_register_masked(Register::F, 0x10, 0x70);
  }

  fn disable_interrupts(&mut self, _opcode: Opcode) {
    self.ime = false;
  }

  fn enable_interrupts(&mut self, _opcode: Opcode) {
    self.ime = true;
  }

  fn halt(&mut self, _opcode: Opcode) {
    //TODO: Implement halt
  }

  fn stop(&mut self) {
    // TODO: Implement stop
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::memory::memory::test::MockMemory;
  use test_case::test_case;
  use crate::context::context::Context;

  #[test]
  fn read_register() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.registers[2] = 0xAB;
    assert_eq!(cpu.read_register(Register::B), 0xAB);
  }

  #[test]
  fn read_register_pair() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.registers[2] = 0xAB;
    cpu.registers[3] = 0xCD;
    assert_eq!(cpu.read_register_pair(Register::BC), 0xABCD);
  }

  #[test]
  fn write_register() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::B, 0xAB);
    assert_eq!(cpu.registers[2], 0xAB);
  }

  #[test]
  fn write_register_pair() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::BC, 0xABCD);
    assert_eq!(cpu.registers[2], 0xAB);
    assert_eq!(cpu.registers[3], 0xCD);
  }

  #[test]
  fn reg_to_reg_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    memory.write(0x0000, 0x45);
    cpu.write_register(Register::L, 0xAB);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::B), 0xAB);
  }

  #[test]
  fn immediate_to_reg_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    memory.write(0x0000, 0x06);
    memory.write(0x0001, 0xAB);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::B), 0xAB);
  }

  #[test]
  fn indirect_to_reg_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    memory.write(0x0000, 0x6E);
    memory.write(0xABCD, 0xEF);
    cpu.write_register_pair(Register::HL, 0xABCD);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::L), 0xEF);
  }

  #[test]
  fn reg_to_indirect_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    cpu.write_register(Register::A, 0xEF);
    memory.write(0x0000, 0x77);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0xEF);
  }

  #[test]
  fn immediate_to_indirect_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x36);
    memory.write(0x0001, 0xEF);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0xEF);
  }

  #[test]
  fn indirect_bc_to_reg_a_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::BC, 0xABCD);
    memory.write(0x0000, 0x0A);
    memory.write(0xABCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0x5A);
  }

  #[test]
  fn indirect_de_to_reg_a_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::DE, 0xABCD);
    memory.write(0x0000, 0x1A);
    memory.write(0xABCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0x5A);
  }

  #[test]
  fn indirect_c_with_offset_to_reg_a_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::C, 0xCD);
    memory.write(0x0000, 0xF2);
    memory.write(0xFFCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0x5A);
  }

  #[test]
  fn reg_a_to_indirect_c_with_offset_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0x5A);
    cpu.write_register(Register::C, 0xCD);
    memory.write(0x0000, 0xE2);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xFFCD), 0x5A);
  }

  #[test]
  fn immediate_indirect_with_offset_to_reg_a_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    memory.write(0x0000, 0xF0);
    memory.write(0x0001, 0xCD);
    memory.write(0xFFCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0x5A);
  }

  #[test]
  fn reg_a_to_immediate_indirect_with_offset_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0x5A);
    memory.write(0x0000, 0xE0);
    memory.write(0x0001, 0xCD);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xFFCD), 0x5A);
  }

  #[test]
  fn immediate_indirect_to_reg_a_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    memory.write(0x0000, 0xFA);
    memory.write(0x0001, 0xCD);
    memory.write(0x0002, 0xAB);
    memory.write(0xABCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0x5A);
  }

  #[test]
  fn reg_a_to_immediate_indirect_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0x5A);
    memory.write(0x0000, 0xEA);
    memory.write(0x0001, 0xCD);
    memory.write(0x0002, 0xAB);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
  }


  #[test]
  fn indirect_hl_to_reg_a_ld_and_increment() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x2A);
    memory.write(0xABCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
    assert_eq!(cpu.read_register_pair(Register::HL), 0xABCE);
  }

  #[test]
  fn indirect_hl_to_reg_a_ld_and_decrement() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x3A);
    memory.write(0xABCD, 0x5A);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
    assert_eq!(cpu.read_register_pair(Register::HL), 0xABCC);
  }

  #[test]
  fn reg_a_to_indirect_bc_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0x5A);
    cpu.write_register_pair(Register::BC, 0xABCD);
    memory.write(0x0000, 0x02);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
  }

  #[test]
  fn reg_a_to_indirect_de_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0x5A);
    cpu.write_register_pair(Register::DE, 0xABCD);
    memory.write(0x0000, 0x12);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
  }

  #[test]
  fn reg_a_to_indirect_hl_ld_and_increment() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0x5A);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x22);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
    assert_eq!(cpu.read_register_pair(Register::HL), 0xABCE);
  }

  #[test]
  fn reg_a_to_indirect_hl_ld_and_decrement() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0x5A);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x32);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
    assert_eq!(cpu.read_register_pair(Register::HL), 0xABCC);
  }


  #[test]
  fn immediate_to_reg_pair_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0x5A);
    memory.write(0x0000, 0x21);
    memory.write(0x0001, 0x5A);
    memory.write(0x0002, 0x7B);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::HL), 0x7B5A);
  }

  #[test]
  fn reg_hl_to_reg_sp_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xF9);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::SP), 0xABCD);
  }

  #[test]
  fn push_reg_pair_to_stack() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
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
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::SP, 0xFFFC);
    memory.write(0x0000, 0xD1);
    memory.write(0xFFFC, 0xCD);
    memory.write(0xFFFD, 0xAB);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::DE), 0xABCD);
    assert_eq!(cpu.read_register_pair(Register::SP), 0xFFFE);
  }

  #[test]
  fn reg_sp_plus_signed_immediate_to_hl_ld_writes_correct_result() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    // Check if carry flag is set correctly
    cpu.write_register_pair(Register::SP, 0x0005);
    memory.write(0x0000, 0xF8);
    memory.write(0x0001, 0xFD);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::HL), 0x0002);
  }

  #[test_case(0x0FF8, 0x07, 0x00; "no flags")]
  #[test_case(0x0FF8, 0x08, 0x20; "only half carry")]
  #[test_case(0xFFF8, 0x08, 0x30; "both carry flags")]
  fn reg_sp_plus_signed_immediate_to_hl_ld_writes_correct_flags(sp: u16, e: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::SP, sp);
    memory.write(0x0000, 0xF8);
    memory.write(0x0001, e);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test]
  fn reg_sp_to_immediate_indirect_ld() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::SP, 0x7B5A);
    memory.write(0x0000, 0x08);
    memory.write(0x0001, 0xCD);
    memory.write(0x0002, 0xAB);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), 0x5A);
    assert_eq!(memory.read(0xABCE), 0x7B);
  }

  #[test_case(0xFC, 0x04, 0x00, 0xB0; "zero flag set correctly")]
  #[test_case(0xF0, 0xF0, 0xE0, 0x10; "carry set correctly")]
  #[test_case(0x08, 0x08, 0x10, 0x20; "half carry set correctly")]
  fn add_reg_to_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0x82);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0x04, 0x00, 0xB0; "zero flag set correctly")]
  #[test_case(0xF0, 0xF0, 0xE0, 0x10; "carry set correctly")]
  #[test_case(0x08, 0x08, 0x10, 0x20; "half carry set correctly")]
  fn add_immediate_to_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    memory.write(0x0000, 0xC6);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0x04, 0x00, 0xB0; "zero flag set correctly")]
  #[test_case(0xF0, 0xF0, 0xE0, 0x10; "carry set correctly")]
  #[test_case(0x08, 0x08, 0x10, 0x20; "half carry set correctly")]
  fn add_indirect_hl_to_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x86);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0x03, 0x00, 0xB0; "zero flag set correctly")]
  #[test_case(0xF0, 0xEF, 0xE0, 0x30; "carry set correctly")]
  #[test_case(0x08, 0x07, 0x10, 0x20; "half carry set correctly")]
  fn add_reg_with_carry_to_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, 0x10);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0x8A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0x03, 0x00, 0xB0; "zero flag set correctly")]
  #[test_case(0xF0, 0xEF, 0xE0, 0x30; "carry set correctly")]
  #[test_case(0x08, 0x07, 0x10, 0x20; "half carry set correctly")]
  fn add_immediate_with_carry_to_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    memory.write(0x0000, 0xCE);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0x03, 0x00, 0xB0; "zero flag set correctly")]
  #[test_case(0xF0, 0x10, 0x01, 0x10; "carry set correctly")]
  #[test_case(0x08, 0x07, 0x10, 0x20; "half carry set correctly")]
  fn add_indirect_hl_with_carry_to_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x8E);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFC, 0x00, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0xE0, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x0E, 0x60; "half carry set correctly")]
  fn subtract_reg_from_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0x92);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFC, 0x00, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0xE0, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x0E, 0x60; "half carry set correctly")]
  fn subtract_immediate_from_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    memory.write(0x0000, 0xD6);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFC, 0x00, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0xE0, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x0E, 0x60; "half carry set correctly")]
  fn subtract_indirect_hl_from_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x96);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFB, 0x00, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3E, 0xE0, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE2, 0x0E, 0x60; "half carry set correctly")]
  fn subtract_reg_with_carry_from_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, 0x10);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0x9A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFB, 0x00, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3E, 0xE0, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE2, 0x0E, 0x60; "half carry set correctly")]
  fn subtract_immediate_with_carry_from_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    memory.write(0x0000, 0xDE);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFB, 0x00, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3E, 0xE0, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE2, 0x0E, 0x60; "half carry set correctly")]
  fn subtract_indirect_hl_with_carry_from_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x9E);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x5A, 0xA5, 0x00, 0xA0; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0x88, 0x20; "half carry set correctly")]
  fn and_reg_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0xA2);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x5A, 0xA5, 0x00, 0xA0; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0x88, 0x20; "half carry set correctly")]
  fn and_immediate_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    memory.write(0x0000, 0xE6);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x5A, 0xA5, 0x00, 0xA0; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0x88, 0x20; "half carry set correctly")]
  fn and_indirect_hl_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xA6);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x00, 0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0xEE, 0x00; "calculates OR correctly")]
  fn or_reg_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0xB2);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x00, 0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0xEE, 0x00; "calculates OR correctly")]
  fn or_immediate_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    memory.write(0x0000, 0xF6);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x00, 0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0xEE, 0x00; "calculates OR correctly")]
  fn or_indirect_hl_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xB6);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xAE, 0xAE, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0x66, 0x00; "calculates XOR correctly")]
  fn xor_reg_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0xAA);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xAE, 0xAE, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0x66, 0x00; "calculates XOR correctly")]
  fn xor_immediate_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    memory.write(0x0000, 0xEE);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xAE, 0xAE, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0x66, 0x00; "calculates XOR correctly")]
  fn xor_indirect_hl_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::F, 0x10);

    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xAE);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFC, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x60; "half carry set correctly")]
  fn compare_reg_with_reg_a(a: u8, value: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0xBA);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFC, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x60; "half carry set correctly")]
  fn compare_immediate_with_reg_a(a: u8, value: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    memory.write(0x0000, 0xFE);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFC, 0xFC, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x60; "half carry set correctly")]
  fn compare_indirect_hl_with_reg_a(a: u8, value: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, a);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xBE);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFF, 0x00, 0x00, 0xA0; "zero flag set correctly and carry is not affected")]
  #[test_case(0x0F, 0x10, 0x10, 0x30; "half carry set correctly")]
  fn increment_reg(value: u8, result: u8, f_old: u8, f_new: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, f_old);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0x14);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::D), result);
    assert_eq!(cpu.read_register(Register::F), f_new);
  }

  #[test_case(0xFF, 0x00, 0x00, 0xA0; "zero flag set correctly and carry is not affected")]
  #[test_case(0x0F, 0x10, 0x10, 0x30; "half carry set correctly")]
  fn increment_indirect_hl(value: u8, result: u8, f_old: u8, f_new: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, f_old);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x34);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), result);
    assert_eq!(cpu.read_register(Register::F), f_new);
  }

  #[test_case(0x01, 0x00, 0x10, 0xD0; "zero flag set correctly and carry not affected")]
  #[test_case(0x10, 0x0F, 0x00, 0x60; "half carry set correctly")]
  fn decrement_reg(value: u8, result: u8, f_old: u8, f_new: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, f_old);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0x15);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::D), result);
    assert_eq!(cpu.read_register(Register::F), f_new);
  }

  #[test_case(0x01, 0x00, 0x10, 0xD0; "zero flag set correctly and carry not affected")]
  #[test_case(0x10, 0x0F, 0x00, 0x60; "half carry set correctly")]
  fn decrement_indirect_hl(value: u8, result: u8, f_old: u8, f_new: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, f_old);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0x35);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), result);
    assert_eq!(cpu.read_register(Register::F), f_new);
  }

  #[test_case(0xF01E, 0xF028, 0xE046, 0x80, 0x90; "carry set correctly and zero flag not affected")]
  #[test_case(0x1E1E, 0x2828, 0x4646, 0x80, 0xA0; "half carry set correctly")]
  fn add_reg_pair_to_reg_hl(hl: u16, value: u16, result: u16, f_old: u8, f_new: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, f_old);
    cpu.write_register_pair(Register::HL, hl);
    cpu.write_register_pair(Register::DE, value);
    memory.write(0x0000, 0x19);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::HL), result);
    assert_eq!(cpu.read_register(Register::F), f_new);
  }

  #[test_case(0xFFDA, 0x26, 0x0000, 0x30; "carry set correctly and zero flag set to zero")]
  #[test_case(0x0FDA, 0x26, 0x1000, 0x20; "half carry set correctly")]
  fn add_immediate_to_reg_sp(sp: u16, value: u8, result: u16, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::SP, sp);
    memory.write(0x0000, 0xE8);
    memory.write(0x0001, value);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::SP), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0xFFFF, 0x0000; "performs wrapping correctly")]
  #[test_case(0x0FDA, 0x0FDB; "increments correctly")]
  fn increment_reg_pair(sp: u16, result: u16) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, 0xF0);
    cpu.write_register_pair(Register::SP, sp);
    memory.write(0x0000, 0x33);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::SP), result);
    assert_eq!(cpu.read_register(Register::F), 0xF0);
  }

  #[test_case(0x0000, 0xFFFF; "performs wrapping correctly")]
  #[test_case(0x0FDA, 0x0FD9; "decrements correctly")]
  fn decrement_reg_pair(sp: u16, result: u16) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, 0xF0);
    cpu.write_register_pair(Register::SP, sp);
    memory.write(0x0000, 0x3B);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::SP), result);
    assert_eq!(cpu.read_register(Register::F), 0xF0);
  }

  #[test]
  fn rotate_reg_a_left() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0xCA);
    memory.write(0x0000, 0x07);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0x95);
    assert_eq!(cpu.read_register(Register::F), 0x10);
  }

  #[test_case(0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xCA, 0x95, 0x10; "rotates left correctly and sets carry")]
  fn rotate_reg_left(value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x02);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::D), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xCA, 0x95, 0x10; "rotates left correctly and sets carry")]
  fn rotate_indirect_hl_left(value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x06);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test]
  fn rotate_reg_a_right() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0x53);
    memory.write(0x0000, 0x0F);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0xA9);
    assert_eq!(cpu.read_register(Register::F), 0x10);
  }

  #[test_case(0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0x53, 0xA9, 0x10; "rotates right correctly and sets carry")]
  fn rotate_reg_right(value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x0A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::D), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }


  #[test_case(0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0x53, 0xA9, 0x10; "rotates right correctly and sets carry")]
  fn rotate_indirect_hl_right(value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x0E);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test]
  fn rotate_reg_a_left_through_carry() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0x4A);
    cpu.write_register(Register::F, 0x10);
    memory.write(0x0000, 0x17);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0x95);
    assert_eq!(cpu.read_register(Register::F), 0x00);
  }

  #[test_case(0x80, 0x00, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0x4A, 0x95, 0x10, 0x00; "rotates left correctly and sets carry")]
  fn rotate_reg_left_through_carry(value: u8, result: u8, old_f: u8, new_f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::D, value);
    cpu.write_register(Register::F, old_f);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x12);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::D), result);
    assert_eq!(cpu.read_register(Register::F), new_f);
  }

  #[test_case(0x80, 0x00, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0x4A, 0x95, 0x10, 0x00; "rotates left correctly and sets carry")]
  fn rotate_indirect_hl_left_through_carry(value: u8, result: u8, old_f: u8, new_f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    cpu.write_register(Register::F, old_f);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x16);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), result);
    assert_eq!(cpu.read_register(Register::F), new_f);
  }

  #[test]
  fn rotate_reg_a_right_through_carry() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0x52);
    cpu.write_register(Register::F, 0x10);
    memory.write(0x0000, 0x1F);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::A), 0xA9);
    assert_eq!(cpu.read_register(Register::F), 0x00);
  }

  #[test_case(0x01, 0x00, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0x52, 0xA9, 0x10, 0x00; "rotates right correctly and sets carry")]
  fn rotate_reg_right_through_carry(value: u8, result: u8, old_f: u8, new_f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::D, 0x52);
    cpu.write_register(Register::F, 0x10);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x1A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::D), 0xA9);
    assert_eq!(cpu.read_register(Register::F), 0x00);
  }

  #[test_case(0x01, 0x00, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0x52, 0xA9, 0x10, 0x00; "rotates right correctly and sets carry")]
  fn rotate_indirect_hl_right_through_carry(value: u8, result: u8, old_f: u8, new_f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    cpu.write_register(Register::F, old_f);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x1E);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), result);
    assert_eq!(cpu.read_register(Register::F), new_f);
  }

  #[test_case(0x80, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0xCA, 0x94, 0x10; "shifts left correctly and sets carry")]
  fn shift_reg_left(value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x22);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::D), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x80, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0xCA, 0x94, 0x10; "shifts left correctly and sets carry")]
  fn shift_indirect_hl_left(value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x26);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x01, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0x53, 0x29, 0x10; "shifts right correctly and sets carry")]
  fn shift_reg_right(value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x3A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::D), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x01, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0x53, 0x29, 0x10; "shifts right correctly and sets carry")]
  fn shift_indirect_hl_right(value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x3E);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x01, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0xA2, 0xD1, 0x00; "shifts right correctly")]
  fn shift_reg_right_arithmetic(value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x2A);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::D), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x01, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0xA2, 0xD1, 0x00; "shifts right correctly")]
  fn shift_indirect_hl_right_arithmetic(value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x2E);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xA6, 0x6A, 0x00; "swaps correctly")]
  fn swap_reg(value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::D, value);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x32);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::D), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test_case(0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xA6, 0x6A, 0x00; "swaps correctly")]
  fn swap_indirect_hl(value: u8, result: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xCB);
    memory.write(0x0001, 0x36);
    memory.write(0xABCD, value);
    cpu.execute(&mut memory);
    assert_eq!(memory.read(0xABCD), result);
    assert_eq!(cpu.read_register(Register::F), f);
  }

  #[test]
  fn get_reg_bit() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::D, 0xA5);
    let bits: Vec<(bool, u8)> = (0u8..8u8).map(|bit| {
      memory.write(2 * (bit), 0xCB);
      memory.write(2 * (bit) + 1, 0x42 | (bit << 3));
      cpu.execute(&mut memory);
      (!cpu.read_register(Register::F).get_bit(7), bit)
    }).collect();
    let result = u8::compose(&bits);
    assert_eq!(result, 0xA5);
    assert_eq!(cpu.read_register(Register::F), 0x20);
  }

  #[test]
  fn get_indirect_hl_bit() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0xABCD, 0xA5);
    let bits: Vec<(bool, u8)> = (0u8..8u8).map(|bit| {
      memory.write(2 * (bit), 0xCB);
      memory.write(2 * (bit) + 1, 0x46 | (bit << 3));
      cpu.execute(&mut memory);
      (!cpu.read_register(Register::F).get_bit(7), bit)
    }).collect();
    let result = u8::compose(&bits);
    assert_eq!(result, 0xA5);
    assert_eq!(cpu.read_register(Register::F), 0x20);
  }

  #[test]
  fn set_reg_bit() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, 0xB0);
    [0, 2, 5, 7].iter().enumerate().for_each(|(index, bit)| {
      memory.write(2 * (index), 0xCB);
      memory.write(2 * (index) + 1, 0xC2 | (bit << 3));
      cpu.execute(&mut memory);
    });
    assert_eq!(cpu.read_register(Register::D), 0xA5);
    assert_eq!(cpu.read_register(Register::F), 0xB0);
  }

  #[test]
  fn set_indirect_hl_bit() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    cpu.write_register(Register::F, 0xB0);
    [0, 2, 5, 7].iter().enumerate().for_each(|(index, bit)| {
      memory.write(2 * (index), 0xCB);
      memory.write(2 * (index) + 1, 0xC6 | (bit << 3));
      cpu.execute(&mut memory);
    });
    assert_eq!(memory.read(0xABCD), 0xA5);
    assert_eq!(cpu.read_register(Register::F), 0xB0);
  }

  #[test]
  fn reset_reg_bit() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::D, 0xFF);
    cpu.write_register(Register::F, 0xB0);
    [1, 3, 4, 6].iter().enumerate().for_each(|(index, bit)| {
      memory.write(2 * (index), 0xCB);
      memory.write(2 * (index) + 1, 0x82 | (bit << 3));
      cpu.execute(&mut memory);
    });
    assert_eq!(cpu.read_register(Register::D), 0xA5);
    assert_eq!(cpu.read_register(Register::F), 0xB0);
  }

  #[test]
  fn reset_indirect_hl_bit() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0xABCD, 0xFF);
    cpu.write_register(Register::F, 0xB0);
    [1, 3, 4, 6].iter().enumerate().for_each(|(index, bit)| {
      memory.write(2 * (index), 0xCB);
      memory.write(2 * (index) + 1, 0x86 | (bit << 3));
      cpu.execute(&mut memory);
    });
    assert_eq!(memory.read(0xABCD), 0xA5);
    assert_eq!(cpu.read_register(Register::F), 0xB0);
  }

  #[test]
  fn jump() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    memory.write(0x0000, 0xC3);
    memory.write(0x0001, 0xCD);
    memory.write(0x0002, 0xAB);
    cpu.execute(&mut memory);

    assert_eq!(cpu.read_register_pair(Register::PC), 0xABCD);
  }

  #[test_case(0x00, 0x70; "jumps when zero flag not set")]
  #[test_case(0x01, 0x80; "jumps when zero flag set")]
  #[test_case(0x02, 0xE0; "jumps when carry not set")]
  #[test_case(0x03, 0x10; "jumps when carry set")]
  fn jump_conditional(condition: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, !f);
    memory.write(0x0000, 0xC2 | (condition << 3));
    memory.write(0x0001, 0xCD);
    memory.write(0x0002, 0xAB);
    memory.write(0x0003, 0xC2 | (condition << 3));
    memory.write(0x0004, 0xCD);
    memory.write(0x0005, 0xAB);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::PC), 0x0003);

    cpu.write_register(Register::F, f);
    cpu.execute(&mut memory);

    assert_eq!(cpu.read_register_pair(Register::PC), 0xABCD);
  }

  #[test]
  fn jump_relative() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    memory.write(0x0000, 0x18);
    memory.write(0x0001, 0x08);
    memory.write(0x000A, 0x18);
    memory.write(0x000B, 0xFC);
    cpu.execute(&mut memory);
    cpu.execute(&mut memory);

    assert_eq!(cpu.read_register_pair(Register::PC), 0x0008);
  }

  #[test_case(0x00, 0x70; "jumps when zero flag not set")]
  #[test_case(0x01, 0x80; "jumps when zero flag set")]
  #[test_case(0x02, 0xE0; "jumps when carry not set")]
  #[test_case(0x03, 0x10; "jumps when carry set")]
  fn jump_conditional_relative(condition: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, !f);
    memory.write(0x0000, 0x20 | (condition << 3));
    memory.write(0x0001, 0x08);
    memory.write(0x0002, 0x20 | (condition << 3));
    memory.write(0x0003, 0x08);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::PC), 0x0002);

    cpu.write_register(Register::F, f);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::PC), 0x000C);
  }

  #[test]
  fn jump_indirect_hl() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::HL, 0xABCD);
    memory.write(0x0000, 0xE9);
    cpu.execute(&mut memory);

    assert_eq!(cpu.read_register_pair(Register::PC), 0xABCD);
  }

  #[test]
  fn call() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::SP, 0xFFFE);
    cpu.write_register_pair(Register::PC, 0x1234);
    memory.write(0x1234, 0xCD);
    memory.write(0x1235, 0xCD);
    memory.write(0x1236, 0xAB);
    cpu.execute(&mut memory);

    assert_eq!(cpu.read_register_pair(Register::SP), 0xFFFC);
    assert_eq!(memory.read(0xFFFD), 0x12);
    assert_eq!(memory.read(0xFFFC), 0x37);
    assert_eq!(cpu.read_register_pair(Register::PC), 0xABCD);
  }

  #[test_case(0x00, 0x70; "calls when zero flag not set")]
  #[test_case(0x01, 0x80; "calls when zero flag set")]
  #[test_case(0x02, 0xE0; "calls when carry not set")]
  #[test_case(0x03, 0x10; "calls when carry set")]
  fn call_conditional(condition: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::SP, 0xFFFE);
    cpu.write_register_pair(Register::PC, 0x1234);
    cpu.write_register(Register::F, !f);
    memory.write(0x1234, 0xC4 | (condition << 3));
    memory.write(0x1235, 0xCD);
    memory.write(0x1236, 0xAB);
    memory.write(0x1237, 0xC4 | (condition << 3));
    memory.write(0x1238, 0xCD);
    memory.write(0x1239, 0xAB);

    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::PC), 0x1237);

    cpu.write_register(Register::F, f);
    cpu.execute(&mut memory);

    assert_eq!(cpu.read_register_pair(Register::PC), 0xABCD);
    assert_eq!(cpu.read_register_pair(Register::SP), 0xFFFC);
    assert_eq!(memory.read(0xFFFD), 0x12);
    assert_eq!(memory.read(0xFFFC), 0x3A);
  }

  #[test]
  fn return_from_call() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::SP, 0xFFFE);
    cpu.write_register_pair(Register::PC, 0x1234);
    memory.write(0x1234, 0xCD);
    memory.write(0x1235, 0xCD);
    memory.write(0x1236, 0xAB);
    memory.write(0xABCD, 0xC9);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::PC), 0xABCD);

    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::PC), 0x1237);
    assert_eq!(cpu.read_register_pair(Register::SP), 0xFFFE);
  }

  #[test]
  fn return_from_interrupt() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::SP, 0xFFFE);
    cpu.write_register_pair(Register::PC, 0x1234);
    memory.write(0x1234, 0xCD);
    memory.write(0x1235, 0xCD);
    memory.write(0x1236, 0xAB);
    memory.write(0xABCD, 0xF3);
    memory.write(0xABCE, 0xD9);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::PC), 0xABCD);

    cpu.execute(&mut memory);
    assert_eq!(cpu.ime, false);

    cpu.execute(&mut memory);
    assert_eq!(cpu.ime, true);
    assert_eq!(cpu.read_register_pair(Register::PC), 0x1237);
    assert_eq!(cpu.read_register_pair(Register::SP), 0xFFFE);
  }

  #[test_case(0x00, 0x70; "returns when zero flag not set")]
  #[test_case(0x01, 0x80; "returns when zero flag set")]
  #[test_case(0x02, 0xE0; "returns when carry not set")]
  #[test_case(0x03, 0x10; "returns when carry set")]
  fn return_conditionally(condition: u8, f: u8) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::SP, 0xFFFE);
    cpu.write_register_pair(Register::PC, 0x1234);
    memory.write(0x1234, 0xCD);
    memory.write(0x1235, 0xCD);
    memory.write(0x1236, 0xAB);
    memory.write(0xABCD, 0xC0 | (condition << 3));
    memory.write(0xABCE, 0xC0 | (condition << 3));
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::PC), 0xABCD);

    cpu.write_register(Register::F, !f);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::PC), 0xABCE);

    cpu.write_register(Register::F, f);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register_pair(Register::PC), 0x1237);
    assert_eq!(cpu.read_register_pair(Register::SP), 0xFFFE);
  }

  #[test_case(0, 0x0000; "restart to 0x0000")]
  #[test_case(1, 0x0008; "restart to 0x0008")]
  #[test_case(2, 0x0010; "restart to 0x0010")]
  #[test_case(3, 0x0018; "restart to 0x0018")]
  #[test_case(4, 0x0020; "restart to 0x0020")]
  #[test_case(5, 0x0028; "restart to 0x0028")]
  #[test_case(6, 0x0030; "restart to 0x0030")]
  #[test_case(7, 0x0038; "restart to 0x0038")]
  fn restart(operand: u8, address: u16) {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register_pair(Register::SP, 0xFFFE);
    cpu.write_register_pair(Register::PC, 0x1234);
    memory.write(0x1234, 0xC7 | (operand << 3));
    cpu.execute(&mut memory);

    assert_eq!(cpu.read_register_pair(Register::SP), 0xFFFC);
    assert_eq!(memory.read(0xFFFD), 0x12);
    assert_eq!(memory.read(0xFFFC), 0x35);
    assert_eq!(cpu.read_register_pair(Register::PC), address);
  }

  #[test]
  fn decimal_adjust_reg_a() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    let mut instruction_index: usize = 0;
    (0u8..99u8).for_each(|x| {
      (0u8..99u8).for_each(|y| {
        let sum = x + y;
        let difference = 100 + x - y;
        let a = (x % 10) | ((x / 10) << 4);
        let d = (y % 10) | ((y / 10) << 4);

        cpu.write_register(Register::A, a);
        cpu.write_register(Register::D, d);
        memory.write(instruction_index, 0x82);
        instruction_index += 1;
        cpu.execute(&mut memory);
        memory.write(instruction_index, 0x27);
        instruction_index += 1;
        cpu.execute(&mut memory);
        let result_bcd_sum = cpu.read_register(Register::A);
        let result_decimal_sum = ((result_bcd_sum & 0xF0) >> 4) * 10 + (result_bcd_sum & 0x0F);
        assert_eq!(result_decimal_sum, sum % 100);
        let f = u8::compose(&[(sum % 100 == 0, 7), (sum >= 100, 4)]);
        assert_eq!(cpu.read_register(Register::F) & 0xB0, f);

        cpu.write_register(Register::A, a);
        cpu.write_register(Register::D, d);
        memory.write(instruction_index, 0x92);
        instruction_index += 1;
        cpu.execute(&mut memory);
        memory.write(instruction_index, 0x27);
        instruction_index += 1;
        cpu.execute(&mut memory);
        let result_bcd_diff = cpu.read_register(Register::A);
        let result_decimal_diff = ((result_bcd_diff & 0xF0) >> 4) * 10 + (result_bcd_diff & 0x0F);
        let f = u8::compose(&[(difference % 100 == 0, 7), (difference < 100, 4)]);
        assert_eq!(cpu.read_register(Register::F) & 0xB0, f);
        assert_eq!(result_decimal_diff, difference % 100);
      })
    })
  }

  #[test]
  fn ones_complement_reg_a() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::A, 0xA6);
    cpu.write_register(Register::F, 0x90);
    memory.write(0x0000, 0x2F);
    cpu.execute(&mut memory);

    assert_eq!(cpu.read_register(Register::A), 0x59);
    assert_eq!(cpu.read_register(Register::F), 0xF0);
  }

  #[test]
  fn flip_carry() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, 0x80);
    memory.write(0x0000, 0x3F);
    memory.write(0x0001, 0x3F);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::F), 0x90);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::F), 0x80);
  }

  #[test]
  fn set_carry() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.write_register(Register::F, 0x80);
    memory.write(0x0000, 0x37);
    memory.write(0x0000, 0x37);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::F), 0x90);
    cpu.execute(&mut memory);
    assert_eq!(cpu.read_register(Register::F), 0x90);
  }

  #[test]
  fn disable_enable_interrupts() {
    let mut memory = Rc::new(RefCell::new(MockMemory::new()));
    let mut cpu = CPU::new(memory);
    cpu.ime = true;
    memory.write(0x0000, 0xF3);
    memory.write(0x0001, 0xFB);
    cpu.execute(&mut memory);
    assert_eq!(cpu.ime, false);
    cpu.execute(&mut memory);
    assert_eq!(cpu.ime, true);
  }
}
