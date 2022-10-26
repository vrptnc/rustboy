use std::borrow::Borrow;
use crate::controllers::dma::DMAController;
use crate::controllers::lcd::LCDController;
use crate::memory::oam::OAM;
use crate::controllers::timer::TimerController;
use crate::memory::bank_memory::BankMemory;
use crate::memory::linear_memory::LinearMemory;
use crate::memory::memory::{CGBMode, Memory};
use crate::memory::stack::Stack;
use crate::memory::vram::VRAMImpl;
use crate::memory::wram::WRAM;

pub struct MainMemory<'a> {
  rom: &'a mut dyn Memory,
  vram: &'a mut dyn Memory,
  wram: &'a mut dyn Memory,
  oam: &'a mut dyn Memory,
  lcd: &'a mut dyn Memory,
  timer: &'a mut dyn Memory,
  dma: &'a mut dyn Memory,
  stack: &'a mut dyn Memory,
  reserved_area_1: &'a mut dyn Memory,
  reserved_area_2: &'a mut dyn Memory,
  interrupt_controller: &'a mut dyn Memory
}

impl<'a> Memory for MainMemory<'a> {
  fn read(&self, address: u16) -> u8 {
    match address {
      0x0000..=0x7FFF => self.rom.read(address),
      0x8000..=0x9FFF => self.vram.read(address),
      0xA000..=0xBFFF => self.rom.read(address),
      0xC000..=0xDFFF => self.wram.read(address),
      0xE000..=0xFDFF => self.reserved_area_1.read(address),
      0xFE00..=0xFE9F => self.oam.read(address),
      0xFEA0..=0xFEFF => self.reserved_area_2.read(address),
      0xFF00..=0xFF03 => 0,
      0xFF04..=0xFF07 => self.timer.read(address),
      0xFF08..=0xFF0E => 0,
      0xFF0F => self.interrupt_controller.read(address),
      0xFF10..=0xFF3F => 0,
      0xFF40..=0xFF45 => self.lcd.read(address),
      0xFF46 => self.dma.read(address),
      0xFF4F => self.vram.read(address),
      0xFF51..=0xFF55 => self.dma.read(address),
      0xFF70 => self.wram.read(address),
      0xFF80..=0xFFFE => self.stack.read(address),
      0xFFFF => self.interrupt_controller.read(0xFFFF),
      _ => panic!("Trying to read value from main memory at unmapped address {:#06x}", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      0x0000..=0x7FFF => self.rom.write(address, value),
      0x8000..=0x9FFF => self.vram.write(address, value),
      0xA000..=0xBFFF => self.rom.write(address, value),
      0xC000..=0xDFFF => self.wram.write(address, value),
      0xE000..=0xFDFF => self.reserved_area_1.write(address - 0xE000, value),
      0xFE00..=0xFEBF => self.oam.write(address, value),
      0xFEA0..=0xFEFF => self.reserved_area_2.write(address - 0xFEA0, value),
      0xFF04..=0xFF07 => self.timer.write(address, value),
      0xFF46 => self.dma.write(address, value),
      0xFF4F => self.vram.write(address, value),
      0xFF51..=0xFF55 => self.dma.write(address, value),
      0xFF70 => self.wram.write(address, value),
      0xFF80..=0xFFFE => self.stack.write(address - 0xFF80, value),
      _ => panic!("Trying to write value to main memory at unmapped address {:#06x}", address)
    }
  }
}