use std::cell::RefCell;
use std::ops::Index;
use std::rc::Rc;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use crate::memory::memory::Memory;
use crate::renderer::renderer::{Color, ColorIndex, PaletteIndex};
use crate::util::bit_util::BitUtil;

const COLORS_PER_PALETTE: usize = 4;
const NUMBER_OF_PALETTES: usize = 8;

pub type CRAMRef = Rc<RefCell<CRAM>>;

pub struct CRAM {
  background_palette_index: u8,
  background_palettes: [u8; 2 * COLORS_PER_PALETTE * NUMBER_OF_PALETTES],
  object_palette_index: u8,
  object_palettes: [u8; 2 * COLORS_PER_PALETTE * NUMBER_OF_PALETTES],
}

impl CRAM {
  pub fn new() -> CRAM {
    CRAM {
      background_palette_index: 0,
      background_palettes: [0; 2 * COLORS_PER_PALETTE * NUMBER_OF_PALETTES],
      object_palette_index: 0,
      object_palettes: [0; 2 * COLORS_PER_PALETTE * NUMBER_OF_PALETTES],
    }
  }

  pub fn get_background_color(&self, palette_index: PaletteIndex, color_index: ColorIndex) -> Color {
    let lower_byte_address = ((palette_index << 3) | (color_index << 1)) as usize;
    let color_word = (&self.background_palettes[lower_byte_address..=lower_byte_address + 1]).read_u16::<LittleEndian>().unwrap();
    Color::from_word(color_word)
  }

  pub fn get_object_color(&self, palette_index: PaletteIndex, color_index: ColorIndex) -> Color {
    let lower_byte_address = ((palette_index << 3) | (color_index << 1)) as usize;
    let color_word = (&self.object_palettes[lower_byte_address..=lower_byte_address + 1]).read_u16::<LittleEndian>().unwrap();
    Color::from_word(color_word)
  }
}

impl Memory for CRAM {
  fn read(&self, address: u16) -> u8 {
    match address {
      0xFF68 => self.background_palette_index,
      0xFF69 => self.background_palettes[(self.background_palette_index & 0x3F) as usize],
      0xFF6A => self.object_palette_index,
      0xFF6B => self.object_palettes[(self.object_palette_index & 0x3F) as usize],
      _ => panic!("Unable to read address {} from CRAM", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      0xFF68 => self.background_palette_index = value & 0xBF,
      0xFF69 => {
        self.background_palettes[(self.background_palette_index & 0x3F) as usize] = value;
        if self.background_palette_index.get_bit(7) { // Auto-increment bcps
          // By clearing bit 6 (which is unused) after increment,
          // we prevent incrementing into the higher bits and allow the index to wrap back to 0
          self.background_palette_index = (self.background_palette_index + 1).reset_bit(6);
        }
      }
      0xFF6A => self.object_palette_index = value & 0xBF,
      0xFF6B => {
        self.object_palettes[(self.object_palette_index & 0x3F) as usize] = value;
        if self.object_palette_index.get_bit(7) { // Auto-increment bcps
          // By clearing bit 6 (which is unused) after increment,
          // we prevent incrementing into the higher bits and allow the index to wrap back to 0
          self.object_palette_index = (self.object_palette_index + 1).reset_bit(6);
        }
      }
      _ => panic!("Unable to read address {} from CRAM", address)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use test_case::test_case;

  #[test_case(0x0FF68, 0xFF69; "background color")]
  #[test_case(0x0FF68, 0xFF69; "object color")]
  fn writes_color_to_correct_location(index_address: u16, data_address: u16) {
    let mut cram = CRAM::new();
    cram.write(index_address, 0x34);
    cram.write(data_address, 0xD5);
    cram.write(index_address, 0x35);
    cram.write(data_address, 0x2B);
    cram.write(index_address, 0x34);
    assert_eq!(cram.read(data_address), 0xD5);
    cram.write(index_address, 0x35);
    assert_eq!(cram.read(data_address), 0x2B);
  }

  #[test_case(0x0FF68, 0xFF69; "background color")]
  #[test_case(0x0FF68, 0xFF69; "object color")]
  fn writes_color_with_auto_increment(index_address: u16, data_address: u16) {
    let mut cram = CRAM::new();
    cram.write(index_address, 0xB4);
    cram.write(data_address, 0xD5);
    cram.write(data_address, 0x2B);
    cram.write(index_address, 0x34);
    assert_eq!(cram.read(data_address), 0xD5);
    cram.write(index_address, 0x35);
    assert_eq!(cram.read(data_address), 0x2B);
  }

  #[test]
  fn get_background_color_returns_correct_color() {
    let mut cram = CRAM::new();
    cram.write(0xFF68, 0xB4);
    cram.write(0xFF69, 0xD5);
    cram.write(0xFF69, 0x2B);
    let color = cram.get_background_color(6, 2);
    assert_eq!(color.red, 0x15); // Red
    assert_eq!(color.green, 0x1E); // Green
    assert_eq!(color.blue, 0x0A); // Blue
  }

  #[test]
  fn get_object_color_returns_correct_color() {
    let mut cram = CRAM::new();
    cram.write(0xFF6A, 0xB4);
    cram.write(0xFF6B, 0xD5);
    cram.write(0xFF6B, 0x2B);
    let color = cram.get_object_color(6, 2);
    assert_eq!(color.red, 0x15); // Red
    assert_eq!(color.green, 0x1E); // Green
    assert_eq!(color.blue, 0x0A); // Blue
  }
}

