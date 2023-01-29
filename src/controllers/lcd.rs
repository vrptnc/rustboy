use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

use closure::closure;
use mockall::automock;
use web_sys::console;

use crate::cpu::interrupts::{Interrupt, InterruptController};
use crate::memory::cram::CRAM;
use crate::memory::mbc::MBC;
use crate::memory::mbc::MockROM;
use crate::memory::memory::{CGBMode, Memory, MemoryAddress};
use crate::memory::oam::{OAM, OAMObject, ObjectReference};
use crate::memory::vram::{BackgroundParams, ObjectParams, VRAM, WindowParams};
use crate::renderer::renderer::{Color, Point, Renderer, TileAddressingMode, TileMapIndex};
use crate::util::bit_util::BitUtil;

const DOTS_PER_FRAME: u32 = 70224;

#[derive(Copy, Clone, PartialEq)]
pub enum LCDMode {
  HBlank,
  VBlank,
  Mode2,
  Mode3,
}

struct Stat(u8);

impl Stat {
  pub fn lyc_interrupt_enabled(&self) -> bool {
    self.0.get_bit(6)
  }

  pub fn interrupt_enabled_for_mode(&self, mode: LCDMode) -> bool {
    match mode {
      LCDMode::HBlank => self.0.get_bit(3),
      LCDMode::VBlank => self.0.get_bit(4),
      LCDMode::Mode2 => self.0.get_bit(5),
      LCDMode::Mode3 => false
    }
  }

  pub fn lyc_equals_line(&self) -> bool {
    self.0.get_bit(2)
  }

  pub fn set_lyc_equals_line(&mut self, lyc_equals_line: bool) {
    self.0 = if lyc_equals_line { self.0.set_bit(2) } else { self.0.reset_bit(2) };
  }

  pub fn set_mode(&mut self, mode: LCDMode) {
    let bits: u8 = match mode {
      LCDMode::HBlank => 0x00,
      LCDMode::VBlank => 0x01,
      LCDMode::Mode2 => 0x02,
      LCDMode::Mode3 => 0x03
    };
    self.0 = (self.0 & 0xFC) | bits;
  }
}

struct LCDC(u8);

impl LCDC {
  pub fn bg_priority(&self) -> bool {
    self.0.get_bit(0)
  }

  pub fn obj_enabled(&self) -> bool {
    self.0.get_bit(1)
  }

  pub fn use_8_x_16_tiles(&self) -> bool {
    self.0.get_bit(2)
  }

  pub fn bg_tile_map_index(&self) -> TileMapIndex {
    if self.0.get_bit(3) { TileMapIndex::TileMap2 } else { TileMapIndex::TileMap1 }
  }

  pub fn window_tile_map_index(&self) -> TileMapIndex {
    if self.0.get_bit(6) { TileMapIndex::TileMap2 } else { TileMapIndex::TileMap1 }
  }

  pub fn bg_and_window_tile_data_address(&self) -> u16 {
    if self.0.get_bit(4) { 0x9000 } else { 0x8000 }
  }

  pub fn bg_and_window_tile_addressing_mode(&self) -> TileAddressingMode {
    if self.0.get_bit(4) { TileAddressingMode::Mode8000 } else { TileAddressingMode::Mode8800 }
  }

  pub fn windowing_enabled(&self) -> bool {
    self.0.get_bit(5)
  }

  pub fn lcd_enabled(&self) -> bool {
    self.0.get_bit(7)
  }
}

#[automock]
pub trait LCDController {
  fn get_mode(&self) -> LCDMode;
}

pub struct LCDControllerImpl {
  cgb_mode: CGBMode,
  current_object_index: u8,
  intersecting_object_references: Vec<ObjectReference>,
  dot: u32,
  line: u8,
  line_rendered: bool,
  column: u16,
  mode: LCDMode,
  lcdc: LCDC,
  stat: Stat,
  interrupt_line: bool,
  // The STAT interrupt is triggered on the rising edge of this line (which is the ORed combination of the various sources that can trigger the input)
  opri: u8,
  scy: u8,
  scx: u8,
  lyc: u8,

  wy: u8,
  wx: u8,
}

impl LCDController for LCDControllerImpl {
  fn get_mode(&self) -> LCDMode {
    if self.line >= 144 {
      LCDMode::VBlank
    } else {
      match self.column {
        0..=79 => LCDMode::Mode2,
        80..=247 => LCDMode::Mode3,
        _ => LCDMode::HBlank
      }
    }
  }
}

impl LCDControllerImpl {
  pub fn new(cgb_mode: CGBMode) -> LCDControllerImpl {
    LCDControllerImpl {
      cgb_mode,
      current_object_index: 0,
      intersecting_object_references: vec![],
      dot: 0,
      line: 0,
      line_rendered: false,
      column: 0,
      mode: LCDMode::Mode2,
      lcdc: LCDC(0),
      stat: Stat(0x02), // TODO: Implement writing these registers correctly
      interrupt_line: false,
      opri: 0,
      scy: 0,
      scx: 0,
      lyc: 0,
      wy: 0,
      wx: 0,
    }
  }

