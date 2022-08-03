use crate::memory::memory::Memory;

const START_ADDRESS: u16 = 0xFF80;
const END_ADDRESS: u16 = 0xFFFE;
const STACK_SIZE: usize = 127;

pub struct Stack {
  bytes: [u8; STACK_SIZE],
}

impl Stack {
  pub fn new() -> Stack {
    Stack {
      bytes: [0; STACK_SIZE]
    }
  }
}

impl Memory for Stack {
  fn read(&self, address: u16) -> u8 {
    match address {
      START_ADDRESS..=END_ADDRESS => self.bytes[(address - START_ADDRESS) as usize],
      _ => panic!("Can't read address {} from stack", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      START_ADDRESS..=END_ADDRESS => self.bytes[(address - START_ADDRESS) as usize] = value,
      _ => panic!("Can't write to address {} in stack", address)
    }
  }
}