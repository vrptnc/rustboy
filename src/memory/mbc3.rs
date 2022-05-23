use crate::context::context::{Context, Executable};
use crate::time::duration::Duration;
use crate::memory::mbc::Loadable;
use crate::memory::memory::{Memory, RAMSize, ROMSize};
use crate::time::time::{ClockAware, TimingAware};
use crate::util::bit_util::BitUtil;

#[derive(Copy, Clone)]
pub struct RTCFields {
  pub seconds: u8,
  pub minutes: u8,
  pub hours: u8,
  pub days_low: u8,
  pub days_high: u8,
}

impl RTCFields {
  pub fn new() -> RTCFields {
    RTCFields {
      seconds: 0,
      minutes: 0,
      hours: 0,
      days_low: 0,
      days_high: 0,
    }
  }

  fn is_halted(&self) -> bool {
    self.days_high.get_bit(6)
  }

  pub fn tick(&self, duration: Duration) -> RTCFields {
    if self.is_halted() {
      *self
    } else {
      let new_duration = self.to_duration() + duration;
      let rtc_duration = new_duration.to_rtc_duration();
      let days_carry = self.days_high.get_bit(7) || rtc_duration.days >= 512;
      RTCFields {
        seconds: rtc_duration.seconds,
        minutes: rtc_duration.minutes,
        hours: rtc_duration.hours,
        days_low: (rtc_duration.days % 256) as u8,
        days_high: (if rtc_duration.days >= 256 && rtc_duration.days < 512 { 1u8 } else { 0u8 }) | (if days_carry { 0x80u8 } else { 0u8 }),
      }
    }
  }

  pub fn to_duration(&self) -> Duration {
    Duration {
      nanoseconds: 0,
      seconds: self.seconds as u64 +
        60 * self.minutes as u64 +
        3600 * self.hours as u64 +
        86400 * self.days_low as u64 +
        86400 * 256 * (self.days_high & 0x01) as u64,
    }
  }
}

struct MBC3 {
  rtc: RTCFields,
  rtc_registers: RTCFields,
  clock_counter_data_latch: bool,
  ram_enabled: bool,
  rom_bank_address: usize,
  ram_bank_address: usize,
  rom: Vec<u8>,
  ram: Vec<u8>,
}

impl MBC3 {
  fn new(rom_size: ROMSize, ram_size: RAMSize) -> MBC3 {
    MBC3 {
      rtc: RTCFields::new(),
      rtc_registers: RTCFields::new(),
      clock_counter_data_latch: false,
      ram_enabled: false,
      rom_bank_address: 0x01,
      ram_bank_address: 0x00,
      ram: vec![0; ram_size.bytes()],
      rom: vec![0; rom_size.bytes()],
    }
  }

  fn latch_counter_data(&mut self) {
    self.rtc_registers = self.rtc;
  }
}

impl ClockAware for MBC3 {
  fn tick(&mut self) {
    self.rtc = self.rtc.tick(Duration::from_nanoseconds(1000));
  }
}

impl Memory for MBC3 {
  fn read(&self, address: usize) -> u8 {
    match address {
      0x0000..=0x3FFF => {
        self.rom[address]
      }
      0x4000..=0x7FFF => {
        let address_in_rom = (address & 0x3FFF) | (self.rom_bank_address << 14);
        self.rom[address_in_rom]
      }
      0xA000..=0xBFFF => {
        match self.ram_bank_address {
          0x0..=0x7 => {
            let address_in_ram = (self.ram_bank_address << 13) | (address & 0x1FFF);
            self.ram[address_in_ram]
          }
          0x8 => self.rtc_registers.seconds,
          0x9 => self.rtc_registers.minutes,
          0xA => self.rtc_registers.hours,
          0xB => self.rtc_registers.days_low,
          0xC => self.rtc_registers.days_high,
          _ => panic!("{:#06x} is not a valid RAM bank address", self.ram_bank_address)
        }
      }
      _ => panic!("Can't read from address {:#06x} on MBC3", address)
    }
  }

