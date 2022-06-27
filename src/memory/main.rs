use crate::memory::bank_memory::BankMemory;
use crate::memory::linear_memory::LinearMemory;
use crate::memory::memory::Memory;

pub struct MainMemory<T> where T: Memory {
  rom: T,
  vram: BankMemory<0x2000, 2>, // Two banks of 8k VRAM memory, switched by VBK register (0xFF4F)
  ram: LinearMemory<0x1000>, //Bank 0 of RAM
  ram_banks: BankMemory<0x1000,7>, // Seven banks of switchable 4k RAM, switched by SVBK register (0xFF70),
  reserved_area_1: LinearMemory<0x1E00>, // In theory, this area is prohibited, but let's map it anyway
  oam: LinearMemory<0xA0>,
  reserved_area_2: LinearMemory<0x60>, // In theory, this area is prohibited, but let's map it anyway
  control_registers: LinearMemory<0x80>,
  stack: LinearMemory<127>,
  interrupt_enable: u8
}

impl<T> Memory for MainMemory<T> where T: Memory {
  fn read(&self, address: u16) -> u8 {
    match address {
      0x0000..=0x7FFF => self.rom.read(address),
      0x8000..=0x9FFF => self.vram.read(address - 0x8000),
      0xA000..=0xBFFF => self.rom.read(address),
      0xC000..=0xCFFF => self.ram.read(address - 0xC000),
      0xD000..=0xDFFF => self.ram_banks.read(address - 0xD000),
      0xE000..=0xFDFF => self.reserved_area_1.read(address - 0xE000),
      0xFE00..=0xFEBF => self.oam.read(address - 0xFE00),
      0xFEA0..=0xFEFF => self.reserved_area_2.read(address - 0xFEA0),
      0xFF00..=0xFF7F => self.control_registers.read(address - 0xFF00),
      0xFF80..=0xFFFE => self.stack.read(address - 0xFF80),
      0xFFFF => self.interrupt_enable,
      _ => panic!("Trying to read value from main memory at unmapped address {:#06x}", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      0x0000..=0x7FFF => self.rom.write(address, value),
      0x8000..=0x9FFF => self.vram.write(address - 0x8000, value),
      0xA000..=0xBFFF => self.rom.write(address, value),
      0xC000..=0xCFFF => self.ram.write(address - 0xC000, value),
      0xD000..=0xDFFF => self.ram_banks.write(address - 0xD000, value),
      0xE000..=0xFDFF => self.reserved_area_1.write(address - 0xE000, value),
      0xFE00..=0xFEBF => self.oam.write(address - 0xFE00, value),
      0xFEA0..=0xFEFF => self.reserved_area_2.write(address - 0xFEA0, value),
      0xFF00..=0xFF7F => self.control_registers.write(address - 0xFF00, value),
      0xFF80..=0xFFFE => self.stack.write(address - 0xFF80, value),
      0xFFFF => self.interrupt_enable = value,
      _ => panic!("Trying to write value to main memory at unmapped address {:#06x}", address)
    }
  }
}

impl<T> MainMemory<T> where T: Memory {
  pub fn new(rom: T) -> MainMemory<T> {
    return MainMemory {
      rom,
      vram: BankMemory::<0x2000, 2>::new(),
      ram: LinearMemory::<0x1000>::new(), //Bank 0 of RAM
      ram_banks: BankMemory::<0x1000,7>::new(), // Seven banks of switchable 4k RAM, switched by SVBK register (0xFF70),
      reserved_area_1: LinearMemory::<0x1E00>::new(), // In theory, this area is prohibited, but let's map it anyway
      oam: LinearMemory::<0xA0>::new(),
      reserved_area_2: LinearMemory::<0x60>::new(), // In theory, this area is prohibited, but let's map it anyway
      control_registers: LinearMemory::<0x80>::new(),
      stack: LinearMemory::<127>::new(),
      interrupt_enable: 0
    };
  }
}