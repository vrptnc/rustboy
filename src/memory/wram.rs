use js_sys::Atomics::add;
use crate::memory::memory::Memory;

const BANK_SIZE: u16 = 0x1000;
const START_ADDRESS: u16 = 0xC000;
const BANK_0_END_ADDRESS: u16 = 0xCFFF;
const END_ADDRESS: u16 = 0xDFFF;

pub struct WRAM {
  bytes: [u8; (8 * BANK_SIZE) as usize],
  bank_index: u8
}

impl WRAM {
  pub fn new() -> WRAM {
    WRAM {
      bytes: [0; (8 * BANK_SIZE) as usize],
      bank_index: 1,
    }
  }
}

impl Memory for WRAM {
  fn read(&self, address: u16) -> u8 {
    match address {
      START_ADDRESS..=BANK_0_END_ADDRESS => {
        self.bytes[(address - START_ADDRESS) as usize]
      }
      BANK_0_END_ADDRESS..=END_ADDRESS => {
        self.bytes[(self.bank_index as u16 * BANK_SIZE + address - BANK_0_END_ADDRESS) as usize]
      },
      0xFF70 => self.bank_index,
      _ => panic!("Can't read address {} from WRAM", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      START_ADDRESS..=BANK_0_END_ADDRESS => {
        self.bytes[(address - START_ADDRESS) as usize] = value;
      }
      BANK_0_END_ADDRESS..=END_ADDRESS => {
        self.bytes[(self.bank_index as u16 * BANK_SIZE + address - BANK_0_END_ADDRESS) as usize] = value;
      },
      0xFF70 => {
        self.bank_index = value & 0x07;
        if self.bank_index == 0 {
          self.bank_index = 1;
        }
      },
      _ => panic!("Can't write to address {} in WRAM", address)
    }
  }
}

