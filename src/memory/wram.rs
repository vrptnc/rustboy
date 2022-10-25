use js_sys::Atomics::add;
use crate::memory::memory::Memory;



pub struct WRAM {
  bytes: [u8; (8 * WRAM::BANK_SIZE) as usize],
  bank_index: u8
}

impl WRAM {
  const START_ADDRESS: u16 = 0xC000;
  const END_ADDRESS: u16 = 0xDFFF;
  const BANK_SIZE: u16 = 0x1000;
  const BANK_0_END_ADDRESS: u16 = 0xCFFF;

  pub fn new() -> WRAM {
    WRAM {
      bytes: [0; (8 * WRAM::BANK_SIZE) as usize],
      bank_index: 1,
    }
  }
}

impl Memory for WRAM {
  fn read(&self, address: u16) -> u8 {
    match address {
      WRAM::START_ADDRESS..=WRAM::BANK_0_END_ADDRESS => {
        self.bytes[(address - WRAM::START_ADDRESS) as usize]
      }
      WRAM::BANK_0_END_ADDRESS..=WRAM::END_ADDRESS => {
        self.bytes[(self.bank_index as u16 * WRAM::BANK_SIZE + address - WRAM::BANK_0_END_ADDRESS) as usize]
      },
      0xFF70 => self.bank_index,
      _ => panic!("Can't read address {} from WRAM", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      WRAM::START_ADDRESS..=WRAM::BANK_0_END_ADDRESS => {
        self.bytes[(address - WRAM::START_ADDRESS) as usize] = value;
      }
      WRAM::BANK_0_END_ADDRESS..=WRAM::END_ADDRESS => {
        self.bytes[(self.bank_index as u16 * WRAM::BANK_SIZE + address - WRAM::BANK_0_END_ADDRESS) as usize] = value;
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

