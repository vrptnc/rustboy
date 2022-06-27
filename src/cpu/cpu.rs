use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use byteorder::{LittleEndian, ReadBytesExt};
use crate::cpu::opcode::Opcode;
use crate::cpu::register::{ByteRegister, Registers, WordRegister};
use crate::memory::memory::{Memory, MemoryRef};
use crate::time::time::ClockAware;
use crate::util::bit_util::BitUtil;

#[derive(Copy, Clone)]
enum ByteLocation {
  Value(u8),
  Register(ByteRegister),
  ByteBuffer,
  LowerAddressBuffer,
  UpperAddressBuffer,
  LowerWordBuffer,
  UpperWordBuffer,
  NextMemoryByte,
  MemoryReferencedByAddressBuffer,
  MemoryReferencedByRegister(WordRegister),
}

#[derive(Copy, Clone)]
enum WordLocation {
  Value(u16),
  Register(WordRegister),
  WordBuffer,
  AddressBuffer,
}

#[derive(Copy, Clone)]
struct ByteArithmeticParams {
  first: ByteLocation,
  second: ByteLocation,
  destination: ByteLocation,
  use_carry: bool,
  flag_mask: u8,
}

#[derive(Copy, Clone)]
struct WordArithmeticParams {
  first: WordLocation,
  second: WordLocation,
  destination: WordLocation,
  flag_mask: u8,
}

struct InstructionContext {
  opcode: Opcode,
  byte_buffer: u8,
  word_buffer: u16,
  address_buffer: u16,
}

type Operation = Box<dyn FnOnce(&mut CPU)>;

pub struct CPU {
  context: InstructionContext,
  operations: VecDeque<Operation>,
  memory: MemoryRef,
  registers: Registers,
  ime: bool,
}

