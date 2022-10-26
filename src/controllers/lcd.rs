use std::cell::RefCell;
use std::rc::Rc;
use std::thread::current;

use closure::closure;
use mockall::automock;
use web_sys::window;

use crate::cpu::interrupts::{Interrupt, InterruptController, InterruptControllerRef};
use crate::memory::cram::{CRAM, CRAMImpl};
use crate::memory::memory::{CGBMode, Memory};
use crate::memory::oam::{OAM, OAMImpl, OAMObject};
use crate::memory::vram::{Tile, TileMapView, VRAM, VRAMImpl};
use crate::renderer::renderer::{Color, Point, Renderer, TileAddressingMode, TileMapIndex};
use crate::time::time::ClockAware;
use crate::util::bit_util::BitUtil;

const DOTS_PER_FRAME: u32 = 70224;

pub struct LCDDependencies<'a> {
  renderer: &'a mut dyn Renderer,
  interrupt_controller: &'a mut dyn InterruptController,
  cram: &'a dyn CRAM,
  oam: &'a dyn OAM,
  vram: &'a dyn VRAM,
}

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
    self.0 = (self.0 & !0x03) | bits;
  }
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
  current_object_index: u8,
  intersecting_object_indices: Vec<u8>,
  dot: u32,
  line: u8,
  column: u16,
  mode: LCDMode,
  lcdc: LCDC,
  stat: Stat,
  interrupt_line: bool,
  // The STAT interrupt is triggered on the rising edge of this line (which is the ORed combination of the various sources that can trigger the input)
  scy: u8,
  scx: u8,
  lyc: u8,
  bgp: u8,
  obp0: u8,
  obp1: u8,
  wy: u8,
  wx: u8,
  cgb_mode: CGBMode,
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
      current_object_index: 0,
      intersecting_object_indices: vec![],
      dot: 0,
      line: 0,
      column: 0,
      mode: LCDMode::Mode2,
      lcdc: LCDC(0),
      stat: Stat(0x02), // TODO: Implement writing these registers correctly
      interrupt_line: false,
      scy: 0,
      scx: 0,
      lyc: 0,
      bgp: 0,
      obp0: 0,
      obp1: 0,
      wy: 0,
      wx: 0,
      cgb_mode,
    }
  }

  fn find_intersecting_objects(&mut self, dependencies: LCDDependencies) {
    let use_8_x_16_tiles = self.lcdc.use_8_x_16_tiles();
    let object_index_for_dot = ((self.dot % 456) / 2) as u8;
    while self.current_object_index <= object_index_for_dot && self.intersecting_object_indices.len() < 10 {
      if dependencies.oam.object_intersects_with_line(self.current_object_index, self.line, use_8_x_16_tiles) {
        self.intersecting_object_indices.push(self.current_object_index);
      }
      self.current_object_index += 1;
    }
  }

  fn draw_background_line(&self, dependencies: &mut LCDDependencies) {
    let tile_map = dependencies.vram.tile_map(self.lcdc.bg_tile_map_index());
    let tile_data_view = dependencies.vram.tile_data(self.lcdc.bg_and_window_tile_addressing_mode());

    let tile_column_offset = self.scx / 8;
    let pixel_column_offset = self.scx % 8;
    let pixel_row = (self.line + self.scy) % 144;
    let pixel_row_offset = pixel_row % 8;

    tile_map.row(pixel_row)
      .cycle()
      .skip(tile_column_offset as usize)
      .enumerate()
      .flat_map(|(tile_index, Tile { chr_code, attributes })| tile_data_view
        .get_tile_data(attributes.tile_bank_index(), chr_code)
        .get_color_indices(pixel_row_offset, attributes.flip_horizontal(), attributes.flip_vertical())
        .skip(if tile_index == 0 { pixel_column_offset as usize } else { 0 })
        .map(closure!(ref dependencies, move attributes, |color_index| dependencies.cram.get_background_color(attributes.palette_index(), color_index)))
      )
      .take(160)
      .enumerate()
      .for_each(|(x, color)| dependencies.renderer.draw_pixel(x as u8, self.line, color, false));
  }

  fn should_draw_window_line(&self) -> bool {
    self.wy >= self.line &&
      self.wy < 144 &&
      self.wx >= 7 &&
      self.wx - 7 < 160
  }

  fn draw_window_line(&self, dependencies: &mut LCDDependencies) {
    if self.lcdc.windowing_enabled() && self.should_draw_window_line() {
      let tile_map = dependencies.vram.tile_map(self.lcdc.window_tile_map_index());
      let tile_data_view = dependencies.vram.tile_data(self.lcdc.bg_and_window_tile_addressing_mode());

      let pixel_row = (self.line - self.wy);
      let pixel_row_offset = pixel_row % 8;
      let window_pixel_column = self.wx - 7;
      let pixels_to_draw = 160 - window_pixel_column;

      tile_map.row(pixel_row)
        .flat_map(|Tile { chr_code, attributes }| tile_data_view
          .get_tile_data(attributes.tile_bank_index(), chr_code)
          .get_color_indices(pixel_row_offset, attributes.flip_horizontal(), attributes.flip_vertical())
          .map(closure!(ref dependencies, move attributes, |color_index| dependencies.cram.get_background_color(attributes.palette_index(), color_index)))
        )
        .take(pixels_to_draw as usize)
        .enumerate()
        .for_each(|(x, color)| dependencies.renderer.draw_pixel(window_pixel_column + x as u8, self.line, color, false))
    }
  }

  fn draw_obj_line(&self, dependencies: &mut LCDDependencies) {
    let tile_data_view = dependencies.vram.tile_data(TileAddressingMode::Mode8000);

    let objects: Vec<OAMObject> = self.intersecting_object_indices.iter()
      .map(|obj_index| dependencies.oam.get_object(*obj_index))
      .collect();
    // objects.sort_by(|a, b| {
    //
    // })


    //   .flat_map(|obj| tile_data_view.get_tile_data())
  }

  fn draw_line(&self, mut dependencies: LCDDependencies) {
    // 1) Draw background
    self.draw_background_line(&mut dependencies);
    // 2) Draw window line
    self.draw_window_line(&mut dependencies);
    // 3) Draw OBJ
    self.draw_obj_line(&mut dependencies);
  }

  pub fn tick(&mut self, dependencies: LCDDependencies) {
    self.handle_tick(dependencies, false);
  }

  pub fn double_tick(&mut self, dependencies: LCDDependencies) {
    self.handle_tick(dependencies, true);
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

  fn maybe_request_interrupt(&mut self, dependencies: &mut LCDDependencies) {
    let new_interrupt_line =
      self.stat.interrupt_enabled_for_mode(self.mode) ||
        (self.stat.lyc_equals_line() && self.stat.lyc_interrupt_enabled());
    if new_interrupt_line && !self.interrupt_line {
      dependencies.interrupt_controller.request_interrupt(Interrupt::Stat);
    }
    self.interrupt_line = new_interrupt_line;
  }

  pub fn handle_tick(&mut self, mut dependencies: LCDDependencies, double_speed: bool) {
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
    self.maybe_request_interrupt(&mut dependencies);

    match self.mode {
      LCDMode::HBlank => {
        if self.column == 248 {
          self.intersecting_object_indices.clear();
        }
      }
      LCDMode::VBlank => {
        if self.column == 0 {
          dependencies.interrupt_controller.request_interrupt(Interrupt::VerticalBlank);
        }
      }
      LCDMode::Mode2 => {
        self.find_intersecting_objects(dependencies)
      }
      LCDMode::Mode3 => {
        todo!("Either only call this once for the current line or progressively draw the line");
        self.draw_line(dependencies)
      }
    }
  }
}

impl Memory for LCDControllerImpl {
  fn read(&self, address: u16) -> u8 {
    match address {
      0xFF40 => self.lcdc.0,
      0xFF41 => self.stat.0,
      0xFF42 => self.scy,
      0xFF43 => self.scx,
      0xFF44 => self.line,
      0xFF45 => self.lyc,
      0xFF4A => self.wy,
      0xFF4B => self.wx,
      _ => panic!("Unable to read address {:#x} from LCD Controller", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      0xFF40 => self.lcdc.0 = value,
      0xFF41 => self.stat.0 = (self.stat.0 & 0x7) | (value & 0xF8),
      0xFF42 => self.scy = value,
      0xFF43 => self.scx = value,
      0xFF45 => self.lyc = value,
      0xFF47 => self.bgp = value,
      0xFF48 => self.obp0 = value,
      0xFF49 => self.obp1 = value,
      0xFF4A => self.wy = value,
      0xFF4B => self.wx = value,
      _ => panic!("Unable to write to address {:#x} in LCD Controller", address)
    }
  }
}

#[cfg(test)]
pub mod tests {
  use super::*;

  #[test]
  fn stat_blocking() {}
}