  fn write(&mut self, address: usize, value: u8) {
    match address {
      0x0000..=0x1FFF => {
        self.ram_enabled = (value & 0x0F) == 0x0A;
      }
      0x2000..=0x3FFF => {
        self.rom_bank_address = value as usize;
        if self.rom_bank_address == 0 {
          self.rom_bank_address = 1;
        }
      }
      0x4000..=0x5FFF if value <= 0x0C => {
        self.ram_bank_address = (value & 0x0F) as usize;
      }
      0x6000..=0x7FFF => {
        let new_value = (value & 1u8) == 1;
        if new_value & !self.clock_counter_data_latch {
          self.latch_counter_data();
        }
        self.clock_counter_data_latch = new_value
      }
      0xA000..=0xBFFF => {
        if self.ram_enabled {
          match self.ram_bank_address {
            0x0..=0x7 => {
              let address_in_ram = (self.ram_bank_address << 13) | (address & 0x1FFF);
              self.ram[address_in_ram] = value;
            }
            0x8 => {
              self.rtc_registers.seconds = value;
              self.rtc.seconds = value;
            }
            0x9 => {
              self.rtc_registers.minutes = value;
              self.rtc.minutes = value;
            }
            0xA => {
              self.rtc_registers.hours = value;
              self.rtc.hours = value;
            }
            0xB => {
              self.rtc_registers.days_low = value;
              self.rtc.days_low = value;
            }
            0xC => {
              self.rtc_registers.days_high = value;
              self.rtc.days_high = value;
            }
            _ => panic!("{:#06x} is not a valid RAM bank address", self.ram_bank_address)
          };
        }
      }
      _ => panic!("Can't write to address {:#06x} on MBC3", address)
    };
  }
}

impl Loadable for MBC3 {
  fn load_byte(&mut self, index: usize, value: u8) {
    self.rom[index] = value;
  }