impl CPU {
  pub fn new(memory: MemoryRef) -> CPU {
    CPU {
      memory,
      context: InstructionContext {
        opcode: Opcode(0),
        byte_buffer: 0u8,
        word_buffer: 0u16,
        address_buffer: 0u16,
      },
      operations: VecDeque::with_capacity(5),
      registers: Registers::new(),
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

  fn fetch_and_execute_instruction(&mut self) {
    let opcode_value = self.read_next_byte();
    self.context.opcode = Opcode(opcode_value);
    match opcode_value {
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
      0x27 => self.decimal_adjust_reg_a(),
      0x28 => self.jump_conditional_relative(),
      0x29 => self.add_reg_pair_to_reg_hl(),
      0x2A => self.indirect_hl_to_reg_a_ld_and_increment(),
      0x2B => self.decrement_reg_pair(),
      0x2C => self.increment_reg(),
      0x2D => self.decrement_reg(),
      0x2E => self.immediate_to_reg_ld(),
      0x2F => self.ones_complement_reg_a(),
      0x30 => self.jump_conditional_relative(),
      0x31 => self.immediate_to_reg_pair_ld(),
      0x32 => self.reg_a_to_indirect_hl_ld_and_decrement(),
      0x33 => self.increment_reg_pair(),
      0x34 => self.increment_indirect_hl(),
      0x35 => self.decrement_indirect_hl(),
      0x36 => self.immediate_to_indirect_ld(),
      0x37 => self.set_carry_flag(),
      0x38 => self.jump_conditional_relative(),
      0x39 => self.add_reg_pair_to_reg_hl(),
      0x3A => self.indirect_hl_to_reg_a_ld_and_decrement(),
      0x3B => self.decrement_reg_pair(),
      0x3C => self.increment_reg(),
      0x3D => self.decrement_reg(),
      0x3E => self.immediate_to_reg_ld(),
      0x3F => self.flip_carry_flag(),
      0x40..=0x45 => self.reg_to_reg_ld(),
      0x46 => self.indirect_to_reg_ld(),
      0x47..=0x4D => self.reg_to_reg_ld(),
      0x4E => self.indirect_to_reg_ld(),
      0x4F => self.reg_to_reg_ld(),
      0x50..=0x55 => self.reg_to_reg_ld(),
      0x56 => self.indirect_to_reg_ld(),
      0x57..=0x5D => self.reg_to_reg_ld(),
      0x5E => self.indirect_to_reg_ld(),
      0x5F => self.reg_to_reg_ld(),
      0x60..=0x65 => self.reg_to_reg_ld(),
      0x66 => self.indirect_to_reg_ld(),
      0x67..=0x6D => self.reg_to_reg_ld(),
      0x6E => self.indirect_to_reg_ld(),
      0x6F => self.reg_to_reg_ld(),
      0x70..=0x75 => self.reg_to_indirect_ld(),
      0x76 => self.halt(),
      0x77 => self.reg_to_indirect_ld(),
      0x78..=0x7D => self.reg_to_reg_ld(),
      0x7E => self.indirect_to_reg_ld(),
      0x7F => self.reg_to_reg_ld(),
      0x80..=0x85 => self.add_reg_to_reg_a_and_write_to_reg_a(false),
      0x86 => self.add_indirect_hl_to_reg_a_and_write_to_reg_a(false),
      0x87 => self.add_reg_to_reg_a_and_write_to_reg_a(false),
      0x88..=0x8D => self.add_reg_to_reg_a_and_write_to_reg_a(true),
      0x8E => self.add_indirect_hl_to_reg_a_and_write_to_reg_a(true),
      0x8F => self.add_reg_to_reg_a_and_write_to_reg_a(true),
      0x90..=0x95 => self.subtract_reg_from_reg_a_and_write_to_reg_a(false),
      0x96 => self.subtract_indirect_hl_from_reg_a_and_write_to_reg_a(false),
      0x97 => self.subtract_reg_from_reg_a_and_write_to_reg_a(false),
      0x98..=0x9D => self.subtract_reg_from_reg_a_and_write_to_reg_a(true),
      0x9E => self.subtract_indirect_hl_from_reg_a_and_write_to_reg_a(true),
      0x9F => self.subtract_reg_from_reg_a_and_write_to_reg_a(true),
      0xA0..=0xA5 => self.and_reg_with_reg_a_and_write_to_reg_a(),
      0xA6 => self.and_indirect_hl_with_reg_a_and_write_to_reg_a(),
      0xA7 => self.and_reg_with_reg_a_and_write_to_reg_a(),
      0xA8..=0xAD => self.xor_reg_with_reg_a_and_write_to_reg_a(),
      0xAE => self.xor_indirect_hl_with_reg_a_and_write_to_reg_a(),
      0xAF => self.xor_reg_with_reg_a_and_write_to_reg_a(),
      0xB0..=0xB5 => self.or_reg_with_reg_a_and_write_to_reg_a(),
      0xB6 => self.or_indirect_hl_with_reg_a_and_write_to_reg_a(),
      0xB7 => self.or_reg_with_reg_a_and_write_to_reg_a(),
      0xB8..=0xBD => self.compare_reg_with_reg_a(),
      0xBE => self.compare_indirect_hl_with_reg_a(),
      0xBF => self.compare_reg_with_reg_a(),
      0xC0 => self.return_conditionally(),
      0xC1 => self.pop_stack_to_reg_pair(),
      0xC2 => self.jump_conditional(),
      0xC3 => self.jump(),
      0xC4 => self.call_conditional(),
      0xC5 => self.push_reg_pair_to_stack(),
      0xC6 => self.add_immediate_to_reg_a_and_write_to_reg_a(false),
      0xC7 => self.restart(),
      0xC8 => self.return_conditionally(),
      0xC9 => self.return_from_call(),
      0xCA => self.jump_conditional(),
      0xCB => self.execute_cb(),
      0xCC => self.call_conditional(),
      0xCD => self.call(),
      0xCE => self.add_immediate_to_reg_a_and_write_to_reg_a(true),
      0xCF => self.restart(),
      0xD0 => self.return_conditionally(),
      0xD1 => self.pop_stack_to_reg_pair(),
      0xD2 => self.jump_conditional(),
      0xD4 => self.call_conditional(),
      0xD5 => self.push_reg_pair_to_stack(),
      0xD6 => self.subtract_immediate_from_reg_a_and_write_to_reg_a(false),
      0xD7 => self.restart(),
      0xD8 => self.return_conditionally(),
      0xD9 => self.return_from_interrupt(),
      0xDA => self.jump_conditional(),
      0xDC => self.call_conditional(),
      0xDE => self.subtract_immediate_from_reg_a_and_write_to_reg_a(true),
      0xDF => self.restart(),
      0xE0 => self.reg_a_to_immediate_indirect_with_offset_ld(),
      0xE1 => self.pop_stack_to_reg_pair(),
      0xE2 => self.reg_a_to_indirect_c_ld(),
      0xE5 => self.push_reg_pair_to_stack(),
      0xE6 => self.and_immediate_with_reg_a_and_write_to_reg_a(),
      0xE7 => self.restart(),
      0xE8 => self.add_immediate_to_reg_sp(),
      0xE9 => self.jump_to_indirect_hl(),
      0xEA => self.reg_a_to_immediate_indirect_ld(),
      0xEE => self.xor_immediate_with_reg_a_and_write_to_reg_a(),
      0xEF => self.restart(),
      0xF0 => self.immediate_indirect_with_offset_to_reg_a_ld(),
      0xF1 => self.pop_stack_to_reg_pair(),
      0xF2 => self.indirect_c_with_offset_to_reg_a_ld(),
      0xF3 => self.disable_interrupts(),
      0xF5 => self.push_reg_pair_to_stack(),
      0xF6 => self.or_immediate_with_reg_a_and_write_to_reg_a(),
      0xF7 => self.restart(),
      0xF8 => self.reg_sp_plus_signed_immediate_to_hl_ld(),
      0xF9 => self.reg_hl_to_reg_sp_ld(),
      0xFA => self.immediate_indirect_to_reg_a_ld(),
      0xFB => self.enable_interrupts(),
      0xFE => self.compare_immediate_with_reg_a(),
      0xFF => self.restart(),
      _ => panic!("Unknown opcode"),
    };
  }

  fn execute_cb(&mut self) {
    self.operations.push_back(Box::new(|this| {
      let opcode_value = this.read_next_byte();
      this.context.opcode = Opcode(opcode_value);
      match opcode_value {
        0x00..=0x05 => this.rotate_reg_left(),
        0x06 => this.rotate_indirect_hl_left(),
        0x07 => this.rotate_reg_left(),
        0x08..=0x0D => this.rotate_reg_right(),
        0x0E => this.rotate_indirect_hl_right(),
        0x0F => this.rotate_reg_right(),
        0x10..=0x15 => this.rotate_reg_left_through_carry(),
        0x16 => this.rotate_indirect_hl_left_through_carry(),
        0x17 => this.rotate_reg_left_through_carry(),
        0x18..=0x1D => this.rotate_reg_right_through_carry(),
        0x1E => this.rotate_indirect_hl_right_through_carry(),
        0x1F => this.rotate_reg_right_through_carry(),
        0x20..=0x25 => this.shift_reg_left(),
        0x26 => this.shift_indirect_hl_left(),
        0x27 => this.shift_reg_left(),
        0x28..=0x2D => this.shift_reg_right_arithmetic(),
        0x2E => this.shift_indirect_hl_right_arithmetic(),
        0x2F => this.shift_reg_right_arithmetic(),
        0x30..=0x35 => this.swap_reg(),
        0x36 => this.swap_indirect_hl(),
        0x37 => this.swap_reg(),
        0x38..=0x3D => this.shift_reg_right(),
        0x3E => this.shift_indirect_hl_right(),
        0x3F => this.shift_reg_right(),
        0x40..=0x45 => this.get_reg_bit(),
        0x46 => this.get_indirect_hl_bit(),
        0x47..=0x4D => this.get_reg_bit(),
        0x4E => this.get_indirect_hl_bit(),
        0x4F..=0x55 => this.get_reg_bit(),
        0x56 => this.get_indirect_hl_bit(),
        0x57..=0x5D => this.get_reg_bit(),
        0x5E => this.get_indirect_hl_bit(),
        0x5F..=0x65 => this.get_reg_bit(),
        0x66 => this.get_indirect_hl_bit(),
        0x67..=0x6D => this.get_reg_bit(),
        0x6E => this.get_indirect_hl_bit(),
        0x6F..=0x75 => this.get_reg_bit(),
        0x76 => this.get_indirect_hl_bit(),
        0x77..=0x7D => this.get_reg_bit(),
        0x7E => this.get_indirect_hl_bit(),
        0x7F => this.get_reg_bit(),
        0x80..=0x85 => this.reset_reg_bit(),
        0x86 => this.reset_indirect_hl_bit(),
        0x87..=0x8D => this.reset_reg_bit(),
        0x8E => this.reset_indirect_hl_bit(),
        0x8F..=0x95 => this.reset_reg_bit(),
        0x96 => this.reset_indirect_hl_bit(),
        0x97..=0x9D => this.reset_reg_bit(),
        0x9E => this.reset_indirect_hl_bit(),
        0x9F..=0xA5 => this.reset_reg_bit(),
        0xA6 => this.reset_indirect_hl_bit(),
        0xA7..=0xAD => this.reset_reg_bit(),
        0xAE => this.reset_indirect_hl_bit(),
        0xAF..=0xB5 => this.reset_reg_bit(),
        0xB6 => this.reset_indirect_hl_bit(),
        0xB7..=0xBD => this.reset_reg_bit(),
        0xBE => this.reset_indirect_hl_bit(),
        0xBF => this.reset_reg_bit(),
        0xC0..=0xC5 => this.set_reg_bit(),
        0xC6 => this.set_indirect_hl_bit(),
        0xC7..=0xCD => this.set_reg_bit(),
        0xCE => this.set_indirect_hl_bit(),
        0xCF..=0xD5 => this.set_reg_bit(),
        0xD6 => this.set_indirect_hl_bit(),
        0xD7..=0xDD => this.set_reg_bit(),
        0xDE => this.set_indirect_hl_bit(),
        0xDF..=0xE5 => this.set_reg_bit(),
        0xE6 => this.set_indirect_hl_bit(),
        0xE7..=0xED => this.set_reg_bit(),
        0xEE => this.set_indirect_hl_bit(),
        0xEF..=0xF5 => this.set_reg_bit(),
        0xF6 => this.set_indirect_hl_bit(),
        0xF7..=0xFD => this.set_reg_bit(),
        0xFE => this.set_indirect_hl_bit(),
        0xFF => this.set_reg_bit(),
        _ => panic!("Unknown opcode"),
      };
    }));
  }

  fn read_next_byte(&mut self) -> u8 {
    let address = self.registers.read_word(WordRegister::PC);
    self.registers.write_word(WordRegister::PC, address + 1);
    self.memory.borrow().read(address)
  }

  fn combine_operations(operation1: Operation, operation2: Operation) -> Operation {
    Box::new(|this| {
      operation1(this);
      operation2(this);
    })
  }

  fn read_byte(&mut self, location: ByteLocation) -> u8 {
    match location {
      ByteLocation::Value(value) => value,
      ByteLocation::Register(register) => self.registers.read_byte(register),
      ByteLocation::ByteBuffer => self.context.byte_buffer,
      ByteLocation::LowerAddressBuffer => self.context.address_buffer as u8,
      ByteLocation::UpperAddressBuffer => (self.context.address_buffer >> 8) as u8,
      ByteLocation::LowerWordBuffer => self.context.word_buffer as u8,
      ByteLocation::UpperWordBuffer => (self.context.word_buffer >> 8) as u8,
      ByteLocation::MemoryReferencedByAddressBuffer => self.memory.borrow().read(self.context.address_buffer),
      ByteLocation::MemoryReferencedByRegister(register) => self.memory.borrow().read(self.registers.read_word(register)),
      ByteLocation::NextMemoryByte => self.read_next_byte(),
    }
  }

  fn write_byte(&mut self, location: ByteLocation, value: u8) {
    match location {
      ByteLocation::Register(register) => self.registers.write_byte(register, value),
      ByteLocation::ByteBuffer => self.context.byte_buffer = value,
      ByteLocation::LowerAddressBuffer => self.context.address_buffer = (self.context.address_buffer & 0xFF00) + (value as u16),
      ByteLocation::UpperAddressBuffer => self.context.address_buffer = (self.context.address_buffer & 0x00FF) + ((value as u16) << 8),
      ByteLocation::LowerWordBuffer => self.context.word_buffer = (self.context.word_buffer & 0xFF00) + (value as u16),
      ByteLocation::UpperWordBuffer => self.context.word_buffer = (self.context.word_buffer & 0x00FF) + ((value as u16) << 8),
      ByteLocation::MemoryReferencedByAddressBuffer => self.memory.borrow_mut().write(self.context.address_buffer, value),
      ByteLocation::MemoryReferencedByRegister(register) => self.memory.borrow_mut().write(self.registers.read_word(register), value),
      ByteLocation::NextMemoryByte => panic!("Can't write byte to next memory location"),
      ByteLocation::Value(_) => panic!("Can't write to passed value")
    }
  }

  fn read_word(&mut self, location: WordLocation) -> u16 {
    match location {
      WordLocation::Value(value) => value,
      WordLocation::Register(register) => self.registers.read_word(register),
      WordLocation::WordBuffer => self.context.word_buffer,
      WordLocation::AddressBuffer => self.context.address_buffer,
    }
  }

  fn write_word(&mut self, location: WordLocation, value: u16) {
    match location {
      WordLocation::Register(register) => self.registers.write_word(register, value),
      WordLocation::WordBuffer => self.context.word_buffer = value,
      WordLocation::AddressBuffer => self.context.address_buffer = value,
      WordLocation::Value(_) => panic!("Can't write to passed value")
    }
  }

  fn noop() -> Operation {
    Box::new(|this| {})
  }

  fn move_byte(source: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(move |this| {
      let byte = this.read_byte(source);
      this.write_byte(destination, byte);
    })
  }

  fn move_word(source: WordLocation, destination: WordLocation) -> Operation {
    Box::new(move |this| {
      let word = this.read_word(source);
      this.write_word(destination, word);
    })
  }

  fn add_bytes(params: ByteArithmeticParams) -> Operation {
    Box::new(move |this| {
      let first_value = this.read_byte(params.first) as u16;
      let second_value = this.read_byte(params.second) as u16;
      let carry = if params.use_carry { this.registers.read_byte(ByteRegister::F).get_bit(4) as u16 } else { 0u16 };
      let result = first_value + second_value + carry;
      let carry_result = first_value ^ second_value ^ result;
      let truncated_result = result as u8;
      let zero = truncated_result == 0;
      if params.flag_mask != 0 {
        let flag =
          ((zero as u8) << 7) |
            ((carry_result.get_bit(4) as u8) << 5) |
            ((carry_result.get_bit(8) as u8) << 4);
        this.registers.write_byte_masked(ByteRegister::F, flag, params.flag_mask);
      }
      this.write_byte(params.destination, truncated_result);
    })
  }

  fn add_words(params: WordArithmeticParams) -> Operation {
    Box::new(move |this| {
      let first_value = this.read_word(params.first);
      let second_value = this.read_word(params.second);
      let le_bytes1 = first_value.to_le_bytes();
      let le_bytes2 = second_value.to_le_bytes();
      let (result1, carry1) = le_bytes1[0].overflowing_add(le_bytes2[0]);
      let result2 = (le_bytes1[1] as u16) + (le_bytes2[1] as u16) + (carry1 as u16);
      let carry_result2 = (le_bytes1[1] as u16) ^ (le_bytes2[1] as u16) ^ result2;
      let result = (&[result1, result2 as u8][..]).read_u16::<LittleEndian>().unwrap();
      let zero = result == 0;
      if params.flag_mask != 0 {
        let flag =
          ((zero as u8) << 7) |
            ((carry_result2.get_bit(4) as u8) << 5) |
            ((carry_result2.get_bit(8) as u8) << 4);
        this.registers.write_byte_masked(ByteRegister::F, flag, params.flag_mask);
      }
      this.write_word(params.destination, result);
    })
  }

  fn subtract_bytes(params: ByteArithmeticParams) -> Operation {
    Box::new(move |this| {
      let first_value = this.read_byte(params.first);
      let second_value = this.read_byte(params.second);
      let borrow = if params.use_carry { this.registers.read_byte(ByteRegister::F).get_bit(4) as u16 } else { 0u16 };
      let result = 0x100u16 + (first_value as u16) - (second_value as u16) - borrow;
      let borrow_result = (0x100u16 + first_value as u16) ^ (second_value as u16) ^ result;
      let truncated_result = result as u8;
      let zero = truncated_result == 0;
      if params.flag_mask != 0 {
        let flag =
          ((zero as u8) << 7) |
            (1u8 << 6) |
            ((borrow_result.get_bit(4) as u8) << 5) |
            ((borrow_result.get_bit(8) as u8) << 4);
        this.registers.write_byte_masked(ByteRegister::F, flag, params.flag_mask);
      }
      this.write_byte(params.destination, truncated_result);
    })
  }

  fn and_bytes(first: ByteLocation, second: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(move |this| {
      let first_value = this.read_byte(first);
      let second_value = this.read_byte(second);
      let result = first_value & second_value;
      let zero = result == 0;
      let flag = ((zero as u8) << 7) | (1u8 << 5);
      this.registers.write_byte(ByteRegister::F, flag);
      this.write_byte(destination, result);
    })
  }

  fn or_bytes(first: ByteLocation, second: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(move |this| {
      let first_value = this.read_byte(first);
      let second_value = this.read_byte(second);
      let result = first_value | second_value;
      let flag = if result == 0 { 0x80u8 } else { 0x00u8 };
      this.registers.write_byte(ByteRegister::F, flag);
      this.write_byte(destination, result);
    })
  }

  fn xor_bytes(first: ByteLocation, second: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(move |this| {
      let first_value = this.read_byte(first);
      let second_value = this.read_byte(second);
      let result = first_value ^ second_value;
      let flag = if result == 0 { 0x80u8 } else { 0x00u8 };
      this.registers.write_byte(ByteRegister::F, flag);
      this.write_byte(destination, result);
    })
  }

  fn rotate_byte_left(source: ByteLocation, destination: ByteLocation, unset_zero: bool) -> Operation {
    Box::new(move |this| {
      let value = this.read_byte(source);
      let result = value.rotate_left(1);
      let zero = !unset_zero && result == 0;
      let flag =
        ((zero as u8) << 7) | ((value.get_bit(7) as u8) << 4);
      this.registers.write_byte(ByteRegister::F, flag);
      this.write_byte(destination, result);
    })
  }

  fn rotate_byte_left_through_carry(source: ByteLocation, destination: ByteLocation, unset_zero: bool) -> Operation {
    Box::new(move |this| {
      let value = this.read_byte(source);
      let carry = this.registers.read_byte(ByteRegister::F).get_bit(4);
      let result = (value << 1) | (carry as u8);
      let zero = !unset_zero && result == 0;
      let flag =
        ((zero as u8) << 7) | ((value.get_bit(7) as u8) << 4);
      this.registers.write_byte(ByteRegister::F, flag);
      this.write_byte(destination, result);
    })
  }

  fn rotate_byte_right(source: ByteLocation, destination: ByteLocation, unset_zero: bool) -> Operation {
    Box::new(move |this| {
      let value = this.read_byte(source);
      let result = value.rotate_right(1);
      let zero = !unset_zero && result == 0;
      let flag =
        ((zero as u8) << 7) | ((value.get_bit(0) as u8) << 4);
      this.registers.write_byte(ByteRegister::F, flag);
      this.write_byte(destination, result);
    })
  }

  fn rotate_byte_right_through_carry(source: ByteLocation, destination: ByteLocation, unset_zero: bool) -> Operation {
    Box::new(move |this| {
      let value = this.read_byte(source);
      let carry = this.registers.read_byte(ByteRegister::F).get_bit(4);
      let result = (value >> 1) | (if carry { 0x80u8 } else { 0x00u8 });
      let zero = !unset_zero && result == 0;
      let flag =
        ((zero as u8) << 7) | ((value.get_bit(0) as u8) << 4);
      this.registers.write_byte(ByteRegister::F, flag);
      this.write_byte(destination, result);
    })
  }

  fn shift_byte_left(source: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(move |this| {
      let value = this.read_byte(source);
      let result = value << 1;
      let zero = result == 0;
      let flag =
        ((zero as u8) << 7) | ((value.get_bit(7) as u8) << 4);
      this.registers.write_byte(ByteRegister::F, flag);
      this.write_byte(destination, result);
    })
  }

  fn shift_byte_right(source: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(move |this| {
      let value = this.read_byte(source);
      let result = value >> 1;
      let zero = result == 0;
      let flag =
        ((zero as u8) << 7) | ((value.get_bit(0) as u8) << 4);
      this.registers.write_byte(ByteRegister::F, flag);
      this.write_byte(destination, result);
    })
  }

  fn shift_byte_right_arithmetic(source: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(move |this| {
      let value = this.read_byte(source);
      let result = (value >> 1) | (value & 0x80);
      let zero = result == 0;
      let flag =
        ((zero as u8) << 7) | ((value.get_bit(0) as u8) << 4);
      this.registers.write_byte(ByteRegister::F, flag);
      this.write_byte(destination, result);
    })
  }

  fn swap_byte(source: ByteLocation, destination: ByteLocation) -> Operation {
    Box::new(move |this| {
      let value = this.read_byte(source);
      let result = value.rotate_left(4);
      let flag = if result == 0 { 0x80u8 } else { 0x00u8 };
      this.registers.write_byte(ByteRegister::F, flag);
      this.write_byte(destination, result);
    })
  }

  fn increment_word(location: WordLocation) -> Operation {
    Box::new(move |this| {
      let word = this.read_word(location);
      this.write_word(location, word.wrapping_add(1));
    })
  }

  fn decrement_word(location: WordLocation) -> Operation {
    Box::new(move |this| {
      let word = this.read_word(location);
      this.write_word(location, word.wrapping_sub(1));
    })
  }

  fn reg_to_reg_ld(&mut self) {
    CPU::move_byte(
      ByteLocation::Register(ByteRegister::from_r_bits(self.context.opcode.z_bits())),
      ByteLocation::Register(ByteRegister::from_r_bits(self.context.opcode.y_bits())),
    )(self);
  }

  fn immediate_to_reg_ld(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::Register(ByteRegister::from_r_bits(self.context.opcode.y_bits())),
      )
    );
  }

