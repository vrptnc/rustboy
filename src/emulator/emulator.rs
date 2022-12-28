use crate::controllers::dma::DMAControllerImpl;
use crate::controllers::lcd::LCDControllerImpl;
use crate::controllers::timer::TimerControllerImpl;
use crate::cpu::interrupts::InterruptControllerImpl;
use crate::memory::control::ControlRegisters;
use crate::memory::linear_memory::LinearMemory;
use crate::memory::mbc0::MBC0;
use crate::memory::mbc1::MBC1;
use crate::memory::mbc2::MBC2;
use crate::memory::mbc3::MBC3;
use crate::memory::mbc5::MBC5;
use crate::memory::mbc::MBC;
use crate::memory::memory::{CGBMode, Memory, RAMSize, ROMSize};
use crate::memory::oam::OAMImpl;
use crate::memory::stack::Stack;
use crate::memory::vram::VRAMImpl;
use crate::memory::wram::WRAMImpl;
use crate::renderer::renderer::Color;
use crate::util::bit_util::BitUtil;

pub struct Emulator {
  rom: Box<dyn MBC>,
  vram: VRAMImpl,
  wram: WRAMImpl,
  oam: OAMImpl,
  lcd: LCDControllerImpl,
  timer: TimerControllerImpl,
  dma: DMAControllerImpl,
  stack: Stack,
  control_registers: ControlRegisters,
  reserved_area_1: LinearMemory::<0x1E00, 0xE000>,
  reserved_area_2: LinearMemory::<0x0060, 0xFEA0>,
  interrupt_controller: InterruptControllerImpl,
}

impl Emulator {
  pub fn new(rom_bytes: &[u8]) -> Emulator {
    let rom_size = ROMSize::from_byte(rom_bytes[0x0148]);
    let ram_size = RAMSize::from_byte(rom_bytes[0x0149]);
    let cgb_mode = CGBMode::from_byte(rom_bytes[0x0143]);
    let rom = Emulator::create_rom(rom_bytes, rom_size, ram_size);
    let vram = VRAMImpl::new();
    let wram = WRAMImpl::new();
    let oam = OAMImpl::new();
    let mut lcd = LCDControllerImpl::new();
    let timer = TimerControllerImpl::new();
    let dma = DMAControllerImpl::new();
    let stack = Stack::new();
    let mut control_registers = ControlRegisters::new();
    let reserved_area_1 = LinearMemory::<0x1E00, 0xE000>::new();
    let reserved_area_2 = LinearMemory::<0x0060, 0xFEA0>::new();
    let interrupt_controller = InterruptControllerImpl::new();

    // If we're in compatibility/color mode, write the compatibility flag as is to KEY0
    // otherwise, write 0x04 to KEY0 and set the OPRI flag on the LCD to 0x01
    if matches!(CGBMode::Color, cgb_mode) {
      control_registers.write(0xFF4C, rom.compatibility_byte());
    } else {
      control_registers.write(0xFF4C, 0x04);
      lcd.write(0xFF6C, 0x01);
    }

    // Write 0x11 to BANK to indicate we're unmapping the boot rom
    control_registers.write(0xFF50, 0x11);

    Emulator {
      rom,
      vram,
      wram,
      oam,
      lcd,
      timer,
      dma,
      stack,
      control_registers,
      reserved_area_1,
      reserved_area_2,
      interrupt_controller,
    }
  }

  fn create_rom(rom_bytes: &[u8], rom_size: ROMSize, ram_size: RAMSize) -> Box<dyn MBC> {
    let mut rom: Box<dyn MBC> = match rom_bytes[0x0147] {
      0x00 => Box::new(MBC0::new(rom_size)),
      0x01..=0x03 => Box::new(MBC1::new(rom_size, ram_size)),
      0x05..=0x06 => Box::new(MBC2::new(rom_size)),
      0x0B..=0x0D => panic!("This emulator currently does not support MMM01 cartridges"),
      0x0F..=0x13 => Box::new(MBC3::new(rom_size, ram_size)),
      0x19..=0x1E => Box::new(MBC5::new(rom_size, ram_size)),
      0x20 => panic!("This emulator currently does not support MBC6 cartridges"),
      0x22 => panic!("This emulator currently does not support MBC7 cartridges"),
      0xFC => panic!("This emulator currently does not support Pocket Camera cartridges"),
      0xFD => panic!("This emulator currently does not support Bandai cartridges"),
      0xFE => panic!("This emulator currently does not support HuC3 cartridges"),
      0xFF => panic!("This emulator currently does not support HuC1 cartridges"),
      _ => panic!("This emulator does not support cartridges with a type byte of {:#x}", rom_bytes[0x0147])
    };
    rom.load_bytes(0x0000, rom_bytes);
    rom
  }

  pub fn run() {
    // let interrupt_controller = Rc::new(RefCell::new(InterruptController::new()));
    // let timer = Rc::new(RefCell::new(TimerController::new(Rc::clone(&interrupt_controller))));
    // let oam = Rc::new(RefCell::new(OAM::new()));
    // let dma: DMAControllerRef = Rc::new(RefCell::new(DMAController::new()));
  }
}