use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::util::bit_util::BitUtil;

pub struct Result<T> {
  pub value: T,
  pub carry: bool,
  pub half_carry: bool,
  pub zero: bool,
}

pub struct ALU {}

impl ALU {
  pub fn add(operand1: u8, operand2: u8) -> Result<u8> {
    ALU::add_with_carry(operand1, operand2, false)
  }

  pub fn add_with_carry(operand1: u8, operand2: u8, carry: bool) -> Result<u8> {
    let result = (operand1 as u16) + (operand2 as u16) + (carry as u16);
    let carry = (operand1 as u16) ^ (operand2 as u16) ^ result;
    let truncated_result = result as u8;
    Result {
      value: truncated_result,
      carry: carry.get_bit(8),
      half_carry: carry.get_bit(4),
      zero: truncated_result == 0,
    }
  }

  pub fn add_pair(operand1: u16, operand2: u16) -> Result<u16> {
    let le_bytes1 = operand1.to_le_bytes();
    let le_bytes2 = operand2.to_le_bytes();
    let result1 = ALU::add(le_bytes1[0], le_bytes2[0]);
    let result2 = ALU::add_with_carry(le_bytes1[1], le_bytes2[1], result1.carry);
    let result = (&[result1.value, result2.value][..]).read_u16::<LittleEndian>().unwrap();
    Result {
      value: result,
      carry: result2.carry,
      half_carry: result2.half_carry,
      zero: result == 0,
    }
  }

  pub fn subtract(operand1: u8, operand2: u8) -> Result<u8> {
    ALU::subtract_with_carry(operand1, operand2, false)
  }

  pub fn subtract_with_carry(operand1: u8, operand2: u8, carry: bool) -> Result<u8> {
    let result = 0x100u16 + (operand1 as u16) - (operand2 as u16) - (carry as u16);
    let borrow = (0x100u16 + operand1 as u16) ^ (operand2 as u16) ^ result;
    let truncated_result = result as u8;
    Result {
      value: truncated_result,
      carry: borrow.get_bit(8),
      half_carry: borrow.get_bit(4),
      zero: truncated_result == 0,
    }
  }

  pub fn subtract_pair(operand1: u16, operand2: u16) -> Result<u16> {
    let le_bytes1 = operand1.to_le_bytes();
    let le_bytes2 = operand2.to_le_bytes();
    let result1 = ALU::subtract(le_bytes1[0], le_bytes2[0]);
    let result2 = ALU::subtract_with_carry(le_bytes1[1], le_bytes2[1], result1.carry);
    let result = (&[result1.value, result2.value][..]).read_u16::<LittleEndian>().unwrap();
    Result {
      value: result,
      carry: result2.carry,
      half_carry: result2.half_carry,
      zero: result == 0,
    }
  }

  pub fn and(operand1: u8, operand2: u8) -> Result<u8> {
    let result = operand1 & operand2;
    Result {
      value: result,
      carry: false,
      half_carry: true,
      zero: result == 0,
    }
  }

  pub fn or(operand1: u8, operand2: u8) -> Result<u8> {
    let result = operand1 | operand2;
    Result {
      value: result,
      carry: false,
      half_carry: false,
      zero: result == 0,
    }
  }

  pub fn xor(operand1: u8, operand2: u8) -> Result<u8> {
    let result = operand1 ^ operand2;
    Result {
      value: result,
      carry: false,
      half_carry: false,
      zero: result == 0,
    }
  }

  pub fn rotate_left(value: u8) -> Result<u8> {
    let result = value.rotate_left(1);
    Result {
      value: result,
      half_carry: false,
      zero: result == 0,
      carry: result % 2 == 1,
    }
  }

  pub fn rotate_left_through_carry(value: u8, carry: bool) -> Result<u8> {
    let result = (value << 1) | (carry as u8);
    Result {
      value: truncated_result,
      half_carry: false,
      zero: truncated_result == 0,
      carry: value.get_bit(7),
    }
  }

  pub fn rotate_right(value: u8) -> Result<u8> {
    let result = value.rotate_right(1);
    Result {
      value: result,
      half_carry: false,
      zero: result == 0,
      carry: value.get_bit(0),
    }
  }

  pub fn rotate_right_through_carry(value: u8, carry: bool) -> Result<u8> {
    let result = (value >> 1) | (if carry { 0x80 } else { 0x00 });
    Result {
      value: result,
      half_carry: false,
      zero: result == 0,
      carry: value.get_bit(0),
    }
  }

  pub fn shift_left(value: u8) -> Result<u8> {
    let result = value << 1;
    Result {
      value: result,
      half_carry: false,
      zero: result == 0,
      carry: value.get_bit(7)
    }
  }

  pub fn shift_right(value: u8) -> Result<u8> {
    let result = value >> 1;
    Result {
      value: result,
      half_carry: false,
      zero: result == 0,
      carry: value.get_bit(0)
    }
  }

  pub fn shift_right_arithmetic(value: u8) -> Result<u8> {
    let result = (value >> 1) | (value & 0x80);
    Result {
      value: result,
      half_carry: false,
      zero: result == 0,
      carry: value.get_bit(0)
    }
  }

  pub fn swap(value: u8) -> Result<u8> {
    let result = value.rotate_left(4);
    Result {
      value: result,
      half_carry: false,
      carry: false,
      zero: result == 0
    }
  }
}