  fn find_intersecting_objects(&mut self, oam: &dyn OAM) {
    let use_8_x_16_tiles = self.lcdc.use_8_x_16_tiles();
    let object_index_for_dot = ((self.dot % 456) / 2) as u8;
    while self.current_object_index <= object_index_for_dot && self.intersecting_object_references.len() < 10 {
      if let Some(object_reference) = oam.get_object_reference_if_intersects(self.current_object_index, self.line, use_8_x_16_tiles) {
        self.intersecting_object_references.push(object_reference);
      }
      self.current_object_index += if use_8_x_16_tiles { 2 } else { 1 };
    }
  }

  fn draw_background_line(&self, vram: &dyn VRAM, cram: &dyn CRAM, renderer: &mut dyn Renderer) {
    let color_references = vram.background_line_colors(BackgroundParams {
      tile_map_index: self.lcdc.bg_tile_map_index(),
      tile_addressing_mode: self.lcdc.bg_and_window_tile_addressing_mode(),
      line: self.line,
      viewport_position: Point {
        x: self.scx,
        y: self.scy,
      },
    });
    color_references.into_iter()
      .map(|color_ref| (color_ref, if self.opri == 1 { cram.monochrome_background_color(color_ref) } else { cram.monochrome_background_color(color_ref) }))
      .enumerate()
      .for_each(|(x, (color_ref, color))| {
        let bg_drawing_priority = if color_ref.color_index == 0 || !self.lcdc.bg_priority() {
          0
        } else if color_ref.foreground {
          4
        } else {
          2
        };
        renderer.draw_pixel(x as u8, self.line, color, bg_drawing_priority)
      });
  }

  fn should_draw_window_line(&self) -> bool {
    self.wy >= self.line &&
      self.wy < 144 &&
      self.wx >= 7 &&
      self.wx - 7 < 160
  }

  fn draw_window_line(&self, vram: &dyn VRAM, cram: &dyn CRAM, renderer: &mut dyn Renderer) {
    if self.lcdc.windowing_enabled() && self.should_draw_window_line() {
      let color_references = vram.window_line_colors(WindowParams {
        tile_map_index: self.lcdc.window_tile_map_index(),
        tile_addressing_mode: self.lcdc.bg_and_window_tile_addressing_mode(),
        line: self.line,
        window_position: Point {
          x: self.wx,
          y: self.wy,
        },
      });
      color_references.into_iter()
        .map(|color_ref| if self.opri == 1 { cram.monochrome_background_color(color_ref) } else { cram.background_color(color_ref) })
        .enumerate()
        .for_each(|(x, color)| renderer.draw_pixel(x as u8 + self.wx - 7, self.line, color, 5));
    }
  }

  fn draw_obj_line(&self, vram: &dyn VRAM, cram: &dyn CRAM, oam: &dyn OAM, renderer: &mut dyn Renderer) {
    let mut objects: Vec<OAMObject> = self.intersecting_object_references.iter()
      .map(|object_reference| oam.get_object(*object_reference))
      .collect();
    if self.opri == 1 {
      objects.sort_by(|a, b| {
        if a.lcd_x < b.lcd_x {
          Ordering::Less
        } else if a.lcd_x > b.lcd_x {
          Ordering::Greater
        } else {
          Ordering::Equal
        }
      });
    }

    objects.into_iter().for_each(|object| {
      let params = ObjectParams {
        object,
        row: self.line + 16 - object.lcd_y,
        monochrome: self.opri == 1,
      };
      let colors = vram.object_line_colors(params);
      colors.into_iter()
        .map(|color_ref| (color_ref, if self.opri == 1 { cram.monochrome_object_color(color_ref) } else { cram.object_color(color_ref) }))
        .enumerate()
        .for_each(|(pixel_offset, (color_ref, color))| {
          let obj_drawing_priority = if color_ref.foreground {
            3
          } else {
            1
          };
          renderer.draw_pixel(object.lcd_x + pixel_offset as u8, self.line, color, obj_drawing_priority);
        });
    });
  }

  fn draw_line(&self, vram: &dyn VRAM, cram: &dyn CRAM, oam: &dyn OAM, renderer: &mut dyn Renderer) {
    // 1) Draw background
    self.draw_background_line(vram, cram, renderer);
    // 2) Draw window line
    self.draw_window_line(vram, cram, renderer);
    // 3) Draw OBJ
    self.draw_obj_line(vram, cram, oam, renderer);
  }

  fn update_mode(&mut self) {
    self.mode = if self.line >= 144 {
      LCDMode::VBlank
    } else {
      match self.column {
        0..=79 => LCDMode::Mode2,
        80..=247 => LCDMode::Mode3,
        _ => LCDMode::HBlank
      }
    };
    self.stat.set_mode(self.mode);
  }

