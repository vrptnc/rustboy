use crate::memory::memory::{Memory, MemoryAddress};

pub struct WRAMImpl {
  bytes: [u8; (8 * WRAMImpl::BANK_SIZE) as usize],
  bank_index: u8
}

impl WRAMImpl {
  const START_ADDRESS: u16 = 0xC000;
  const END_ADDRESS: u16 = 0xDFFF;
  const BANK_SIZE: u16 = 0x1000;
  const BANK_0_END_ADDRESS: u16 = 0xCFFF;
  const DYNAMIC_BANK_START_ADDRESS: u16 = 0xD000;

  pub fn new() -> WRAMImpl {
    WRAMImpl {
      bytes: [0; (8 * WRAMImpl::BANK_SIZE) as usize],
      bank_index: 1,
    }
  }
}

impl Memory for WRAMImpl {
  fn read(&self, address: u16) -> u8 {
    match address {
      WRAMImpl::START_ADDRESS..=WRAMImpl::BANK_0_END_ADDRESS => {
        self.bytes[(address - WRAMImpl::START_ADDRESS) as usize]
      }
      WRAMImpl::DYNAMIC_BANK_START_ADDRESS..=WRAMImpl::END_ADDRESS => {
        self.bytes[(self.bank_index as u16 * WRAMImpl::BANK_SIZE + address - WRAMImpl::DYNAMIC_BANK_START_ADDRESS) as usize]
      },
      MemoryAddress::SVBK => self.bank_index,
      _ => panic!("Can't read address {} from WRAM", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      WRAMImpl::START_ADDRESS..=WRAMImpl::BANK_0_END_ADDRESS => {
        self.bytes[(address - WRAMImpl::START_ADDRESS) as usize] = value;
      }
      WRAMImpl::DYNAMIC_BANK_START_ADDRESS..=WRAMImpl::END_ADDRESS => {
        self.bytes[(self.bank_index as u16 * WRAMImpl::BANK_SIZE + address - WRAMImpl::DYNAMIC_BANK_START_ADDRESS) as usize] = value;
      },
      MemoryAddress::SVBK => {
        self.bank_index = value & 0x07;
        if self.bank_index == 0 {
          self.bank_index = 1;
        }
      },
      _ => panic!("Can't write to address {} in WRAM", address)
    }
  }
}