  fn immediate_to_indirect_ld(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::ByteBuffer,
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
      )
    );
  }

  fn indirect_to_reg_ld(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::Register(ByteRegister::from_r_bits(self.context.opcode.y_bits())),
      )
    );
  }

  fn reg_to_indirect_ld(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::Register(ByteRegister::from_r_bits(self.context.opcode.z_bits())),
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
      )
    );
  }

  fn indirect_bc_to_reg_a_ld(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::BC),
        ByteLocation::Register(ByteRegister::A),
      )
    );
  }

  fn indirect_de_to_reg_a_ld(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::DE),
        ByteLocation::Register(ByteRegister::A),
      )
    );
  }

  fn indirect_c_with_offset_to_reg_a_ld(&mut self) {
    CPU::move_byte(ByteLocation::Value(0xFF), ByteLocation::UpperAddressBuffer)(self);
    CPU::move_byte(ByteLocation::Register(ByteRegister::C), ByteLocation::LowerAddressBuffer)(self);
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByAddressBuffer,
        ByteLocation::Register(ByteRegister::A),
      )
    );
  }

  fn reg_a_to_indirect_c_ld(&mut self) {
    CPU::move_byte(ByteLocation::Value(0xFF), ByteLocation::UpperAddressBuffer)(self);
    CPU::move_byte(ByteLocation::Register(ByteRegister::C), ByteLocation::LowerAddressBuffer)(self);
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::Register(ByteRegister::A),
        ByteLocation::MemoryReferencedByAddressBuffer,
      )
    );
  }

  fn immediate_indirect_with_offset_to_reg_a_ld(&mut self) {
    self.operations.push_back(
      CPU::combine_operations(
        CPU::move_byte(
          ByteLocation::Value(0xFF),
          ByteLocation::UpperAddressBuffer,
        ),
        CPU::move_byte(
          ByteLocation::NextMemoryByte,
          ByteLocation::LowerAddressBuffer,
        ),
      ));
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByAddressBuffer,
        ByteLocation::Register(ByteRegister::A),
      )
    );
  }

  fn reg_a_to_immediate_indirect_with_offset_ld(&mut self) {
    self.operations.push_back(
      CPU::combine_operations(
        CPU::move_byte(
          ByteLocation::Value(0xFF),
          ByteLocation::UpperAddressBuffer,
        ),
        CPU::move_byte(
          ByteLocation::NextMemoryByte,
          ByteLocation::LowerAddressBuffer,
        ),
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::Register(ByteRegister::A),
        ByteLocation::MemoryReferencedByAddressBuffer,
      )
    );
  }

  fn immediate_indirect_to_reg_a_ld(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::LowerAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::UpperAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByAddressBuffer,
        ByteLocation::Register(ByteRegister::A),
      )
    );
  }

  fn reg_a_to_immediate_indirect_ld(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::LowerAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::UpperAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::Register(ByteRegister::A),
        ByteLocation::MemoryReferencedByAddressBuffer,
      )
    );
  }

  fn indirect_hl_to_reg_a_ld_and_increment(&mut self) {
    self.operations.push_back(
      CPU::combine_operations(
        CPU::move_byte(
          ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
          ByteLocation::Register(ByteRegister::A),
        ),
        CPU::increment_word(WordLocation::Register(WordRegister::HL)),
      )
    );
  }

  fn indirect_hl_to_reg_a_ld_and_decrement(&mut self) {
    self.operations.push_back(
      CPU::combine_operations(
        CPU::move_byte(
          ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
          ByteLocation::Register(ByteRegister::A),
        ),
        CPU::decrement_word(WordLocation::Register(WordRegister::HL)),
      )
    );
  }

  fn reg_a_to_indirect_bc_ld(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::Register(ByteRegister::A),
        ByteLocation::MemoryReferencedByRegister(WordRegister::BC),
      )
    );
  }

  fn reg_a_to_indirect_de_ld(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::Register(ByteRegister::A),
        ByteLocation::MemoryReferencedByRegister(WordRegister::DE),
      )
    );
  }

  fn reg_a_to_indirect_hl_ld_and_increment(&mut self) {
    self.operations.push_back(
      CPU::combine_operations(
        CPU::move_byte(
          ByteLocation::Register(ByteRegister::A),
          ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ),
        CPU::increment_word(WordLocation::Register(WordRegister::HL)),
      )
    );
  }

  fn reg_a_to_indirect_hl_ld_and_decrement(&mut self) {
    self.operations.push_back(
      CPU::combine_operations(
        CPU::move_byte(
          ByteLocation::Register(ByteRegister::A),
          ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ),
        CPU::decrement_word(WordLocation::Register(WordRegister::HL)),
      )
    );
  }

  fn immediate_to_reg_pair_ld(&mut self) {
    let register = WordRegister::from_dd_bits(self.context.opcode.dd_bits());
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::Register(register.get_lower_byte_register()),
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::Register(register.get_upper_byte_register()),
      )
    );
  }

  fn reg_hl_to_reg_sp_ld(&mut self) {
    CPU::move_byte(
      ByteLocation::Register(ByteRegister::LowerHL),
      ByteLocation::Register(ByteRegister::LowerSP),
    )(self);
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::Register(ByteRegister::UpperHL),
        ByteLocation::Register(ByteRegister::UpperSP),
      )
    );
  }

  fn push_reg_pair_to_stack(&mut self) {
    let register = WordRegister::from_qq_bits(self.context.opcode.qq_bits());
    self.operations.push_back(
      CPU::combine_operations(
        CPU::decrement_word(WordLocation::Register(WordRegister::SP)),
        CPU::move_byte(
          ByteLocation::Register(register.get_upper_byte_register()),
          ByteLocation::MemoryReferencedByRegister(WordRegister::SP),
        ),
      )
    );
    self.operations.push_back(
      CPU::combine_operations(
        CPU::decrement_word(WordLocation::Register(WordRegister::SP)),
        CPU::move_byte(
          ByteLocation::Register(register.get_lower_byte_register()),
          ByteLocation::MemoryReferencedByRegister(WordRegister::SP),
        ),
      )
    );
    self.operations.push_back(CPU::noop()); // Normally we'd decrement the SP by 2 here, but we've already done this in the previous steps
  }

  fn pop_stack_to_reg_pair(&mut self) {
    let register = WordRegister::from_qq_bits(self.context.opcode.qq_bits());
    self.operations.push_back(
      CPU::combine_operations(
        CPU::move_byte(
          ByteLocation::MemoryReferencedByRegister(WordRegister::SP),
          ByteLocation::Register(register.get_lower_byte_register()),
        ),
        CPU::increment_word(WordLocation::Register(WordRegister::SP)),
      )
    );
    self.operations.push_back(
      CPU::combine_operations(
        CPU::move_byte(
          ByteLocation::MemoryReferencedByRegister(WordRegister::SP),
          ByteLocation::Register(register.get_upper_byte_register()),
        ),
        CPU::increment_word(WordLocation::Register(WordRegister::SP)),
      )
    );
  }

  // TODO: Do a more thorough check to see if this is correct. There seems to be a lot of confusion surrounding the (half) carry bits
  fn reg_sp_plus_signed_immediate_to_hl_ld(&mut self) {
    CPU::move_byte(
      ByteLocation::Value(0x00),
      ByteLocation::Register(ByteRegister::F),
    )(self);
    self.operations.push_back(Box::new(|this| {
      this.context.word_buffer = this.read_next_byte() as i8 as u16;
    }));
    self.operations.push_back(
      CPU::add_words(WordArithmeticParams {
        first: WordLocation::Register(WordRegister::SP),
        second: WordLocation::WordBuffer,
        destination: WordLocation::Register(WordRegister::HL),
        flag_mask: 0x30,
      })
    );
  }

  fn reg_sp_to_immediate_indirect_ld(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::LowerAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::UpperAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::Register(ByteRegister::LowerSP),
        ByteLocation::MemoryReferencedByAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::combine_operations(
        CPU::increment_word(WordLocation::AddressBuffer),
        CPU::move_byte(
          ByteLocation::Register(ByteRegister::UpperSP),
          ByteLocation::MemoryReferencedByAddressBuffer),
      ),
    );
  }

  fn add_reg_to_reg_a_and_write_to_reg_a(&mut self, use_carry: bool) {
    CPU::add_bytes(ByteArithmeticParams {
      first: ByteLocation::Register(ByteRegister::A),
      second: ByteLocation::Register(ByteRegister::from_r_bits(self.context.opcode.z_bits())),
      destination: ByteLocation::Register(ByteRegister::A),
      use_carry,
      flag_mask: 0xF0,
    })(self);
  }

  fn add_immediate_to_reg_a_and_write_to_reg_a(&mut self, use_carry: bool) {
    self.operations.push_back(
      CPU::add_bytes(ByteArithmeticParams {
        first: ByteLocation::Register(ByteRegister::A),
        second: ByteLocation::NextMemoryByte,
        destination: ByteLocation::Register(ByteRegister::A),
        use_carry,
        flag_mask: 0xF0,
      })
    );
  }

  fn add_indirect_hl_to_reg_a_and_write_to_reg_a(&mut self, use_carry: bool) {
    self.operations.push_back(
      CPU::add_bytes(ByteArithmeticParams {
        first: ByteLocation::Register(ByteRegister::A),
        second: ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        destination: ByteLocation::Register(ByteRegister::A),
        use_carry,
        flag_mask: 0xF0,
      })
    );
  }

  fn subtract_reg_from_reg_a_and_write_to_reg_a(&mut self, use_carry: bool) {
    CPU::subtract_bytes(ByteArithmeticParams {
      first: ByteLocation::Register(ByteRegister::A),
      second: ByteLocation::Register(ByteRegister::from_r_bits(self.context.opcode.z_bits())),
      destination: ByteLocation::Register(ByteRegister::A),
      use_carry,
      flag_mask: 0xF0,
    })(self);
  }

  fn subtract_immediate_from_reg_a_and_write_to_reg_a(&mut self, use_carry: bool) {
    self.operations.push_back(
      CPU::subtract_bytes(ByteArithmeticParams {
        first: ByteLocation::Register(ByteRegister::A),
        second: ByteLocation::NextMemoryByte,
        destination: ByteLocation::Register(ByteRegister::A),
        use_carry,
        flag_mask: 0xF0,
      })
    );
  }

  fn subtract_indirect_hl_from_reg_a_and_write_to_reg_a(&mut self, use_carry: bool) {
    self.operations.push_back(
      CPU::subtract_bytes(ByteArithmeticParams {
        first: ByteLocation::Register(ByteRegister::A),
        second: ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        destination: ByteLocation::Register(ByteRegister::A),
        use_carry,
        flag_mask: 0xF0,
      })
    );
  }

  fn and_reg_with_reg_a_and_write_to_reg_a(&mut self) {
    CPU::and_bytes(
      ByteLocation::Register(ByteRegister::from_r_bits(self.context.opcode.z_bits())),
      ByteLocation::Register(ByteRegister::A),
      ByteLocation::Register(ByteRegister::A),
    )(self);
  }

  fn and_immediate_with_reg_a_and_write_to_reg_a(&mut self) {
    self.operations.push_back(
      CPU::and_bytes(
        ByteLocation::NextMemoryByte,
        ByteLocation::Register(ByteRegister::A),
        ByteLocation::Register(ByteRegister::A),
      )
    );
  }

  fn and_indirect_hl_with_reg_a_and_write_to_reg_a(&mut self) {
    self.operations.push_back(
      CPU::and_bytes(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::Register(ByteRegister::A),
        ByteLocation::Register(ByteRegister::A),
      )
    );
  }

  fn or_reg_with_reg_a_and_write_to_reg_a(&mut self) {
    CPU::or_bytes(
      ByteLocation::Register(ByteRegister::from_r_bits(self.context.opcode.z_bits())),
      ByteLocation::Register(ByteRegister::A),
      ByteLocation::Register(ByteRegister::A),
    )(self);
  }

  fn or_immediate_with_reg_a_and_write_to_reg_a(&mut self) {
    self.operations.push_back(
      CPU::or_bytes(
        ByteLocation::NextMemoryByte,
        ByteLocation::Register(ByteRegister::A),
        ByteLocation::Register(ByteRegister::A),
      )
    );
  }

  fn or_indirect_hl_with_reg_a_and_write_to_reg_a(&mut self) {
    self.operations.push_back(
      CPU::or_bytes(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::Register(ByteRegister::A),
        ByteLocation::Register(ByteRegister::A),
      )
    );
  }

  fn xor_reg_with_reg_a_and_write_to_reg_a(&mut self) {
    CPU::xor_bytes(
      ByteLocation::Register(ByteRegister::from_r_bits(self.context.opcode.z_bits())),
      ByteLocation::Register(ByteRegister::A),
      ByteLocation::Register(ByteRegister::A),
    )(self);
  }

  fn xor_immediate_with_reg_a_and_write_to_reg_a(&mut self) {
    self.operations.push_back(
      CPU::xor_bytes(
        ByteLocation::NextMemoryByte,
        ByteLocation::Register(ByteRegister::A),
        ByteLocation::Register(ByteRegister::A),
      )
    );
  }

  fn xor_indirect_hl_with_reg_a_and_write_to_reg_a(&mut self) {
    self.operations.push_back(
      CPU::xor_bytes(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::Register(ByteRegister::A),
        ByteLocation::Register(ByteRegister::A),
      )
    );
  }

  fn compare_reg_with_reg_a(&mut self) {
    CPU::subtract_bytes(ByteArithmeticParams {
      first: ByteLocation::Register(ByteRegister::A),
      second: ByteLocation::Register(ByteRegister::from_r_bits(self.context.opcode.z_bits())),
      destination: ByteLocation::ByteBuffer,
      use_carry: false,
      flag_mask: 0xF0,
    })(self);
  }

  fn compare_immediate_with_reg_a(&mut self) {
    self.operations.push_back(
      CPU::subtract_bytes(ByteArithmeticParams {
        first: ByteLocation::Register(ByteRegister::A),
        second: ByteLocation::NextMemoryByte,
        destination: ByteLocation::ByteBuffer,
        use_carry: false,
        flag_mask: 0xF0,
      })
    );
  }

  fn compare_indirect_hl_with_reg_a(&mut self) {
    self.operations.push_back(
      CPU::subtract_bytes(ByteArithmeticParams {
        first: ByteLocation::Register(ByteRegister::A),
        second: ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        destination: ByteLocation::ByteBuffer,
        use_carry: false,
        flag_mask: 0xF0,
      })
    );
  }

  fn increment_reg(&mut self) {
    let register = ByteRegister::from_r_bits(self.context.opcode.y_bits());
    CPU::add_bytes(ByteArithmeticParams {
      first: ByteLocation::Register(register),
      second: ByteLocation::Value(1),
      destination: ByteLocation::Register(register),
      use_carry: false,
      flag_mask: 0xE0,
    })(self);
  }

  fn increment_indirect_hl(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      CPU::add_bytes(ByteArithmeticParams {
        first: ByteLocation::ByteBuffer,
        second: ByteLocation::Value(1),
        destination: ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        use_carry: false,
        flag_mask: 0xE0,
      })
    );
  }

  fn decrement_reg(&mut self) {
    let register = ByteRegister::from_r_bits(self.context.opcode.y_bits());
    CPU::subtract_bytes(ByteArithmeticParams {
      first: ByteLocation::Register(register),
      second: ByteLocation::Value(1),
      destination: ByteLocation::Register(register),
      use_carry: false,
      flag_mask: 0xE0,
    })(self);
  }

  fn decrement_indirect_hl(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      CPU::subtract_bytes(ByteArithmeticParams {
        first: ByteLocation::ByteBuffer,
        second: ByteLocation::Value(1),
        destination: ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        use_carry: false,
        flag_mask: 0xE0,
      })
    );
  }

  fn add_reg_pair_to_reg_hl(&mut self) {
    let register = WordRegister::from_dd_bits(self.context.opcode.dd_bits());
    CPU::add_words(WordArithmeticParams {
      first: WordLocation::Register(register),
      second: WordLocation::Register(WordRegister::HL),
      destination: WordLocation::WordBuffer,
      flag_mask: 0x70,
    })(self);
    CPU::move_byte(
      ByteLocation::LowerWordBuffer,
      ByteLocation::Register(ByteRegister::LowerHL),
    )(self);
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::UpperWordBuffer,
        ByteLocation::Register(ByteRegister::UpperHL),
      )
    );
  }

  //TODO: Check whether the flags are set correctly
  fn add_immediate_to_reg_sp(&mut self) {
    self.operations.push_back(Box::new(|this| {
      this.context.word_buffer = this.read_next_byte() as i8 as u16;
    }));
    self.operations.push_back(
      CPU::combine_operations(
        CPU::add_words(WordArithmeticParams {
          first: WordLocation::Register(WordRegister::SP),
          second: WordLocation::WordBuffer,
          destination: WordLocation::WordBuffer,
          flag_mask: 0x30,
        }),
        CPU::move_byte(
          ByteLocation::LowerWordBuffer,
          ByteLocation::Register(ByteRegister::LowerSP),
        ),
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::UpperWordBuffer,
        ByteLocation::Register(ByteRegister::UpperSP),
      )
    );
  }

  fn increment_reg_pair(&mut self) {
    let register = WordRegister::from_dd_bits(self.context.opcode.dd_bits());
    CPU::move_word(
      WordLocation::Register(register),
      WordLocation::WordBuffer,
    )(self);
    CPU::increment_word(WordLocation::WordBuffer)(self);
    CPU::move_byte(
      ByteLocation::LowerWordBuffer,
      ByteLocation::Register(register.get_lower_byte_register()),
    )(self);
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::UpperWordBuffer,
        ByteLocation::Register(register.get_upper_byte_register()),
      )
    );
  }

  fn decrement_reg_pair(&mut self) {
    let register = WordRegister::from_dd_bits(self.context.opcode.dd_bits());
    CPU::move_word(
      WordLocation::Register(register),
      WordLocation::WordBuffer,
    )(self);
    CPU::decrement_word(WordLocation::WordBuffer)(self);
    CPU::move_byte(
      ByteLocation::LowerWordBuffer,
      ByteLocation::Register(register.get_lower_byte_register()),
    )(self);
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::UpperWordBuffer,
        ByteLocation::Register(register.get_upper_byte_register()),
      )
    );
  }

  fn rotate_reg_a_left(&mut self) {
    CPU::rotate_byte_left(
      ByteLocation::Register(ByteRegister::A),
      ByteLocation::Register(ByteRegister::A),
      true,
    )(self);
  }

  fn rotate_reg_left(&mut self) {
    let register = ByteRegister::from_r_bits(self.context.opcode.z_bits());
    CPU::rotate_byte_left(
      ByteLocation::Register(register),
      ByteLocation::Register(register),
      false,
    )(self);
  }

  fn rotate_indirect_hl_left(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      CPU::rotate_byte_left(
        ByteLocation::ByteBuffer,
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        false,
      )
    );
  }

  fn rotate_reg_a_left_through_carry(&mut self) {
    CPU::rotate_byte_left_through_carry(
      ByteLocation::Register(ByteRegister::A),
      ByteLocation::Register(ByteRegister::A),
      true,
    )(self);
  }

  fn rotate_reg_left_through_carry(&mut self) {
    let register = ByteRegister::from_r_bits(self.context.opcode.z_bits());
    CPU::rotate_byte_left_through_carry(
      ByteLocation::Register(register),
      ByteLocation::Register(register),
      false,
    )(self);
  }

  fn rotate_indirect_hl_left_through_carry(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      CPU::rotate_byte_left_through_carry(
        ByteLocation::ByteBuffer,
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        false,
      )
    );
  }

  fn rotate_reg_a_right(&mut self) {
    CPU::rotate_byte_right(
      ByteLocation::Register(ByteRegister::A),
      ByteLocation::Register(ByteRegister::A),
      true,
    )(self);
  }

  fn rotate_reg_right(&mut self) {
    let register = ByteRegister::from_r_bits(self.context.opcode.z_bits());
    CPU::rotate_byte_right(
      ByteLocation::Register(register),
      ByteLocation::Register(register),
      false,
    )(self);
  }

  fn rotate_indirect_hl_right(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      CPU::rotate_byte_right(
        ByteLocation::ByteBuffer,
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        false,
      )
    );
  }

  fn rotate_reg_a_right_through_carry(&mut self) {
    CPU::rotate_byte_right_through_carry(
      ByteLocation::Register(ByteRegister::A),
      ByteLocation::Register(ByteRegister::A),
      true,
    )(self);
  }

  fn rotate_reg_right_through_carry(&mut self) {
    let register = ByteRegister::from_r_bits(self.context.opcode.z_bits());
    CPU::rotate_byte_right_through_carry(
      ByteLocation::Register(register),
      ByteLocation::Register(register),
      false,
    )(self);
  }

  fn rotate_indirect_hl_right_through_carry(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      CPU::rotate_byte_right_through_carry(
        ByteLocation::ByteBuffer,
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        false,
      )
    );
  }

  fn shift_reg_left(&mut self) {
    let register = ByteRegister::from_r_bits(self.context.opcode.z_bits());
    CPU::shift_byte_left(
      ByteLocation::Register(register),
      ByteLocation::Register(register),
    )(self);
  }

  fn shift_reg_right(&mut self) {
    let register = ByteRegister::from_r_bits(self.context.opcode.z_bits());
    CPU::shift_byte_right(
      ByteLocation::Register(register),
      ByteLocation::Register(register),
    )(self);
  }

  fn shift_reg_right_arithmetic(&mut self) {
    let register = ByteRegister::from_r_bits(self.context.opcode.z_bits());
    CPU::shift_byte_right_arithmetic(
      ByteLocation::Register(register),
      ByteLocation::Register(register),
    )(self);
  }

  fn shift_indirect_hl_left(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      CPU::shift_byte_left(
        ByteLocation::ByteBuffer,
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
      )
    );
  }

  fn shift_indirect_hl_right(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      CPU::shift_byte_right(
        ByteLocation::ByteBuffer,
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
      )
    );
  }

  fn shift_indirect_hl_right_arithmetic(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      CPU::shift_byte_right_arithmetic(
        ByteLocation::ByteBuffer,
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
      )
    );
  }

  fn swap_reg(&mut self) {
    let register = ByteRegister::from_r_bits(self.context.opcode.z_bits());
    CPU::swap_byte(
      ByteLocation::Register(register),
      ByteLocation::Register(register),
    )(self);
  }

  fn swap_indirect_hl(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      CPU::swap_byte(
        ByteLocation::ByteBuffer,
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
      )
    );
  }

  fn get_reg_bit(&mut self) {
    let value = self.registers.read_byte(ByteRegister::from_r_bits(self.context.opcode.z_bits()));
    let bit = self.context.opcode.y_bits();
    self.registers.write_byte_masked(ByteRegister::F, u8::compose(&[(!value.get_bit(bit), 7), (false, 6), (true, 5)]), 0xE0);
  }

  fn get_indirect_hl_bit(&mut self) {
    self.operations.push_back(Box::new(|this| {
      let address = this.registers.read_word(WordRegister::HL);
      let value = this.memory.borrow().read(address);
      let bit = this.context.opcode.y_bits();
      this.registers.write_byte_masked(ByteRegister::F, u8::compose(&[(!value.get_bit(bit), 7), (false, 6), (true, 5)]), 0xE0);
    }));
  }

  fn set_reg_bit(&mut self) {
    let register = ByteRegister::from_r_bits(self.context.opcode.z_bits());
    let value = self.registers.read_byte(register);
    let bit = self.context.opcode.y_bits();
    self.registers.write_byte(register, value.set_bit(bit));
  }

  fn set_indirect_hl_bit(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      Box::new(|this| {
        let bit = this.context.opcode.y_bits();
        CPU::move_byte(
          ByteLocation::Value(this.context.byte_buffer.set_bit(bit)),
          ByteLocation::MemoryReferencedByRegister(WordRegister::HL)
        )(this);
      })
    );
  }

  fn reset_reg_bit(&mut self) {
    let register = ByteRegister::from_r_bits(self.context.opcode.z_bits());
    let value = self.registers.read_byte(register);
    let bit = self.context.opcode.y_bits();
    self.registers.write_byte(register, value.reset_bit(bit));
  }

  fn reset_indirect_hl_bit(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::MemoryReferencedByRegister(WordRegister::HL),
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      Box::new(|this| {
        let bit = this.context.opcode.y_bits();
        CPU::move_byte(
          ByteLocation::Value(this.context.byte_buffer.reset_bit(bit)),
          ByteLocation::MemoryReferencedByRegister(WordRegister::HL)
        )(this);
      })
    );
  }

  fn jump(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::LowerAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::UpperAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::move_word(
        WordLocation::AddressBuffer,
        WordLocation::Register(WordRegister::PC),
      )
    );
  }

  fn satisfies_condition(&self, opcode: Opcode) -> bool {
    let condition = opcode.cc_bits();
    match condition {
      0x00 => !self.registers.read_byte(ByteRegister::F).get_bit(7),
      0x01 => self.registers.read_byte(ByteRegister::F).get_bit(7),
      0x02 => !self.registers.read_byte(ByteRegister::F).get_bit(4),
      0x03 => self.registers.read_byte(ByteRegister::F).get_bit(4),
      _ => panic!("{} doesn't map to a condition value", condition)
    }
  }

  fn jump_conditional(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::LowerAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::UpperAddressBuffer,
      )
    );
    if self.satisfies_condition(self.context.opcode) {
      self.operations.push_back(
        CPU::move_word(
          WordLocation::AddressBuffer,
          WordLocation::Register(WordRegister::PC),
        )
      );
    }
  }

  fn jump_relative(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::ByteBuffer,
      )
    );
    self.operations.push_back(
      Box::new(|this| {
        this.registers.write_word(WordRegister::PC, this.registers.read_word(WordRegister::PC).wrapping_add(this.context.byte_buffer as i8 as u16));
      })
    );
  }

  fn jump_conditional_relative(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::ByteBuffer,
      )
    );
    if self.satisfies_condition(self.context.opcode) {
      self.operations.push_back(
        Box::new(|this| {
          this.registers.write_word(WordRegister::PC, this.registers.read_word(WordRegister::PC).wrapping_add(this.context.byte_buffer as i8 as u16));
        })
      );
    }
  }

  fn jump_to_indirect_hl(&mut self) {
    CPU::move_word(
      WordLocation::Register(WordRegister::HL),
      WordLocation::Register(WordRegister::PC),
    )(self);
  }

  fn call(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::LowerAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::UpperAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::combine_operations(
        CPU::decrement_word(WordLocation::Register(WordRegister::SP)),
        CPU::move_byte(
          ByteLocation::Register(ByteRegister::UpperPC),
          ByteLocation::MemoryReferencedByRegister(WordRegister::SP),
        ),
      )
    );
    self.operations.push_back(
      CPU::combine_operations(
        CPU::decrement_word(WordLocation::Register(WordRegister::SP)),
        CPU::move_byte(
          ByteLocation::Register(ByteRegister::LowerPC),
          ByteLocation::MemoryReferencedByRegister(WordRegister::SP),
        ),
      )
    );
    self.operations.push_back(
      CPU::move_word(
        WordLocation::AddressBuffer,
        WordLocation::Register(WordRegister::PC),
      )
    );
  }

  fn call_conditional(&mut self) {
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::LowerAddressBuffer,
      )
    );
    self.operations.push_back(
      CPU::move_byte(
        ByteLocation::NextMemoryByte,
        ByteLocation::UpperAddressBuffer,
      )
    );
    if self.satisfies_condition(self.context.opcode) {
      self.operations.push_back(
        CPU::combine_operations(
          CPU::decrement_word(WordLocation::Register(WordRegister::SP)),
          CPU::move_byte(
            ByteLocation::Register(ByteRegister::UpperPC),
            ByteLocation::MemoryReferencedByRegister(WordRegister::SP),
          ),
        )
      );
      self.operations.push_back(
        CPU::combine_operations(
          CPU::decrement_word(WordLocation::Register(WordRegister::SP)),
          CPU::move_byte(
            ByteLocation::Register(ByteRegister::LowerPC),
            ByteLocation::MemoryReferencedByRegister(WordRegister::SP),
          ),
        )
      );
      self.operations.push_back(
        CPU::move_word(
          WordLocation::AddressBuffer,
          WordLocation::Register(WordRegister::PC),
        )
      );
    }
  }

  fn return_from_call(&mut self) {
    self.operations.push_back(
      CPU::combine_operations(
        CPU::move_byte(
          ByteLocation::MemoryReferencedByRegister(WordRegister::SP),
          ByteLocation::LowerWordBuffer,
        ),
        CPU::increment_word(WordLocation::Register(WordRegister::SP)),
      )
    );
    self.operations.push_back(
      CPU::combine_operations(
        CPU::move_byte(
          ByteLocation::MemoryReferencedByRegister(WordRegister::SP),
          ByteLocation::UpperWordBuffer,
        ),
        CPU::increment_word(WordLocation::Register(WordRegister::SP)),
      )
    );
    self.operations.push_back(
      CPU::move_word(
        WordLocation::WordBuffer,
        WordLocation::Register(WordRegister::PC),
      )
    );
  }

  fn return_from_interrupt(&mut self) {
    self.return_from_call();
    self.ime = true;
  }

  fn return_conditionally(&mut self) {
    self.operations.push_back(
      Box::new(|this| {
        if this.satisfies_condition(this.context.opcode) {
          this.return_from_call();
        }
      })
    );
  }

  fn restart(&mut self) {
    let address = match self.context.opcode.y_bits() {
      0 => 0x0000u16,
      1 => 0x0008u16,
      2 => 0x0010u16,
      3 => 0x0018u16,
      4 => 0x0020u16,
      5 => 0x0028u16,
      6 => 0x0030u16,
      7 => 0x0038u16,
      _ => panic!("{} is not a valid restart code", self.context.opcode.y_bits())
    };
    self.operations.push_back(
      CPU::combine_operations(
        CPU::decrement_word(WordLocation::Register(WordRegister::SP)),
        CPU::move_byte(
          ByteLocation::Register(ByteRegister::UpperPC),
          ByteLocation::MemoryReferencedByRegister(WordRegister::SP),
        ),
      )
    );
    self.operations.push_back(
      CPU::combine_operations(
        CPU::decrement_word(WordLocation::Register(WordRegister::SP)),
        CPU::move_byte(
          ByteLocation::Register(ByteRegister::LowerPC),
          ByteLocation::MemoryReferencedByRegister(WordRegister::SP),
        ),
      )
    );
    self.operations.push_back(
      CPU::move_word(
        WordLocation::Value(address),
        WordLocation::Register(WordRegister::PC),
      )
    );
  }

  fn decimal_adjust_reg_a(&mut self) {
    let a = self.registers.read_byte(ByteRegister::A);
    let f = self.registers.read_byte(ByteRegister::F);
    let n = f.get_bit(6);
    let carry = f.get_bit(4);
    let half_carry = f.get_bit(5);
    if n {
      let lower = if half_carry { 6u8 } else { 0u8 };
      let upper = if carry { 0x60u8 } else { 0u8 };
      CPU::subtract_bytes(ByteArithmeticParams {
        first: ByteLocation::Value(a),
        second: ByteLocation::Value(upper | lower),
        destination: ByteLocation::Register(ByteRegister::A),
        use_carry: false,
        flag_mask: 0xB0,
      })(self);
    } else {
      let lower = if half_carry || ((a & 0x0F) >= 0x0A) { 6u8 } else { 0u8 };
      let upper = if carry || (a > 0x99) { 0x60u8 } else { 0u8 };
      CPU::add_bytes(ByteArithmeticParams {
        first: ByteLocation::Value(a),
        second: ByteLocation::Value(upper | lower),
        destination: ByteLocation::Register(ByteRegister::A),
        use_carry: false,
        flag_mask: 0xB0,
      })(self);
    };
    if carry {
      self.registers.write_byte_masked(ByteRegister::F, 0x10, 0x30);
    } else {
      self.registers.write_byte_masked(ByteRegister::F, 0x00, 0x20);
    }
  }

  fn ones_complement_reg_a(&mut self) {
    self.registers.write_byte(ByteRegister::A, !self.registers.read_byte(ByteRegister::A));
    self.registers.write_byte_masked(ByteRegister::F, 0x60, 0x60);
  }

  fn flip_carry_flag(&mut self) {
    self.registers.write_byte_masked(ByteRegister::F, (self.registers.read_byte(ByteRegister::F) ^ 0x10) & 0x90, 0x70);
  }

  fn set_carry_flag(&mut self) {
    self.registers.write_byte_masked(ByteRegister::F, 0x10, 0x70);
  }

  fn disable_interrupts(&mut self) {
    self.ime = false;
  }

  fn enable_interrupts(&mut self) {
    self.ime = true;
  }

  fn halt(&mut self) {
    //TODO: Implement halt
  }

  fn stop(&mut self) {
    // TODO: Implement stop
  }
}

