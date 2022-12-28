use mockall::automock;

#[derive(Copy, Clone)]
pub struct Point {
  pub x: u8,
  pub y: u8,
}

#[derive(Copy, Clone)]
pub enum TileMapIndex {
  TileMap1,
  TileMap2,
}

#[derive(Copy, Clone)]
pub enum TileAddressingMode {
  Mode8000,
  Mode8800,
}

#[derive(Copy, Clone, Debug)]
pub struct Color {
  pub red: u8,
  pub green: u8,
  pub blue: u8,
  pub transparent: bool,
}

impl PartialEq for Color {
  fn eq(&self, other: &Self) -> bool {
    self.red == other.red &&
      self.green == other.green &&
      self.blue == other.blue &&
      self.transparent == other.transparent
  }
}

impl Color {
  pub fn from_word(color_word: u16) -> Color {
    Color {
      red: (color_word & 0x1F) as u8,
      green: ((color_word & 0x3E0) >> 5) as u8,
      blue: ((color_word & 0x7C00) >> 10) as u8,
      transparent: false,
    }
  }

  pub const fn from_rgb(red: u8, green: u8, blue: u8) -> Color {
    Color {
      red,
      green,
      blue,
      transparent: false,
    }
  }

  pub fn to_word(&self) -> u16 {
    (self.red & 0x1F) as u16 |
      ((self.green & 0x1F) as u16) << 5 |
      ((self.blue & 0x1F) as u16) << 10
  }

  fn to_5_bit(value: u8) -> u8 {
    value >> 3
  }

  fn to_8_bit(value: u8) -> u8 {
    if value == 0 {
      0
    } else {
      (value << 3) | (0x07)
    }
  }

  pub fn to_rgb555(&self) -> Color {
    Color {
      red: Color::to_5_bit(self.red),
      green: Color::to_5_bit(self.green),
      blue: Color::to_5_bit(self.blue),
      transparent: self.transparent,
    }
  }

  pub fn to_rgb888(&self) -> Color {
    Color {
      red: Color::to_8_bit(self.red),
      green: Color::to_8_bit(self.green),
      blue: Color::to_8_bit(self.blue),
      transparent: self.transparent,
    }
  }

  pub fn transparent() -> Color {
    Color {
      red: 0,
      green: 0,
      blue: 0,
      transparent: true,
    }
  }
}

#[automock]
pub trait Renderer {
  fn draw_pixel(&self, x: u8, y: u8, color: Color, draw_in_back: bool);
}