use crate::memory::main::{MainMemory, Register};

struct Opcode {
  opcode: u8,
}

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

struct CPU {}


impl CPU {
  fn new() -> CPU {
    CPU {}
  }

  fn execute(&mut self, opcode: Opcode, memory: &mut MainMemory) {
    let operation = match opcode.value() {
      0x00 => CPU::noop,
      0x40..=0x45 => CPU::reg_to_reg_ld,
      0x47..=0x4D => CPU::reg_to_reg_ld,
      0x4F => CPU::reg_to_reg_ld,
      0x50..=0x55 => CPU::reg_to_reg_ld,
      0x57..=0x5D => CPU::reg_to_reg_ld,
      0x5F => CPU::reg_to_reg_ld,
      0x60..=0x65 => CPU::reg_to_reg_ld,
      0x67..=0x6D => CPU::reg_to_reg_ld,
      0x6F => CPU::reg_to_reg_ld,
      0x78..=0x7D => CPU::reg_to_reg_ld,
      0x7F => CPU::reg_to_reg_ld,
      _ => panic!("Unknown opcode"),
    };
    operation(self, opcode, memory)
  }

  fn noop(&mut self, _opcode: Opcode, _memory: &mut MainMemory) {}

  fn reg_to_reg_ld(&mut self, opcode: Opcode, memory: &mut MainMemory) {
    let src = Register::from_r_bits(opcode.z_bits());
    let dest = Register::from_r_bits(opcode.y_bits());
    memory.write_register(dest, memory.read_register(src));
  }

  fn immediate_to_reg_ld(&mut self, opcode: Opcode, memory: &mut MainMemory) {

  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn reg_to_reg_ld() {
    let mut cpu = CPU::new();
    let mut memory = MainMemory::new();
    memory.write_register(Register::L, 0xAB);
    cpu.execute(Opcode::new(0x45), &mut memory);
    assert_eq!(memory.read_register(Register::B), 0xAB);
  }
}
