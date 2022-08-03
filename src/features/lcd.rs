use std::cell::RefCell;
use std::rc::Rc;
use std::thread::current;
use crate::cpu::interrupts::{Interrupt, InterruptControllerRef};
use crate::features::oam::OAMRef;
use crate::memory::memory::MemoryRef;
use crate::time::time::ClockAware;
use crate::util::bit_util::BitUtil;

pub type LCDRef = Rc<RefCell<LCD>>;

const DOTS_PER_FRAME: u32 = 70224;

pub enum LCDMode {
  HBlank,
  VBlank,
  Mode2,
  Mode3,
}

struct LCDC(u8);

impl LCDC {
  pub fn bg_enabled(&self) -> bool {
    self.0.get_bit(0)
  }

  pub fn obj_enabled(&self) -> bool {
    self.0.get_bit(1)
  }

  pub fn use_8_x_16_tiles(&self) -> bool {
    self.0.get_bit(2)
  }

  pub fn bg_tile_map_address(&self) -> u16 {
    if self.0.get_bit(3) { 0x9C00 } else { 0x9800 }
  }

  pub fn bg_and_window_tile_data_address(&self) -> u16 {
    if self.0.get_bit(4) { 0x9000 } else { 0x8000 }
  }

  pub fn use_signed_bg_and_window_tile_addressing(&self) -> bool {
    !self.0.get_bit(4)
  }

  pub fn windowing_enabled(&self) -> bool {
    self.0.get_bit(5)
  }

  pub fn window_tile_map_address(&self) -> u16 {
    if self.0.get_bit(6) { 0x9C00 } else { 0x9800 }
  }

  pub fn lcd_enabled(&self) -> bool {
    self.0.get_bit(7)
  }
}

pub struct LCD {
  intersecting_object_indices: Vec<u8>,
  interrupt_controller: InterruptControllerRef,
  memory: Option<MemoryRef>,
  oam: Option<OAMRef>,
  dot: u32,
  lcdc: LCDC,
  stat: u8,
  scy: u8,
  scx: u8,
  ly: u8,
  lyc: u8,
  bgp: u8,
  obp0: u8,
  obp1: u8,
  wy: u8,
  wx: u8,
}

impl LCD {
  pub fn new(interrupt_controller: InterruptControllerRef) -> LCD {
    LCD {
      intersecting_object_indices: vec![],
      interrupt_controller,
      memory: None,
      oam: None,
      dot: 0,
      lcdc: LCDC(0),
      stat: 0,
      scy: 0,
      scx: 0,
      ly: 0,
      lyc: 0,
      bgp: 0,
      obp0: 0,
      obp1: 0,
      wy: 0,
      wx: 0,
    }
  }

  pub fn set_memory(&mut self, memory: MemoryRef) {
    self.memory = Some(memory);
  }

  pub fn set_oam(&mut self, oam: OAMRef) {
    self.oam = Some(oam);
  }

  pub fn current_line(&self) -> u8 {
    (self.dot / 456) as u8
  }

  pub fn current_column(&self) -> u16 {
    (self.dot % 456) as u16
  }

  pub fn get_mode(&self) -> LCDMode {
    if self.current_line() >= 144 {
      LCDMode::VBlank
    } else {
      match self.current_column() {
        0..=79 => LCDMode::Mode2,
        80..=247 => LCDMode::Mode3,
        _ => LCDMode::HBlank
      }
    }
  }

  fn find_intersecting_objects(&mut self) {
    let object_index = ((self.dot % 456) / 2) as u8;
    let current_line = self.current_line();
    let use_8_x_16_tiles = self.lcdc.use_8_x_16_tiles();
    let oam = self.oam.as_ref().unwrap().borrow();
    if oam.object_intersects_with_line(object_index, current_line, use_8_x_16_tiles) && self.intersecting_object_indices.len() < 10 {
      self.intersecting_object_indices.push(object_index);
    }
    if oam.object_intersects_with_line(object_index + 1, current_line, use_8_x_16_tiles) && self.intersecting_object_indices.len() < 10 {
      self.intersecting_object_indices.push(object_index + 1);
    }
  }

  fn draw_line(&self) {
    // 1) Draw background
    let bg_tile_map_address = self.lcdc.bg_tile_map_address();



  }
}

impl ClockAware for LCD {
  fn handle_tick(&mut self, double_speed: bool) {
    /*
     * The LCD works with a dot clock, that ticks at the clock frequency.
     * The LCD works with 154 scanlines of 456 dots each = 70224 dots per frame
     * The LCD is only 160 x 144 pixels wide, so scanlines 144-153 are the VBlank period.
     * The 456 dots per scanline consist of 80 dots spent in mode 2 (searching the OAM for viable objects that intersect the current scanline),
     * 168-291 dots spent in mode 3 (rendering the image), and the remaining dots spent in HBlank
     */
    self.dot = (self.dot + 4) % DOTS_PER_FRAME;
    match self.get_mode() {
      LCDMode::HBlank => {
        if self.current_column() == 248 {
          self.intersecting_object_indices.clear();
        }
      }
      LCDMode::VBlank => {
        if self.current_column() == 0 {
          self.interrupt_controller.borrow_mut().request_interrupt(Interrupt::VerticalBlank)
        }
      }
      LCDMode::Mode2 => {
        self.find_intersecting_objects()
      }
      LCDMode::Mode3 => {}
    }
  }
}