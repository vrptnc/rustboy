use byteorder::{LittleEndian, ReadBytesExt};
use mockall::automock;
use crate::memory::memory::Memory;
use crate::renderer::renderer::Color;
use crate::util::bit_util::BitUtil;

const COLORS_PER_PALETTE: usize = 4;
const NUMBER_OF_PALETTES: usize = 8;

#[derive(Copy, Clone)]
pub struct ColorReference {
  pub color_index: u8,
  pub palette_index: u8,
}

#[automock]
pub trait CRAM {
  fn background_color(&self, color_ref: ColorReference) -> Color;
  fn object_color(&self, color_ref: ColorReference) -> Color;
}

pub struct CRAMImpl {
  grayscale_background_palette: u8,
  grayscale_object_palette_0: u8,
  grayscale_object_palette_1: u8,
  background_palette_index: u8,
  background_palettes: [u8; 2 * COLORS_PER_PALETTE * NUMBER_OF_PALETTES],
  object_palette_index: u8,
  object_palettes: [u8; 2 * COLORS_PER_PALETTE * NUMBER_OF_PALETTES],
}

impl CRAMImpl {
  pub fn new() -> CRAMImpl {
    CRAMImpl {
      grayscale_background_palette: 0,
      grayscale_object_palette_0: 0,
      grayscale_object_palette_1: 0,
      background_palette_index: 0,
      background_palettes: [0; 2 * COLORS_PER_PALETTE * NUMBER_OF_PALETTES],
      object_palette_index: 0,
      object_palettes: [0; 2 * COLORS_PER_PALETTE * NUMBER_OF_PALETTES],
    }
  }
}

impl CRAM for CRAMImpl {

  fn background_color(&self, color_ref: ColorReference) -> Color {
    let lower_byte_address = ((color_ref.palette_index << 3) | (color_ref.color_index << 1)) as usize;
    let color_word = (&self.background_palettes[lower_byte_address..=lower_byte_address + 1]).read_u16::<LittleEndian>().unwrap();
    Color::from_word(color_word)
  }

  fn object_color(&self, color_ref: ColorReference) -> Color {
    if color_ref.color_index == 0 {
      Color::transparent()
    } else {
      let lower_byte_address = ((color_ref.palette_index << 3) | (color_ref.color_index << 1)) as usize;
      let color_word = (&self.object_palettes[lower_byte_address..=lower_byte_address + 1]).read_u16::<LittleEndian>().unwrap();
      Color::from_word(color_word)
    }
  }
}

impl Memory for CRAMImpl {
  fn read(&self, address: u16) -> u8 {
    match address {
      0xFF47 => self.grayscale_background_palette,
      0xFF48 => self.grayscale_object_palette_0,
      0xFF49 => self.grayscale_object_palette_1,
      0xFF68 => self.background_palette_index,
      0xFF69 => self.background_palettes[(self.background_palette_index & 0x3F) as usize],
      0xFF6A => self.object_palette_index,
      0xFF6B => self.object_palettes[(self.object_palette_index & 0x3F) as usize],
      _ => panic!("Unable to read address {:#x} from CRAM", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      0xFF47 => self.grayscale_background_palette = value,
      0xFF48 => self.grayscale_object_palette_0 = value,
      0xFF49 => self.grayscale_object_palette_1 = value,
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
        if self.object_palette_index.get_bit(7) { // Auto-increment ocps
          // By clearing bit 6 (which is unused) after increment,
          // we prevent incrementing into the higher bits and allow the index to wrap back to 0
          self.object_palette_index = (self.object_palette_index + 1).reset_bit(6);
        }
      }
      _ => panic!("Unable to write to address {:#x} in CRAM", address)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use test_case::test_case;

  //TODO add test cases for grayscale palettes

  #[test_case(0x0FF68, 0xFF69; "background color")]
  #[test_case(0x0FF68, 0xFF69; "object color")]
  fn writes_color_to_correct_location(index_address: u16, data_address: u16) {
    let mut cram = CRAMImpl::new();
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
    let mut cram = CRAMImpl::new();
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
    let mut cram = CRAMImpl::new();
    cram.write(0xFF68, 0xB4);
    cram.write(0xFF69, 0xD5);
    cram.write(0xFF69, 0x2B);
    let color = cram.background_color(ColorReference{ color_index: 6, palette_index: 2 });
    assert_eq!(color.red, 0x15); // Red
    assert_eq!(color.green, 0x1E); // Green
    assert_eq!(color.blue, 0x0A); // Blue
  }

  #[test]
  fn get_object_color_returns_correct_color() {
    let mut cram = CRAMImpl::new();
    cram.write(0xFF6A, 0xB4);
    cram.write(0xFF6B, 0xD5);
    cram.write(0xFF6B, 0x2B);
    let color = cram.object_color(ColorReference{ color_index: 6, palette_index: 2 });
    assert_eq!(color.red, 0x15); // Red
    assert_eq!(color.green, 0x1E); // Green
    assert_eq!(color.blue, 0x0A); // Blue
  }
}