  fn load_bytes(&mut self, index: usize, values: &[u8]) {
    self.rom.as_mut_slice()[index..(index + values.len())].copy_from_slice(values);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use assert_hex::assert_eq_hex;
  use crate::time::duration::RTCDuration;

  #[test]
  fn read_write_ram() {
    let mut memory = MBC3::new(ROMSize::KB256, RAMSize::KB32);
    memory.write(0x0000, 0xA); // Enable RAM
    memory.write(0xA000, 0xAB);
    memory.write(0xA080, 0xCD);
    memory.write(0xA1FF, 0xEF);
    assert_eq_hex!(memory.read(0xA000), 0xAB);
    assert_eq_hex!(memory.read(0xA080), 0xCD);
    assert_eq_hex!(memory.read(0xA1FF), 0xEF);
  }

  #[test]
  fn ram_enabled_register_blocks_writes() {
    let mut memory = MBC3::new(ROMSize::KB256, RAMSize::KB32);
    memory.write(0x0000, 0xA); // Enable RAM
    memory.write(0xA080, 0xAB);
    memory.write(0x0000, 0xB); // Disable RAM
    memory.write(0xA080, 0xCD);
    assert_eq_hex!(memory.read(0xA080), 0xAB);
  }

  #[test]
  fn read_lower_rom() {
    let mut memory = MBC3::new(ROMSize::KB256, RAMSize::KB32);
    memory.load_byte(0x0000, 0x12);
    memory.load_byte(0x2ABC, 0x34);
    memory.load_byte(0x3FFF, 0x56);
    assert_eq_hex!(memory.read(0x0000), 0x12);
    assert_eq_hex!(memory.read(0x2ABC), 0x34);
    assert_eq_hex!(memory.read(0x3FFF), 0x56);
  }

  #[test]
  fn read_upper_rom() {
    let mut memory = MBC3::new(ROMSize::KB256, RAMSize::KB32);
    memory.load_byte(0x4000, 0x12);
    memory.load_byte(0x5ABC, 0x34);
    memory.load_byte(0x7FFF, 0x56);
    memory.load_byte(0x14000, 0x78); // Load bytes into bank 5
    memory.load_byte(0x15ABC, 0x9A);
    memory.load_byte(0x17FFF, 0xBC);
    assert_eq_hex!(memory.read(0x4000), 0x12);
    assert_eq_hex!(memory.read(0x5ABC), 0x34);
    assert_eq_hex!(memory.read(0x7FFF), 0x56);
    memory.write(0x3000, 0x05);
    // Switch to bank 5
    assert_eq_hex!(memory.read(0x4000), 0x78);
    assert_eq_hex!(memory.read(0x5ABC), 0x9A);
    assert_eq_hex!(memory.read(0x7FFF), 0xBC);
  }

  #[test]
  fn rom_bank_address_is_never_zero() {
    let mut memory = MBC3::new(ROMSize::KB256, RAMSize::KB32);
    memory.write(0x3000, 0x00);
    memory.load_byte(0x4000, 0x12);
    memory.load_byte(0x5ABC, 0x34);
    memory.load_byte(0x7FFF, 0x56);
    assert_eq_hex!(memory.read(0x4000), 0x12);
    assert_eq_hex!(memory.read(0x5ABC), 0x34);
    assert_eq_hex!(memory.read(0x7FFF), 0x56);
  }

  #[test]
  fn read_write_rtc() {
    let mut memory = MBC3::new(ROMSize::KB256, RAMSize::KB32);
    memory.write(0x0000, 0xA); // Enable RAM
    memory.write(0x4000, 0x08); // Set RAM bank to RTC seconds
    memory.write(0xA000, 56); // Write 56 seconds
    memory.write(0x4000, 0x09); // Set RAM bank to RTC minutes
    memory.write(0xA000, 34); // Write 34 minutes
    memory.write(0x4000, 0x0A); // Set RAM bank to RTC hours
    memory.write(0xA000, 12); // Write 12 hours
    memory.write(0x4000, 0x0B); // Set RAM bank to RTC days low
    memory.write(0xA000, 105); // Write 105 days low
    memory.write(0x4000, 0x0C); // Set RAM bank to RTC days high
    memory.write(0xA000, 0x81); // Write 768 days high (non-halted)
    memory.write(0x0000, 0xB); // Disable RAM
    memory.write(0x4000, 0x08); // Set RAM bank to RTC seconds
    assert_eq!(memory.read(0xA000), 56); // Read seconds
    memory.write(0x4000, 0x09); // Set RAM bank to RTC minutes
    assert_eq!(memory.read(0xA000), 34); // Read minutes
    memory.write(0x4000, 0x0A); // Set RAM bank to RTC hours
    assert_eq!(memory.read(0xA000), 12); // Read hours
    memory.write(0x4000, 0x0B); // Set RAM bank to RTC days low
    assert_eq!(memory.read(0xA000), 105); // Read days low
    memory.write(0x4000, 0x0C);
    // Set RAM bank to RTC days high
    assert_eq_hex!(memory.read(0xA000), 0x81); // Read days high (non-halted)
  }

  #[test]
  fn tick_rtc() {
    let mut memory = MBC3::new(ROMSize::KB256, RAMSize::KB32);
    memory.write(0x0000, 0xA); // Enable RAM
    memory.write(0x4000, 0x08); // Set RAM bank to RTC seconds
    memory.write(0xA000, 56); // Write 56 seconds
    memory.write(0x4000, 0x09); // Set RAM bank to RTC minutes
    memory.write(0xA000, 34); // Write 34 minutes
    memory.write(0x4000, 0x0A); // Set RAM bank to RTC hours
    memory.write(0xA000, 12); // Write 12 hours
    memory.write(0x4000, 0x0B); // Set RAM bank to RTC days low
    memory.write(0xA000, 105); // Write 105 days low
    memory.write(0x4000, 0x0C); // Set RAM bank to RTC days high
    memory.write(0xA000, 0x01); // Write 361 days high (non-halted)
    memory.write(0x0000, 0xB); // Disable RAM
    memory.tick();
    memory.write(0x4000, 0x08); // Set RAM bank to RTC seconds
    assert_eq!(memory.read(0xA000), 56); // Read seconds
    memory.write(0x4000, 0x09); // Set RAM bank to RTC minutes
    assert_eq!(memory.read(0xA000), 34); // Read minutes
    memory.write(0x4000, 0x0A); // Set RAM bank to RTC hours
    assert_eq!(memory.read(0xA000), 12); // Read hours
    memory.write(0x4000, 0x0B); // Set RAM bank to RTC days low
    assert_eq!(memory.read(0xA000), 105); // Read days low
    memory.write(0x4000, 0x0C);
    // Set RAM bank to RTC days high
    assert_eq_hex!(memory.read(0xA000), 0x01); // Read days high (non-halted)
    memory.write(0x6000, 0x00);
    memory.write(0x6000, 0x01);
    memory.write(0x4000, 0x08); // Set RAM bank to RTC seconds
    assert_eq!(memory.read(0xA000), 38); // Read seconds
    memory.write(0x4000, 0x09); // Set RAM bank to RTC minutes
    assert_eq!(memory.read(0xA000), 02); // Read minutes
    memory.write(0x4000, 0x0A); // Set RAM bank to RTC hours
    assert_eq!(memory.read(0xA000), 2); // Read hours
    memory.write(0x4000, 0x0B); // Set RAM bank to RTC days low
    assert_eq!(memory.read(0xA000), 0x05); // Read days low
    memory.write(0x4000, 0x0C);
    // Set RAM bank to RTC days high
    assert_eq_hex!(memory.read(0xA000), 0x80); // Read days high (non-halted)
  }
}