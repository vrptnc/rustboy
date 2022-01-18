use crate::memory::memory::Memory;

struct MBC1 {
  ram_enabled: bool,
  ram_banking_mode: bool,
  bank1: usize,
  bank2: usize,
  rom: Vec<u8>,
  ram: Vec<u8>,
}

impl MBC1 {
  fn new(rom_size: usize, ram_size: usize) -> MBC1 {
    MBC1 {
      ram_enabled: false,
      ram_banking_mode: false,
      bank1: 0x01,
      bank2: 0x00,
      ram: vec![0; ram_size],
      rom: vec![0; rom_size],
    }
  }
}

impl Memory for MBC1 {
  fn read(&self, address: usize) -> u8 {
    match address {
      0x0000..=0x3FFF => {
        let address_in_rom = (address & 0x1FFF) | (if self.ram_banking_mode { self.bank2 << 19 } else { 0 });
        self.rom[address_in_rom]
      }
      0x4000..=0x7FFF => {
        let address_in_rom = (address & 0x1FFF) | (self.bank1 << 14) | (self.bank2 << 19);
        self.rom[address_in_rom]
      }
      0xA000..=0xBFFF => {
        let address_in_ram = (address & 0x1FFF) | (if self.ram_banking_mode { self.bank2 << 13 } else { 0 });
        self.ram[address_in_ram]
      }
      _ => panic!("Can't read from address {} on MBC1", address)
    }
  }

  fn write(&mut self, address: usize, value: u8) {
    match address {
      0x0000..=0x1FFF => {
        self.ram_enabled = (value & 0x0F) == 0x0A;
      }
      0x2000..=0x3FFF => {
        self.bank1 = (value & 0x1F) as usize;
        if self.bank1 == 0 {
          self.bank1 = 1;
        }
      }
      0x4000..=0x5FFF => {
        self.bank2 = (value & 0x03) as usize;
      }
      0x6000..=0x7FFF => {
        self.ram_banking_mode = (value & 0x01) == 0x01;
      }
      0xA000..=0xBFFF => {
        if self.ram_enabled {
          let address_in_ram = (address & 0x1FFF) | (if self.ram_banking_mode { self.bank2 << 13 } else { 0 });
          self.ram[address_in_ram] = value;
        }
      }
      _ => panic!("Can't write to address {} on MBC1", address)
    };
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::memory::memory::test::MockMemory;
  use test_case::test_case;

  #[test]
  fn read_write_ram() {
    let mut memory = MBC1::new(0x80 * 0x4000, 4 * 0x4000);
    memory.write()

  }
}