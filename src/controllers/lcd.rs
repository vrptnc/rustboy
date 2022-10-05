use std::cell::RefCell;
use std::rc::Rc;
use std::thread::current;
use web_sys::window;
use crate::cpu::interrupts::{Interrupt, InterruptControllerRef};
use crate::memory::cram::CRAMRef;
use crate::memory::oam::{OAMObject, OAMRef};
use crate::memory::memory::{CGBMode, Memory, MemoryRef};
use crate::memory::vram::{Tile, TileMapView, VRAMRef};
use crate::renderer::renderer::{Point, RendererRef, TileAddressingMode, TileMapIndex};
use crate::time::time::ClockAware;
use crate::util::bit_util::BitUtil;

pub type LCDControllerRef = Rc<RefCell<LCDController>>;

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

pub struct LCDController {
  current_object_index: u8,
  intersecting_object_indices: Vec<u8>,
  interrupt_controller: InterruptControllerRef,
  renderer: Option<RendererRef>,
  cram: Option<CRAMRef>,
  vram: Option<VRAMRef>,
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
  cgb_mode: CGBMode
}

impl LCDController {
  pub fn new(cgb_mode: CGBMode, interrupt_controller: InterruptControllerRef) -> LCDController {
    LCDController {
      current_object_index: 0,
      intersecting_object_indices: vec![],
      interrupt_controller,
      renderer: None,
      cram: None,
      vram: None,
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
      cgb_mode
    }
  }

  pub fn set_renderer(&mut self, renderer: RendererRef) {
    self.renderer = Some(renderer);
  }

  pub fn set_oam(&mut self, oam: OAMRef) {
    self.oam = Some(oam);
  }

  pub fn set_cram(&mut self, cram: CRAMRef) {
    self.cram = Some(cram);
  }

  pub fn set_vram(&mut self, vram: VRAMRef) {
    self.vram = Some(vram);
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
    let current_line = self.current_line();
    let use_8_x_16_tiles = self.lcdc.use_8_x_16_tiles();
    let oam = self.oam.as_ref().unwrap().borrow();
    let object_index_for_dot = ((self.dot % 456) / 2) as u8;
    while self.current_object_index <= object_index_for_dot && self.intersecting_object_indices.len() < 10 {
      if oam.object_intersects_with_line(self.current_object_index, current_line, use_8_x_16_tiles) {
        self.intersecting_object_indices.push(self.current_object_index);
      }
      self.current_object_index += 1;
    }
  }

  fn draw_background_line(&self) {
    let current_line = self.current_line();
    let vram = self.vram.as_ref().unwrap().borrow();
    let renderer = self.renderer.as_ref().unwrap().borrow();
    let tile_map = vram.tile_map(self.lcdc.bg_tile_map_index());
    let tile_data_view = vram.tile_data(self.lcdc.bg_and_window_tile_addressing_mode());

    let tile_column_offset = self.scx / 8;
    let pixel_column_offset = self.scx % 8;
    let pixel_row = (current_line + self.scy) % 144;
    let pixel_row_offset = pixel_row % 8;

    tile_map.row(pixel_row)
      .cycle()
      .skip(tile_column_offset as usize)
      .enumerate()
      .flat_map(|(tile_index, Tile { chr_code, attributes })| tile_data_view
        .get_tile_data(attributes.tile_bank_index(), chr_code)
        .get_color_indices(pixel_row_offset, attributes.flip_horizontal(), attributes.flip_vertical())
        .skip(if tile_index == 0 { pixel_column_offset as usize } else { 0 })
        .map(move |color_index| {
          self.cram.as_ref().unwrap().borrow().get_background_color(attributes.palette_index(), color_index)
        })
      )
      .take(160)
      .enumerate()
      .for_each(|(x, color)| {
        renderer.draw_pixel(x as u8, current_line, color, false);
      });
  }

  fn should_draw_window_line(&self) -> bool {
    self.wy >= self.current_line() &&
      self.wy < 144 &&
      self.wx >= 7 &&
      self.wx - 7 < 160
  }

  fn draw_window_line(&self) {
    if self.lcdc.windowing_enabled() && self.should_draw_window_line() {
      let current_line = self.current_line();
      let vram = self.vram.as_ref().unwrap().borrow();
      let renderer = self.renderer.as_ref().unwrap().borrow();
      let tile_map = vram.tile_map(self.lcdc.window_tile_map_index());
      let tile_data_view = vram.tile_data(self.lcdc.bg_and_window_tile_addressing_mode());

      let pixel_row = (current_line - self.wy);
      let pixel_row_offset = pixel_row % 8;
      let window_pixel_column = self.wx - 7;
      let pixels_to_draw = 160 - window_pixel_column;

      tile_map.row(pixel_row)
        .flat_map(|Tile { chr_code, attributes }| tile_data_view
          .get_tile_data(attributes.tile_bank_index(), chr_code)
          .get_color_indices(pixel_row_offset, attributes.flip_horizontal(), attributes.flip_vertical())
          .map(move |color_index| {
            self.cram.as_ref().unwrap().borrow().get_background_color(attributes.palette_index(), color_index)
          })
        )
        .take(pixels_to_draw as usize)
        .enumerate()
        .for_each(|(x, color)| {
          renderer.draw_pixel(window_pixel_column + x as u8, current_line, color, false);
        });
    }
  }

  fn draw_obj_line(&self) {
    let oam = self.oam.as_ref().unwrap().borrow();
    let vram = self.vram.as_ref().unwrap().borrow();
    let tile_data_view = vram.tile_data(TileAddressingMode::Mode8000);

    let objects: Vec<OAMObject> = self.intersecting_object_indices.iter()
      .map(|obj_index| oam.get_object(*obj_index))
      .collect();
    // objects.sort_by(|a, b| {
    //
    // })


    //   .flat_map(|obj| tile_data_view.get_tile_data())
  }

  fn draw_line(&self) {
    // 1) Draw background
    self.draw_background_line();
    // 2) Draw window line
    self.draw_window_line();
    // 3) Draw OBJ
    self.draw_obj_line();
  }

  pub fn tick(&mut self) {
    self.handle_tick(false);
  }

  pub fn double_tick(&mut self) {
    self.handle_tick(true);
  }

  pub fn handle_tick(&mut self, double_speed: bool) {
    /*
     * The LCD works with a dot clock, that ticks at the clock frequency.
     * The LCD works with 154 scanlines of 456 dots each = 70224 dots per frame
     * The LCD is only 160 x 144 pixels wide, so scanlines 144-153 are the VBlank period.
     * The 456 dots per scanline consist of 80 dots spent in mode 2 (searching the OAM for viable objects that intersect the current scanline),
     * 168-291 dots spent in mode 3 (rendering the image), and the remaining dots spent in HBlank
     */
    let number_of_dots_for_tick = if double_speed { 2u32 } else { 4u32 };
    self.dot = (self.dot + number_of_dots_for_tick) % DOTS_PER_FRAME;
    match self.get_mode() {
      LCDMode::HBlank => {
        if self.current_column() == 248 {
          self.intersecting_object_indices.clear();
        }
      }
      LCDMode::VBlank => {
        if self.current_column() == 0 {
          self.interrupt_controller.as_ref().borrow_mut().request_interrupt(Interrupt::VerticalBlank)
        }
      }
      LCDMode::Mode2 => {
        self.find_intersecting_objects()
      }
      LCDMode::Mode3 => {
        todo!("Either only call this once for the current line or progressively draw the line");
        self.draw_line()
      }
    }
  }
}