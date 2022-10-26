use crate::memory::memory::Memory;
use crate::memory::oam::OAMImpl;
use crate::memory::vram::VRAMImpl;
use crate::memory::wram::WRAM;

pub struct DMAMemoryView<'a> {
  rom: &'a dyn Memory,
  vram: &'a mut VRAMImpl,
  wram: &'a WRAM,
  oam: &'a mut OAMImpl
}

impl<'a> Memory for DMAMemoryView<'a> {
  fn read(&self, address: u16) -> u8 {
    match address {
      0x0000..=0x7FFF => self.rom.read(address),
      0x8000..=0x9FFF => self.vram.read(address),
      0xA000..=0xBFFF => self.rom.read(address),
      0xC000..=0xDFFF => self.wram.read(address),
      0xFE00..=0xFE9F => self.oam.read(address),
      _ => panic!("DMA does not have read access to memory at address {:#06x}", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      0x8000..=0x9FFF => self.vram.write(address, value),
      0xFE00..=0xFE9F => self.oam.write(address, value),
      _ => panic!("DMA does not have write access to memory at address {:#06x}", address)
    }
  }
}