  fn maybe_request_interrupt(&mut self, interrupt_controller: &mut dyn InterruptController) {
    let new_interrupt_line =
      self.stat.interrupt_enabled_for_mode(self.mode) ||
        (self.stat.lyc_equals_line() && self.stat.lyc_interrupt_enabled());
    if new_interrupt_line && !self.interrupt_line {
      interrupt_controller.request_interrupt(Interrupt::Stat);
    }
    self.interrupt_line = new_interrupt_line;
  }

  pub fn tick(&mut self, vram: &dyn VRAM, cram: &dyn CRAM, oam: &dyn OAM,
              renderer: &mut dyn Renderer, interrupt_controller: &mut dyn InterruptController,
              double_speed: bool) {
    /*
     * The LCD works with a dot clock, that ticks at the clock frequency.
     * The LCD works with 154 scanlines of 456 dots each = 70224 dots per frame
     * The LCD is only 160 x 144 pixels wide, so scanlines 144-153 are the VBlank period.
     * The 456 dots per scanline consist of 80 dots spent in mode 2 (searching the OAM for viable objects that intersect the current scanline),
     * 168-291 dots spent in mode 3 (rendering the image), and the remaining dots spent in HBlank
     */
    let number_of_dots_for_tick = if double_speed { 2u32 } else { 4u32 };
    self.dot = (self.dot + number_of_dots_for_tick) % DOTS_PER_FRAME;
    self.line = (self.dot / 456) as u8;
    self.column = (self.dot % 456) as u16;
    self.stat.set_lyc_equals_line(self.line == self.lyc);
    self.update_mode();
    self.maybe_request_interrupt(interrupt_controller);

    match self.mode {
      LCDMode::HBlank => {
        if self.column == 248 {
          self.intersecting_object_references.clear();
          self.current_object_index = 0;
        }
      }
      LCDMode::VBlank => {
        if self.column == 0 && self.line == 144 {
          interrupt_controller.request_interrupt(Interrupt::VerticalBlank);
          renderer.flush();
        }
      }
      LCDMode::Mode2 => {
        self.line_rendered = false;
        self.find_intersecting_objects(oam);
      }
      LCDMode::Mode3 => {
        // TODO Examine whether drawing the entire line at once is the best way to go, maybe we should draw it progressively
        if !self.line_rendered {
          self.draw_line(vram, cram, oam, renderer);
          self.line_rendered = true;
        }
      }
    }
  }
}

impl Memory for LCDControllerImpl {
  fn read(&self, address: u16) -> u8 {
    match address {
      MemoryAddress::LCDC => self.lcdc.0,
      MemoryAddress::STAT => self.stat.0,
      MemoryAddress::SCY => self.scy,
      MemoryAddress::SCX => self.scx,
      MemoryAddress::LY => self.line,
      MemoryAddress::LYC => self.lyc,
      MemoryAddress::WY => self.wy,
      MemoryAddress::WX => self.wx,
      MemoryAddress::OPRI => self.opri,
      _ => panic!("Unable to read address {:#x} from LCD Controller", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      MemoryAddress::LCDC => self.lcdc.0 = value,
      MemoryAddress::STAT => self.stat.0 = (self.stat.0 & 0x7) | (value & 0xF8),
      MemoryAddress::SCY => self.scy = value,
      MemoryAddress::SCX => self.scx = value,
      MemoryAddress::LYC => self.lyc = value,
      MemoryAddress::WY => self.wy = value,
      MemoryAddress::WX => self.wx = value,
      MemoryAddress::OPRI => self.opri = value,
      _ => panic!("Unable to write to address {:#x} in LCD Controller", address)
    }
  }
}

#[cfg(test)]
pub mod tests {
  use mockall::predicate::eq;

  use crate::cpu::interrupts::MockInterruptController;
  use crate::memory::cram::MockCRAM;
  use crate::memory::oam::MockOAM;
  use crate::memory::vram::MockVRAM;
  use crate::renderer::renderer::MockRenderer;

  use super::*;

  #[test]
  fn stat_blocking() {
    let mut controller = LCDControllerImpl::new(CGBMode::Color);
    let mut renderer = MockRenderer::new();
    let mut interrupt_controller = MockInterruptController::new();
    interrupt_controller.expect_request_interrupt().never();
    let vram = MockVRAM::new();
    let cram = MockCRAM::new();
    let mut oam = MockOAM::new();
    oam.expect_get_object_reference_if_intersects().return_const(None);
    // Advance to right before HBlank
    for _ in 0..248 {
      controller.tick(&vram, &cram, &oam, &mut renderer, &mut interrupt_controller, false);
    }
    controller.write(MemoryAddress::STAT, 0x28); // Enable STAT interrupt for Mode 2 and HBlank
    interrupt_controller.expect_request_interrupt().with(eq(Interrupt::Stat)).once();
    controller.tick(&vram, &cram, &oam, &mut renderer, &mut interrupt_controller, false); // Enter HBlank
    // Advance to well within Mode 2 of the next line. No additional interrupt should be requested due to STAT blocking
    for _ in 249..500 {
      controller.tick(&vram, &cram, &oam, &mut renderer, &mut interrupt_controller, false);
    }
  }
}