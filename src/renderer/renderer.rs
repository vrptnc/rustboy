use std::cell::RefCell;
use std::rc::Rc;
use mockall::automock;

pub struct Point {
  pub x: u8,
  pub y: u8
}

pub enum TileMapIndex {
  TileMap1,
  TileMap2
}

pub enum TileAddressingMode {
  Mode8000,
  Mode8800
}

pub type PaletteIndex = u8;
pub type ColorIndex = u8;

#[derive(Copy, Clone)]
pub struct Color {
  pub red: u8,
  pub green: u8,
  pub blue: u8,
}

impl Color {
  pub fn from_word(color_word: u16) -> Color {
    Color {
      red: (color_word & 0x1F) as u8,
      green: ((color_word & 0x3E0) >> 5) as u8,
      blue: ((color_word & 0x7C00) >> 10) as u8
    }
  }
}

#[automock]
pub trait Renderer {
  fn draw_pixel(&self, x: u8, y: u8, color: Color, draw_in_back: bool);
}