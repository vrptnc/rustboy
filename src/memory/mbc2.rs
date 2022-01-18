use crate::memory::bank_memory::BankMemory;
use crate::memory::memory::Memory;
use crate::util::bit_util::BitUtil;

struct MBC2 {
  ram_enabled: bool,
  ram_banking_mode: bool,
  rom_bank: usize,
  bank2: usize,
  rom: Vec<u8>,
  ram: Vec<u8>,
}

impl MBC2 {
  fn new(rom_size: usize, ram_size: usize) -> MBC2 {
    MBC2 {
      ram_enabled: false,
      ram_banking_mode: false,
      rom_bank: 0x01,
      bank2: 0x00,
      ram: vec![0; ram_size],
      rom: vec![0; rom_size],
    }
  }
}

impl Memory for MBC2 {
  fn read(&self, address: usize) -> u8 {
    match address {
      0x0000..=0x3FFF => {
        let address_in_rom = (address & 0x1FFF) | (if self.ram_banking_mode {self.bank2 << 19} else {0});
        self.rom[address_in_rom]
      },
      0x4000..=0x7FFF => {
        let address_in_rom = (address & 0x1FFF) | (self.rom_bank << 14) | (self.bank2 << 19);
        self.rom[address_in_rom]
      },
      0xA000..=0xBFFF => {
        let address_in_ram = (address & 0x1FFF) | (if self.ram_banking_mode { self.bank2 << 13 } else { 0 });
        self.ram[address_in_ram]
      },
      _ => panic!("Can't read from address {} on MBC2", address)
    }
  }

  fn write(&mut self, address: usize, value: u8) {
    match address {
      0x0000..=0x3FFF => {
        if address.get_bit(8) {
          self.rom_bank = (value & 0x1F) as usize;
          if self.rom_bank == 0 {
            self.rom_bank = 1;
          }
        } else {
          self.ram_enabled = (value & 0x0F) == 0x0A;
        }
      },
      0x4000..=0x5FFF => {
        self.bank2 = (value & 0x03) as usize;
      },
      0x6000..=0x7FFF => {
        self.ram_banking_mode = (value & 0x01) == 0x01;
      },
      0xA000..=0xBFFF => {
        let address_in_ram = (address & 0x1FFF) | (if self.ram_banking_mode { self.bank2 << 13 } else { 0 });
        self.ram[address_in_ram] = value;
      },
      _ => panic!("Can't write to address {} on MBC2", address)
    };
  }
}