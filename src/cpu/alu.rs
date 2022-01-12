use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crate::util::bit_util::BitUtil;

pub struct Result {
  pub value: u8,
  pub carry: bool,
  pub half_carry: bool,
  pub zero: bool
}

pub struct ALU {
}

impl ALU {
  pub fn add(operand1: u8, operand2: u8) -> Result {
    ALU::add_with_carry(operand1, operand2, false)
  }

  pub fn add_with_carry(operand1: u8, operand2: u8, carry: bool) -> Result {
    let result = (operand1 as u16) + (operand2 as u16) + if carry {1} else {0};
    let carry = (operand1 as u16) ^ (operand2 as u16) ^ result;
    let truncated_result = result as u8;
    Result {
      value: truncated_result,
      carry: carry.get_bit(8),
      half_carry: carry.get_bit(4),
      zero: truncated_result == 0
    }
  }

  pub fn subtract(operand1: u8, operand2: u8) -> Result {
    ALU::subtract_with_carry(operand1, operand2, false)
  }

  pub fn subtract_with_carry(operand1: u8, operand2: u8, carry: bool) -> Result {
    let result = 0x100u16 + (operand1 as u16) - (operand2 as u16) - if carry {1} else {0};
    let borrow = (0x100u16 + operand1 as u16) ^ (operand2 as u16) ^ result;
    let truncated_result = result as u8;
    Result {
      value: truncated_result,
      carry: borrow.get_bit(8),
      half_carry: borrow.get_bit(4),
      zero: truncated_result == 0
    }
  }

  pub fn and(operand1: u8, operand2: u8) -> Result {
    let result = operand1 & operand2;
    Result {
      value: result,
      carry: false,
      half_carry: true,
      zero: result == 0
    }
  }

  pub fn or(operand1: u8, operand2: u8) -> Result {
    let result = operand1 | operand2;
    Result {
      value: result,
      carry: false,
      half_carry: false,
      zero: result == 0
    }
  }

  pub fn xor(operand1: u8, operand2: u8) -> Result {
    let result = operand1 ^ operand2;
    Result {
      value: result,
      carry: false,
      half_carry: false,
      zero: result == 0
    }
  }
}