impl ClockAware for CPU {
  fn handle_tick(&mut self, _double_speed: bool) {
    if let Some(operation) = self.operations.pop_front() {
      operation(self);
    } else {
      self.fetch_and_execute_instruction();
    }
  }
}

#[cfg(test)]
mod tests {
  use assert_hex::assert_eq_hex;
  use super::*;
  use crate::memory::memory::test::MockMemory;
  use test_case::test_case;

  #[test]
  fn reg_to_reg_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    memory.borrow_mut().write(0x0000, 0x45);
    cpu.registers.write_byte(ByteRegister::LowerHL, 0xAB);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::B), 0xAB);
  }

  #[test]
  fn immediate_to_reg_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    memory.borrow_mut().write(0x0000, 0x06);
    memory.borrow_mut().write(0x0001, 0xAB);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::B), 0xAB);
  }

  #[test]
  fn indirect_to_reg_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    memory.borrow_mut().write(0x0000, 0x6E);
    memory.borrow_mut().write(0xABCD, 0xEF);
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::LowerHL), 0xEF);
  }

  #[test]
  fn reg_to_indirect_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    cpu.registers.write_byte(ByteRegister::A, 0xEF);
    memory.borrow_mut().write(0x0000, 0x77);
    cpu.ticks(2);
    assert_eq!(memory.borrow().read(0xABCD), 0xEF);
  }

  #[test]
  fn immediate_to_indirect_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x36);
    memory.borrow_mut().write(0x0001, 0xEF);
    cpu.ticks(3);
    assert_eq!(memory.borrow().read(0xABCD), 0xEF);
  }

  #[test]
  fn indirect_bc_to_reg_a_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::BC, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x0A);
    memory.borrow_mut().write(0xABCD, 0x5A);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), 0x5A);
  }

  #[test]
  fn indirect_de_to_reg_a_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::DE, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x1A);
    memory.borrow_mut().write(0xABCD, 0x5A);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), 0x5A);
  }

  #[test]
  fn indirect_c_with_offset_to_reg_a_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::C, 0xCD);
    memory.borrow_mut().write(0x0000, 0xF2);
    memory.borrow_mut().write(0xFFCD, 0x5A);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), 0x5A);
  }

  #[test]
  fn reg_a_to_indirect_c_with_offset_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0x5A);
    cpu.registers.write_byte(ByteRegister::C, 0xCD);
    memory.borrow_mut().write(0x0000, 0xE2);
    cpu.ticks(2);
    assert_eq!(memory.borrow().read(0xFFCD), 0x5A);
  }

  #[test]
  fn immediate_indirect_with_offset_to_reg_a_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    memory.borrow_mut().write(0x0000, 0xF0);
    memory.borrow_mut().write(0x0001, 0xCD);
    memory.borrow_mut().write(0xFFCD, 0x5A);
    cpu.ticks(3);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), 0x5A);
  }

  #[test]
  fn reg_a_to_immediate_indirect_with_offset_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0x5A);
    memory.borrow_mut().write(0x0000, 0xE0);
    memory.borrow_mut().write(0x0001, 0xCD);
    cpu.ticks(3);
    assert_eq!(memory.borrow().read(0xFFCD), 0x5A);
  }

  #[test]
  fn immediate_indirect_to_reg_a_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    memory.borrow_mut().write(0x0000, 0xFA);
    memory.borrow_mut().write(0x0001, 0xCD);
    memory.borrow_mut().write(0x0002, 0xAB);
    memory.borrow_mut().write(0xABCD, 0x5A);
    cpu.ticks(4);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), 0x5A);
  }

  #[test]
  fn reg_a_to_immediate_indirect_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0x5A);
    memory.borrow_mut().write(0x0000, 0xEA);
    memory.borrow_mut().write(0x0001, 0xCD);
    memory.borrow_mut().write(0x0002, 0xAB);
    cpu.ticks(4);
    assert_eq!(memory.borrow().read(0xABCD), 0x5A);
  }


  #[test]
  fn indirect_hl_to_reg_a_ld_and_increment() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x2A);
    memory.borrow_mut().write(0xABCD, 0x5A);
    cpu.ticks(2);
    assert_eq!(memory.borrow().read(0xABCD), 0x5A);
    assert_eq!(cpu.registers.read_word(WordRegister::HL), 0xABCE);
  }

  #[test]
  fn indirect_hl_to_reg_a_ld_and_decrement() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x3A);
    memory.borrow_mut().write(0xABCD, 0x5A);
    cpu.ticks(2);
    assert_eq!(memory.borrow().read(0xABCD), 0x5A);
    assert_eq!(cpu.registers.read_word(WordRegister::HL), 0xABCC);
  }

  #[test]
  fn reg_a_to_indirect_bc_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0x5A);
    cpu.registers.write_word(WordRegister::BC, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x02);
    cpu.ticks(2);
    assert_eq!(memory.borrow().read(0xABCD), 0x5A);
  }

  #[test]
  fn reg_a_to_indirect_de_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0x5A);
    cpu.registers.write_word(WordRegister::DE, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x12);
    cpu.ticks(2);
    assert_eq!(memory.borrow().read(0xABCD), 0x5A);
  }

  #[test]
  fn reg_a_to_indirect_hl_ld_and_increment() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0x5A);
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x22);
    cpu.ticks(2);
    assert_eq!(memory.borrow().read(0xABCD), 0x5A);
    assert_eq!(cpu.registers.read_word(WordRegister::HL), 0xABCE);
  }

  #[test]
  fn reg_a_to_indirect_hl_ld_and_decrement() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0x5A);
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x32);
    cpu.ticks(2);
    assert_eq!(memory.borrow().read(0xABCD), 0x5A);
    assert_eq!(cpu.registers.read_word(WordRegister::HL), 0xABCC);
  }


  #[test]
  fn immediate_to_reg_pair_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0x5A);
    memory.borrow_mut().write(0x0000, 0x21);
    memory.borrow_mut().write(0x0001, 0x5A);
    memory.borrow_mut().write(0x0002, 0x7B);
    cpu.ticks(3);
    assert_eq!(cpu.registers.read_word(WordRegister::HL), 0x7B5A);
  }

  #[test]
  fn reg_hl_to_reg_sp_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xF9);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_word(WordRegister::SP), 0xABCD);
  }

  #[test]
  fn push_reg_pair_to_stack() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::SP, 0xFFFE);
    cpu.registers.write_word(WordRegister::DE, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xD5);
    cpu.ticks(4);
    assert_eq!(memory.borrow().read(0xFFFD), 0xAB);
    assert_eq!(memory.borrow().read(0xFFFC), 0xCD);
    assert_eq!(cpu.registers.read_word(WordRegister::SP), 0xFFFC);
  }

  #[test]
  fn pop_stack_to_reg_pair() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::SP, 0xFFFC);
    memory.borrow_mut().write(0x0000, 0xD1);
    memory.borrow_mut().write(0xFFFC, 0xCD);
    memory.borrow_mut().write(0xFFFD, 0xAB);
    cpu.ticks(3);
    assert_eq!(cpu.registers.read_word(WordRegister::DE), 0xABCD);
    assert_eq!(cpu.registers.read_word(WordRegister::SP), 0xFFFE);
  }

  #[test]
  fn reg_sp_plus_signed_immediate_to_hl_ld_writes_correct_result() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    // Check if carry flag is set correctly
    cpu.registers.write_word(WordRegister::SP, 0x0005);
    memory.borrow_mut().write(0x0000, 0xF8);
    memory.borrow_mut().write(0x0001, 0xFD);
    cpu.ticks(3);
    assert_eq!(cpu.registers.read_word(WordRegister::HL), 0x0002);
  }

  #[test_case(0x0FF8, 0x07, 0x00; "no flags")]
  #[test_case(0x0FF8, 0x08, 0x20; "only half carry")]
  #[test_case(0xFFF8, 0x08, 0x30; "both carry flags")]
  fn reg_sp_plus_signed_immediate_to_hl_ld_writes_correct_flags(sp: u16, e: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::SP, sp);
    memory.borrow_mut().write(0x0000, 0xF8);
    memory.borrow_mut().write(0x0001, e);
    cpu.ticks(3);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test]
  fn reg_sp_to_immediate_indirect_ld() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::SP, 0x7B5A);
    memory.borrow_mut().write(0x0000, 0x08);
    memory.borrow_mut().write(0x0001, 0xCD);
    memory.borrow_mut().write(0x0002, 0xAB);
    cpu.ticks(5);
    assert_eq!(memory.borrow().read(0xABCD), 0x5A);
    assert_eq!(memory.borrow().read(0xABCE), 0x7B);
  }

  #[test_case(0xFC, 0x04, 0x00, 0xB0; "zero flag set correctly")]
  #[test_case(0xF0, 0xF0, 0xE0, 0x10; "carry set correctly")]
  #[test_case(0x08, 0x08, 0x10, 0x20; "half carry set correctly")]
  fn add_reg_to_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0x82);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0x04, 0x00, 0xB0; "zero flag set correctly")]
  #[test_case(0xF0, 0xF0, 0xE0, 0x10; "carry set correctly")]
  #[test_case(0x08, 0x08, 0x10, 0x20; "half carry set correctly")]
  fn add_immediate_to_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    memory.borrow_mut().write(0x0000, 0xC6);
    memory.borrow_mut().write(0x0001, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0x04, 0x00, 0xB0; "zero flag set correctly")]
  #[test_case(0xF0, 0xF0, 0xE0, 0x10; "carry set correctly")]
  #[test_case(0x08, 0x08, 0x10, 0x20; "half carry set correctly")]
  fn add_indirect_hl_to_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x86);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0x03, 0x00, 0xB0; "zero flag set correctly")]
  #[test_case(0xF0, 0xEF, 0xE0, 0x30; "carry set correctly")]
  #[test_case(0x08, 0x07, 0x10, 0x20; "half carry set correctly")]
  fn add_reg_with_carry_to_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, 0x10);
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0x8A);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0x03, 0x00, 0xB0; "zero flag set correctly")]
  #[test_case(0xF0, 0xEF, 0xE0, 0x30; "carry set correctly")]
  #[test_case(0x08, 0x07, 0x10, 0x20; "half carry set correctly")]
  fn add_immediate_with_carry_to_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::F, 0x10);

    memory.borrow_mut().write(0x0000, 0xCE);
    memory.borrow_mut().write(0x0001, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0x03, 0x00, 0xB0; "zero flag set correctly")]
  #[test_case(0xF0, 0x10, 0x01, 0x10; "carry set correctly")]
  #[test_case(0x08, 0x07, 0x10, 0x20; "half carry set correctly")]
  fn add_indirect_hl_with_carry_to_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::F, 0x10);

    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x8E);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0xFC, 0x00, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0xE0, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x0E, 0x60; "half carry set correctly")]
  fn subtract_reg_from_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0x92);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0xFC, 0x00, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0xE0, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x0E, 0x60; "half carry set correctly")]
  fn subtract_immediate_from_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    memory.borrow_mut().write(0x0000, 0xD6);
    memory.borrow_mut().write(0x0001, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0xFC, 0x00, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0xE0, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x0E, 0x60; "half carry set correctly")]
  fn subtract_indirect_hl_from_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x96);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0xFB, 0x00, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3E, 0xE0, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE2, 0x0E, 0x60; "half carry set correctly")]
  fn subtract_reg_with_carry_from_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, 0x10);
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0x9A);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0xFB, 0x00, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3E, 0xE0, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE2, 0x0E, 0x60; "half carry set correctly")]
  fn subtract_immediate_with_carry_from_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::F, 0x10);

    memory.borrow_mut().write(0x0000, 0xDE);
    memory.borrow_mut().write(0x0001, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0xFB, 0x00, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3E, 0xE0, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE2, 0x0E, 0x60; "half carry set correctly")]
  fn subtract_indirect_hl_with_carry_from_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::F, 0x10);

    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x9E);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x5A, 0xA5, 0x00, 0xA0; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0x88, 0x20; "half carry set correctly")]
  fn and_reg_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0xA2);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x5A, 0xA5, 0x00, 0xA0; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0x88, 0x20; "half carry set correctly")]
  fn and_immediate_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::F, 0x10);

    memory.borrow_mut().write(0x0000, 0xE6);
    memory.borrow_mut().write(0x0001, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x5A, 0xA5, 0x00, 0xA0; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0x88, 0x20; "half carry set correctly")]
  fn and_indirect_hl_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::F, 0x10);

    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xA6);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x00, 0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0xEE, 0x00; "calculates OR correctly")]
  fn or_reg_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0xB2);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x00, 0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0xEE, 0x00; "calculates OR correctly")]
  fn or_immediate_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::F, 0x10);

    memory.borrow_mut().write(0x0000, 0xF6);
    memory.borrow_mut().write(0x0001, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x00, 0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0xEE, 0x00; "calculates OR correctly")]
  fn or_indirect_hl_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::F, 0x10);

    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xB6);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xAE, 0xAE, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0x66, 0x00; "calculates XOR correctly")]
  fn xor_reg_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0xAA);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xAE, 0xAE, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0x66, 0x00; "calculates XOR correctly")]
  fn xor_immediate_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::F, 0x10);

    memory.borrow_mut().write(0x0000, 0xEE);
    memory.borrow_mut().write(0x0001, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xAE, 0xAE, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xAC, 0xCA, 0x66, 0x00; "calculates XOR correctly")]
  fn xor_indirect_hl_with_reg_a_and_write_to_reg_a(a: u8, value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::F, 0x10);

    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xAE);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0xFC, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x60; "half carry set correctly")]
  fn compare_reg_with_reg_a(a: u8, value: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0xBA);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0xFC, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x60; "half carry set correctly")]
  fn compare_immediate_with_reg_a(a: u8, value: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    memory.borrow_mut().write(0x0000, 0xFE);
    memory.borrow_mut().write(0x0001, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFC, 0xFC, 0xC0; "zero flag set correctly")]
  #[test_case(0x1F, 0x3F, 0x50; "carry set correctly")]
  #[test_case(0xF1, 0xE3, 0x60; "half carry set correctly")]
  fn compare_indirect_hl_with_reg_a(a: u8, value: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, a);
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xBE);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFF, 0x00, 0x00, 0xA0; "zero flag set correctly and carry is not affected")]
  #[test_case(0x0F, 0x10, 0x10, 0x30; "half carry set correctly")]
  fn increment_reg(value: u8, result: u8, f_old: u8, f_new: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, f_old);
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0x14);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::D), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f_new);
  }

  #[test_case(0xFF, 0x00, 0x00, 0xA0; "zero flag set correctly and carry is not affected")]
  #[test_case(0x0F, 0x10, 0x10, 0x30; "half carry set correctly")]
  fn increment_indirect_hl(value: u8, result: u8, f_old: u8, f_new: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, f_old);
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x34);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(3);
    assert_eq!(memory.borrow().read(0xABCD), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f_new);
  }

  #[test_case(0x01, 0x00, 0x10, 0xD0; "zero flag set correctly and carry not affected")]
  #[test_case(0x10, 0x0F, 0x00, 0x60; "half carry set correctly")]
  fn decrement_reg(value: u8, result: u8, f_old: u8, f_new: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, f_old);
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0x15);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::D), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f_new);
  }

  #[test_case(0x01, 0x00, 0x10, 0xD0; "zero flag set correctly and carry not affected")]
  #[test_case(0x10, 0x0F, 0x00, 0x60; "half carry set correctly")]
  fn decrement_indirect_hl(value: u8, result: u8, f_old: u8, f_new: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, f_old);
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0x35);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(3);
    assert_eq!(memory.borrow().read(0xABCD), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f_new);
  }

  #[test_case(0xF01E, 0xF028, 0xE046, 0x80, 0x90; "carry set correctly and zero flag not affected")]
  #[test_case(0x1E1E, 0x2828, 0x4646, 0x80, 0xA0; "half carry set correctly")]
  fn add_reg_pair_to_reg_hl(hl: u16, value: u16, result: u16, f_old: u8, f_new: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, f_old);
    cpu.registers.write_word(WordRegister::HL, hl);
    cpu.registers.write_word(WordRegister::DE, value);
    memory.borrow_mut().write(0x0000, 0x19);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_word(WordRegister::HL), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f_new);
  }

  #[test_case(0xFFDA, 0x26, 0x0000, 0x30; "carry set correctly and zero flag set to zero")]
  #[test_case(0x0FDA, 0x26, 0x1000, 0x20; "half carry set correctly")]
  fn add_immediate_to_reg_sp(sp: u16, value: u8, result: u16, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::SP, sp);
    memory.borrow_mut().write(0x0000, 0xE8);
    memory.borrow_mut().write(0x0001, value);
    cpu.ticks(4);
    assert_eq!(cpu.registers.read_word(WordRegister::SP), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0xFFFF, 0x0000; "performs wrapping correctly")]
  #[test_case(0x0FDA, 0x0FDB; "increments correctly")]
  fn increment_reg_pair(sp: u16, result: u16) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, 0xF0);
    cpu.registers.write_word(WordRegister::SP, sp);
    memory.borrow_mut().write(0x0000, 0x33);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_word(WordRegister::SP), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0xF0);
  }

  #[test_case(0x0000, 0xFFFF; "performs wrapping correctly")]
  #[test_case(0x0FDA, 0x0FD9; "decrements correctly")]
  fn decrement_reg_pair(sp: u16, result: u16) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, 0xF0);
    cpu.registers.write_word(WordRegister::SP, sp);
    memory.borrow_mut().write(0x0000, 0x3B);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_word(WordRegister::SP), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0xF0);
  }

  #[test]
  fn rotate_reg_a_left() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0xCA);
    memory.borrow_mut().write(0x0000, 0x07);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), 0x95);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0x10);
  }

  #[test_case(0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xCA, 0x95, 0x10; "rotates left correctly and sets carry")]
  fn rotate_reg_left(value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x02);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::D), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xCA, 0x95, 0x10; "rotates left correctly and sets carry")]
  fn rotate_indirect_hl_left(value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x06);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(4);
    assert_eq!(memory.borrow().read(0xABCD), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test]
  fn rotate_reg_a_right() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0x53);
    memory.borrow_mut().write(0x0000, 0x0F);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), 0xA9);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0x10);
  }

  #[test_case(0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0x53, 0xA9, 0x10; "rotates right correctly and sets carry")]
  fn rotate_reg_right(value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x0A);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::D), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }


  #[test_case(0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0x53, 0xA9, 0x10; "rotates right correctly and sets carry")]
  fn rotate_indirect_hl_right(value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x0E);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(4);
    assert_eq!(memory.borrow().read(0xABCD), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test]
  fn rotate_reg_a_left_through_carry() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0x4A);
    cpu.registers.write_byte(ByteRegister::F, 0x10);
    memory.borrow_mut().write(0x0000, 0x17);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), 0x95);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0x00);
  }

  #[test_case(0x80, 0x00, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0x4A, 0x95, 0x10, 0x00; "rotates left correctly and sets carry")]
  fn rotate_reg_left_through_carry(value: u8, result: u8, old_f: u8, new_f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::D, value);
    cpu.registers.write_byte(ByteRegister::F, old_f);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x12);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::D), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), new_f);
  }

  #[test_case(0x80, 0x00, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0x4A, 0x95, 0x10, 0x00; "rotates left correctly and sets carry")]
  fn rotate_indirect_hl_left_through_carry(value: u8, result: u8, old_f: u8, new_f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    cpu.registers.write_byte(ByteRegister::F, old_f);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x16);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(4);
    assert_eq!(memory.borrow().read(0xABCD), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), new_f);
  }

  #[test]
  fn rotate_reg_a_right_through_carry() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0x52);
    cpu.registers.write_byte(ByteRegister::F, 0x10);
    memory.borrow_mut().write(0x0000, 0x1F);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::A), 0xA9);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0x00);
  }

  #[test_case(0x01, 0x00, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0x52, 0xA9, 0x10, 0x00; "rotates right correctly and sets carry")]
  fn rotate_reg_right_through_carry(value: u8, result: u8, old_f: u8, new_f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::D, 0x52);
    cpu.registers.write_byte(ByteRegister::F, 0x10);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x1A);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::D), 0xA9);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0x00);
  }

  #[test_case(0x01, 0x00, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0x52, 0xA9, 0x10, 0x00; "rotates right correctly and sets carry")]
  fn rotate_indirect_hl_right_through_carry(value: u8, result: u8, old_f: u8, new_f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    cpu.registers.write_byte(ByteRegister::F, old_f);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x1E);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(4);
    assert_eq!(memory.borrow().read(0xABCD), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), new_f);
  }

  #[test_case(0x80, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0xCA, 0x94, 0x10; "shifts left correctly and sets carry")]
  fn shift_reg_left(value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x22);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::D), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x80, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0xCA, 0x94, 0x10; "shifts left correctly and sets carry")]
  fn shift_indirect_hl_left(value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x26);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(4);
    assert_eq!(memory.borrow().read(0xABCD), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x01, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0x53, 0x29, 0x10; "shifts right correctly and sets carry")]
  fn shift_reg_right(value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x3A);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::D), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x01, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0x53, 0x29, 0x10; "shifts right correctly and sets carry")]
  fn shift_indirect_hl_right(value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x3E);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(4);
    assert_eq!(memory.borrow().read(0xABCD), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x01, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0xA2, 0xD1, 0x00; "shifts right correctly")]
  fn shift_reg_right_arithmetic(value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x2A);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::D), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x01, 0x00, 0x90; "zero flag set correctly")]
  #[test_case(0xA2, 0xD1, 0x00; "shifts right correctly")]
  fn shift_indirect_hl_right_arithmetic(value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x2E);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(4);
    assert_eq!(memory.borrow().read(0xABCD), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xA6, 0x6A, 0x00; "swaps correctly")]
  fn swap_reg(value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::D, value);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x32);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_byte(ByteRegister::D), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test_case(0x00, 0x00, 0x80; "zero flag set correctly")]
  #[test_case(0xA6, 0x6A, 0x00; "swaps correctly")]
  fn swap_indirect_hl(value: u8, result: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xCB);
    memory.borrow_mut().write(0x0001, 0x36);
    memory.borrow_mut().write(0xABCD, value);
    cpu.ticks(4);
    assert_eq!(memory.borrow().read(0xABCD), result);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), f);
  }

  #[test]
  fn get_reg_bit() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::D, 0xA5);
    let bits: Vec<(bool, u8)> = (0u8..8u8).map(|bit| {
      memory.borrow_mut().write((2 * bit) as u16, 0xCB);
      memory.borrow_mut().write((2 * bit + 1) as u16, 0x42 | (bit << 3));
      cpu.ticks(2);
      (!cpu.registers.read_byte(ByteRegister::F).get_bit(7), bit)
    }).collect();
    let result = u8::compose(&bits);
    assert_eq!(result, 0xA5);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0x20);
  }

  #[test]
  fn get_indirect_hl_bit() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0xABCD, 0xA5);
    let bits: Vec<(bool, u8)> = (0u8..8u8).map(|bit| {
      memory.borrow_mut().write((2 * bit) as u16, 0xCB);
      memory.borrow_mut().write((2 * bit + 1) as u16, 0x46 | (bit << 3));
      cpu.ticks(3);
      (!cpu.registers.read_byte(ByteRegister::F).get_bit(7), bit)
    }).collect();
    let result = u8::compose(&bits);
    assert_eq_hex!(result, 0xA5);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0x20);
  }

  #[test]
  fn set_reg_bit() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, 0xB0);
    [0, 2, 5, 7].iter().enumerate().for_each(|(index, bit)| {
      memory.borrow_mut().write((2 * index) as u16, 0xCB);
      memory.borrow_mut().write((2 * index + 1) as u16, 0xC2 | (bit << 3));
      cpu.ticks(2);
    });
    assert_eq!(cpu.registers.read_byte(ByteRegister::D), 0xA5);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0xB0);
  }

  #[test]
  fn set_indirect_hl_bit() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    cpu.registers.write_byte(ByteRegister::F, 0xB0);
    [0, 2, 5, 7].iter().enumerate().for_each(|(index, bit)| {
      memory.borrow_mut().write((2 * index) as u16, 0xCB);
      memory.borrow_mut().write((2 * index + 1) as u16, 0xC6 | (bit << 3));
      cpu.ticks(4);
    });
    assert_eq!(memory.borrow().read(0xABCD), 0xA5);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0xB0);
  }

  #[test]
  fn reset_reg_bit() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::D, 0xFF);
    cpu.registers.write_byte(ByteRegister::F, 0xB0);
    [1, 3, 4, 6].iter().enumerate().for_each(|(index, bit)| {
      memory.borrow_mut().write((2 * index) as u16, 0xCB);
      memory.borrow_mut().write((2 * index + 1) as u16, 0x82 | (bit << 3));
      cpu.ticks(2);
    });
    assert_eq!(cpu.registers.read_byte(ByteRegister::D), 0xA5);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0xB0);
  }

  #[test]
  fn reset_indirect_hl_bit() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0xABCD, 0xFF);
    cpu.registers.write_byte(ByteRegister::F, 0xB0);
    [1, 3, 4, 6].iter().enumerate().for_each(|(index, bit)| {
      memory.borrow_mut().write((2 * index) as u16, 0xCB);
      memory.borrow_mut().write((2 * index + 1) as u16, 0x86 | (bit << 3));
      cpu.ticks(4);
    });
    assert_eq_hex!(memory.borrow().read(0xABCD), 0xA5);
    assert_eq_hex!(cpu.registers.read_byte(ByteRegister::F), 0xB0);
  }

  #[test]
  fn jump() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    memory.borrow_mut().write(0x0000, 0xC3);
    memory.borrow_mut().write(0x0001, 0xCD);
    memory.borrow_mut().write(0x0002, 0xAB);
    cpu.ticks(4);

    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0xABCD);
  }

  #[test_case(0x00, 0x70; "jumps when zero flag not set")]
  #[test_case(0x01, 0x80; "jumps when zero flag set")]
  #[test_case(0x02, 0xE0; "jumps when carry not set")]
  #[test_case(0x03, 0x10; "jumps when carry set")]
  fn jump_conditional(condition: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, !f);
    memory.borrow_mut().write(0x0000, 0xC2 | (condition << 3));
    memory.borrow_mut().write(0x0001, 0xCD);
    memory.borrow_mut().write(0x0002, 0xAB);
    memory.borrow_mut().write(0x0003, 0xC2 | (condition << 3));
    memory.borrow_mut().write(0x0004, 0xCD);
    memory.borrow_mut().write(0x0005, 0xAB);
    cpu.ticks(3);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0x0003);

    cpu.registers.write_byte(ByteRegister::F, f);
    cpu.ticks(4);

    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0xABCD);
  }

  #[test]
  fn jump_relative() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    memory.borrow_mut().write(0x0000, 0x18);
    memory.borrow_mut().write(0x0001, 0x08);
    memory.borrow_mut().write(0x000A, 0x18);
    memory.borrow_mut().write(0x000B, 0xFC);
    cpu.ticks(6);

    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0x0008);
  }

  #[test_case(0x00, 0x70; "jumps when zero flag not set")]
  #[test_case(0x01, 0x80; "jumps when zero flag set")]
  #[test_case(0x02, 0xE0; "jumps when carry not set")]
  #[test_case(0x03, 0x10; "jumps when carry set")]
  fn jump_conditional_relative(condition: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, !f);
    memory.borrow_mut().write(0x0000, 0x20 | (condition << 3));
    memory.borrow_mut().write(0x0001, 0x08);
    memory.borrow_mut().write(0x0002, 0x20 | (condition << 3));
    memory.borrow_mut().write(0x0003, 0x08);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0x0002);

    cpu.registers.write_byte(ByteRegister::F, f);
    cpu.ticks(3);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0x000C);
  }

  #[test]
  fn jump_indirect_hl() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::HL, 0xABCD);
    memory.borrow_mut().write(0x0000, 0xE9);
    cpu.tick();

    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0xABCD);
  }

  #[test]
  fn call() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::SP, 0xFFFE);
    cpu.registers.write_word(WordRegister::PC, 0x1234);
    memory.borrow_mut().write(0x1234, 0xCD);
    memory.borrow_mut().write(0x1235, 0xCD);
    memory.borrow_mut().write(0x1236, 0xAB);
    cpu.ticks(6);

    assert_eq!(cpu.registers.read_word(WordRegister::SP), 0xFFFC);
    assert_eq!(memory.borrow().read(0xFFFD), 0x12);
    assert_eq!(memory.borrow().read(0xFFFC), 0x37);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0xABCD);
  }

  #[test_case(0x00, 0x70; "calls when zero flag not set")]
  #[test_case(0x01, 0x80; "calls when zero flag set")]
  #[test_case(0x02, 0xE0; "calls when carry not set")]
  #[test_case(0x03, 0x10; "calls when carry set")]
  fn call_conditional(condition: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::SP, 0xFFFE);
    cpu.registers.write_word(WordRegister::PC, 0x1234);
    cpu.registers.write_byte(ByteRegister::F, !f);
    memory.borrow_mut().write(0x1234, 0xC4 | (condition << 3));
    memory.borrow_mut().write(0x1235, 0xCD);
    memory.borrow_mut().write(0x1236, 0xAB);
    memory.borrow_mut().write(0x1237, 0xC4 | (condition << 3));
    memory.borrow_mut().write(0x1238, 0xCD);
    memory.borrow_mut().write(0x1239, 0xAB);

    cpu.ticks(3);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0x1237);

    cpu.registers.write_byte(ByteRegister::F, f);
    cpu.ticks(6);

    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0xABCD);
    assert_eq!(cpu.registers.read_word(WordRegister::SP), 0xFFFC);
    assert_eq!(memory.borrow().read(0xFFFD), 0x12);
    assert_eq!(memory.borrow().read(0xFFFC), 0x3A);
  }

  #[test]
  fn return_from_call() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::SP, 0xFFFE);
    cpu.registers.write_word(WordRegister::PC, 0x1234);
    memory.borrow_mut().write(0x1234, 0xCD);
    memory.borrow_mut().write(0x1235, 0xCD);
    memory.borrow_mut().write(0x1236, 0xAB);
    memory.borrow_mut().write(0xABCD, 0xC9);
    cpu.ticks(6);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0xABCD);

    cpu.ticks(4);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0x1237);
    assert_eq!(cpu.registers.read_word(WordRegister::SP), 0xFFFE);
  }

  #[test]
  fn return_from_interrupt() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::SP, 0xFFFE);
    cpu.registers.write_word(WordRegister::PC, 0x1234);
    memory.borrow_mut().write(0x1234, 0xCD);
    memory.borrow_mut().write(0x1235, 0xCD);
    memory.borrow_mut().write(0x1236, 0xAB);
    memory.borrow_mut().write(0xABCD, 0xF3);
    memory.borrow_mut().write(0xABCE, 0xD9);
    cpu.ticks(6);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0xABCD);

    cpu.tick();
    assert_eq!(cpu.ime, false);

    cpu.ticks(4);
    assert_eq!(cpu.ime, true);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0x1237);
    assert_eq!(cpu.registers.read_word(WordRegister::SP), 0xFFFE);
  }

  #[test_case(0x00, 0x70; "returns when zero flag not set")]
  #[test_case(0x01, 0x80; "returns when zero flag set")]
  #[test_case(0x02, 0xE0; "returns when carry not set")]
  #[test_case(0x03, 0x10; "returns when carry set")]
  fn return_conditionally(condition: u8, f: u8) {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::SP, 0xFFFE);
    cpu.registers.write_word(WordRegister::PC, 0x1234);
    memory.borrow_mut().write(0x1234, 0xCD);
    memory.borrow_mut().write(0x1235, 0xCD);
    memory.borrow_mut().write(0x1236, 0xAB);
    memory.borrow_mut().write(0xABCD, 0xC0 | (condition << 3));
    memory.borrow_mut().write(0xABCE, 0xC0 | (condition << 3));
    cpu.ticks(6);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0xABCD);

    cpu.registers.write_byte(ByteRegister::F, !f);
    cpu.ticks(2);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0xABCE);

    cpu.registers.write_byte(ByteRegister::F, f);
    cpu.ticks(5);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), 0x1237);
    assert_eq!(cpu.registers.read_word(WordRegister::SP), 0xFFFE);
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
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_word(WordRegister::SP, 0xFFFE);
    cpu.registers.write_word(WordRegister::PC, 0x1234);
    memory.borrow_mut().write(0x1234, 0xC7 | (operand << 3));
    cpu.ticks(4);

    assert_eq!(cpu.registers.read_word(WordRegister::SP), 0xFFFC);
    assert_eq!(memory.borrow().read(0xFFFD), 0x12);
    assert_eq!(memory.borrow().read(0xFFFC), 0x35);
    assert_eq!(cpu.registers.read_word(WordRegister::PC), address);
  }

  #[test]
  fn decimal_adjust_reg_a() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    let mut instruction_index = 0u16;
    (0u8..99u8).for_each(|x| {
      (0u8..99u8).for_each(|y| {
        let sum = x + y;
        let difference = 100 + x - y;
        let a = (x % 10) | ((x / 10) << 4);
        let d = (y % 10) | ((y / 10) << 4);
        let f = u8::compose(&[(sum % 100 == 0, 7), (sum >= 100, 4)]);
        if a == 0x1 && d == 0x9 {
          println!("Invalid result. A: {:#x}, D: {:#x}, F: {:#x}", a, d, f);
        }

        cpu.registers.write_byte(ByteRegister::A, a);
        cpu.registers.write_byte(ByteRegister::D, d);
        memory.borrow_mut().write(instruction_index, 0x82);
        instruction_index += 1;
        cpu.tick();
        memory.borrow_mut().write(instruction_index, 0x27);
        instruction_index += 1;
        cpu.tick();
        let result_bcd_sum = cpu.registers.read_byte(ByteRegister::A);
        let result_decimal_sum = ((result_bcd_sum & 0xF0) >> 4) * 10 + (result_bcd_sum & 0x0F);
        assert_eq!(result_decimal_sum, sum % 100);
        assert_eq_hex!(cpu.registers.read_byte(ByteRegister::F) & 0xB0, f);

        cpu.registers.write_byte(ByteRegister::A, a);
        cpu.registers.write_byte(ByteRegister::D, d);
        memory.borrow_mut().write(instruction_index, 0x92);
        instruction_index += 1;
        cpu.tick();
        memory.borrow_mut().write(instruction_index, 0x27);
        instruction_index += 1;
        cpu.tick();
        let result_bcd_diff = cpu.registers.read_byte(ByteRegister::A);
        let result_decimal_diff = ((result_bcd_diff & 0xF0) >> 4) * 10 + (result_bcd_diff & 0x0F);
        let f = u8::compose(&[(difference % 100 == 0, 7), (difference < 100, 4)]);
        assert_eq_hex!(cpu.registers.read_byte(ByteRegister::F) & 0xB0, f);
        assert_eq!(result_decimal_diff, difference % 100);
      })
    })
  }

  #[test]
  fn ones_complement_reg_a() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::A, 0xA6);
    cpu.registers.write_byte(ByteRegister::F, 0x90);
    memory.borrow_mut().write(0x0000, 0x2F);
    cpu.tick();

    assert_eq!(cpu.registers.read_byte(ByteRegister::A), 0x59);
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0xF0);
  }

  #[test]
  fn flip_carry() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, 0x80);
    memory.borrow_mut().write(0x0000, 0x3F);
    memory.borrow_mut().write(0x0001, 0x3F);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0x90);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0x80);
  }

  #[test]
  fn set_carry() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.registers.write_byte(ByteRegister::F, 0x80);
    memory.borrow_mut().write(0x0000, 0x37);
    cpu.tick();
    assert_eq!(cpu.registers.read_byte(ByteRegister::F), 0x90);
  }

  #[test]
  fn disable_enable_interrupts() {
    let mut memory: MemoryRef = Rc::new(RefCell::new(Box::new(MockMemory::new())));
    let mut cpu = CPU::new(Rc::clone(&memory));
    cpu.ime = true;
    memory.borrow_mut().write(0x0000, 0xF3);
    memory.borrow_mut().write(0x0001, 0xFB);
    cpu.tick();
    assert_eq!(cpu.ime, false);
    cpu.tick();
    assert_eq!(cpu.ime, true);
  }
}
