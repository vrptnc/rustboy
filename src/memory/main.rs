use std::borrow::Borrow;
use crate::controllers::dma::DMAControllerRef;
use crate::controllers::lcd::LCDControllerRef;
use crate::memory::oam::OAMRef;
use crate::controllers::timer::TimerControllerRef;
use crate::memory::bank_memory::BankMemory;
use crate::memory::linear_memory::LinearMemory;
use crate::memory::memory::Memory;
use crate::memory::stack::Stack;
use crate::memory::vram::VRAM;
use crate::memory::wram::WRAM;

pub struct MemoryBus<T> where T: Memory {
  rom: T,
  vram: VRAM, // Two banks of 8k VRAM memory, switched by VBK register (0xFF4F)
  wram: WRAM,
  oam: OAMRef,
  lcd: LCDControllerRef,
  timer: TimerControllerRef,
  dma: DMAControllerRef,
  stack: Stack,
  reserved_area_1: LinearMemory<0x1E00, 0xE000>, // In theory, this area is prohibited, but let's map it anyway
  reserved_area_2: LinearMemory<0x60, 0xFEA0>, // In theory, this area is prohibited, but let's map it anyway
  interrupt_enable: u8
}

impl<T> Memory for MemoryBus<T> where T: Memory {
  fn read(&self, address: u16) -> u8 {
    match address {
      0x0000..=0x7FFF => self.rom.read(address),
      0x8000..=0x9FFF => self.vram.read(address),
      0xA000..=0xBFFF => self.rom.read(address),
      0xC000..=0xDFFF => self.wram.read(address),
      0xE000..=0xFDFF => self.reserved_area_1.read(address),
      0xFE00..=0xFE9F => self.oam.as_ref().borrow().read(address),
      0xFEA0..=0xFEFF => self.reserved_area_2.read(address),
      0xFF04..=0xFF07 => self.timer.as_ref().borrow().read(address),
      0xFF46 => self.dma.as_ref().borrow().read(address),
      0xFF4F => self.vram.read(address),
      0xFF51..=0xFF55 => self.dma.as_ref().borrow().read(address),
      0xFF70 => self.wram.read(address),
      0xFF80..=0xFFFE => self.stack.read(address),
      0xFFFF => self.interrupt_enable,
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
      0xFE00..=0xFEBF => self.oam.as_ref().borrow_mut().write(address, value),
      0xFEA0..=0xFEFF => self.reserved_area_2.write(address - 0xFEA0, value),
      0xFF04..=0xFF07 => self.timer.as_ref().borrow_mut().write(address, value),
      0xFF46 => self.dma.as_ref().borrow_mut().write(address, value),
      0xFF4F => self.vram.write(address, value),
      0xFF51..=0xFF55 => self.dma.as_ref().borrow_mut().write(address, value),
      0xFF70 => self.wram.write(address, value),
      0xFF80..=0xFFFE => self.stack.write(address - 0xFF80, value),
      0xFFFF => self.interrupt_enable = value,
      _ => panic!("Trying to write value to main memory at unmapped address {:#06x}", address)
    }
  }
}

impl<T> MemoryBus<T> where T: Memory {
  pub fn new(rom: T, oam: OAMRef, dma: DMAControllerRef, lcd: LCDControllerRef, timer: TimerControllerRef) -> MemoryBus<T> {
    return MemoryBus {
      rom,
      vram: VRAM::new(),
      wram: WRAM::new(),
      stack: Stack::new(),
      oam,
      dma,
      timer,
      lcd,
      reserved_area_1: LinearMemory::<0x1E00, 0xE000>::new(), // In theory, this area is prohibited, but let's map it anyway
      reserved_area_2: LinearMemory::<0x60, 0xFEA0>::new(), // In theory, this area is prohibited, but let's map it anyway
      interrupt_enable: 0
    };
  }
}