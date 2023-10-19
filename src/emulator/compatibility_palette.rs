use std::cell::RefCell;
use std::rc::Rc;
use crate::emulator::emulator::Emulator;
use crate::memory::mbc::MBC;
use crate::renderer::renderer::Color;
use crate::util::bit_util::BitUtil;

#[derive(Copy, Clone)]
pub struct CompatibilityPalettes {
  pub bgp: [Color; 4],
  pub obj0: [Color; 4],
  pub obj1: [Color; 4],
}

pub struct CompatibilityPaletteLoader {}

impl CompatibilityPaletteLoader {
  const TITLE_CHECKSUMS: [u8; 79] = [
    0x00, 0x88, 0x16, 0x36, 0xD1, 0xDB, 0xF2, 0x3C, 0x8C, 0x92, 0x3D, 0x5C, 0x58, 0xC9, 0x3E, 0x70,
    0x1D, 0x59, 0x69, 0x19, 0x35, 0xA8, 0x14, 0xAA, 0x75, 0x95, 0x99, 0x34, 0x6F, 0x15, 0xFF, 0x97,
    0x4B, 0x90, 0x17, 0x10, 0x39, 0xF7, 0xF6, 0xA2, 0x49, 0x4E, 0x43, 0x68, 0xE0, 0x8B, 0xF0, 0xCE,
    0x0C, 0x29, 0xE8, 0xB7, 0x86, 0x9A, 0x52, 0x01, 0x9D, 0x71, 0x9C, 0xBD, 0x5D, 0x6D, 0x67, 0x3F,
    0x6B, 0xB3, 0x46, 0x28, 0xA5, 0xC6, 0xD3, 0x27, 0x61, 0x18, 0x66, 0x6A, 0xBF, 0x0D, 0xF4
  ];
  const PALETTE_INDEX_INDEXES_AND_FLAGS: [u8; 94] = [
    0x7C, 0x08, 0x12, 0xA3, 0xA2, 0x07, 0x87, 0x4B, 0x20, 0x12, 0x65, 0xA8, 0x16, 0xA9, 0x86, 0xB1,
    0x68, 0xA0, 0x87, 0x66, 0x12, 0xA1, 0x30, 0x3C, 0x12, 0x85, 0x12, 0x64, 0x1B, 0x07, 0x06, 0x6F,
    0x6E, 0x6E, 0xAE, 0xAF, 0x6F, 0xB2, 0xAF, 0xB2, 0xA8, 0xAB, 0x6F, 0xAF, 0x86, 0xAE, 0xA2, 0xA2,
    0x12, 0xAF, 0x13, 0x12, 0xA1, 0x6E, 0xAF, 0xAF, 0xAD, 0x06, 0x4C, 0x6E, 0xAF, 0xAF, 0x12, 0x7C,
    0xAC, 0xA8, 0x6A, 0x6E, 0x13, 0xA0, 0x2D, 0xA8, 0x2B, 0xAC, 0x64, 0xAC, 0x6D, 0x87, 0xBC, 0x60,
    0xB4, 0x13, 0x72, 0x7C, 0xB5, 0xAE, 0xAE, 0x7C, 0x7C, 0x65, 0xA2, 0x6C, 0x64, 0x85
  ];
  const PALETTE_INDEXES: [u8; 87] = [
    0x80, 0xB0, 0x40,
    0x88, 0x20, 0x68,
    0xDE, 0x00, 0x70,
    0xDE, 0x20, 0x78,
    0x20, 0x20, 0x38,
    0x20, 0xB0, 0x90,
    0x20, 0xB0, 0xA0,
    0xE0, 0xB0, 0xC0,
    0x98, 0xB6, 0x48,
    0x80, 0xE0, 0x50,
    0x1E, 0x1E, 0x58,
    0x20, 0xB8, 0xE0,
    0x88, 0xB0, 0x10,
    0x20, 0x00, 0x10,
    0x20, 0xE0, 0x18,
    0xE0, 0x18, 0x00,
    0x18, 0xE0, 0x20,
    0xA8, 0xE0, 0x20,
    0x18, 0xE0, 0x00,
    0x20, 0x18, 0xD8,
    0xC8, 0x18, 0xE0,
    0x00, 0xE0, 0x40,
    0x28, 0x28, 0x28,
    0x18, 0xE0, 0x60,
    0x20, 0x18, 0xE0,
    0x00, 0x00, 0x08,
    0xE0, 0x18, 0x30,
    0xD0, 0xD0, 0xD0,
    0x20, 0xE0, 0xE8,
  ];
  const PALETTES: [Color; 120] = [
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x1F, 0x15, 0x0C),
    Color::from_rgb(0x10, 0x06, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1C, 0x18),
    Color::from_rgb(0x19, 0x13, 0x10),
    Color::from_rgb(0x10, 0x0D, 0x05),
    Color::from_rgb(0x0B, 0x06, 0x01),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x11, 0x11, 0x1B),
    Color::from_rgb(0x0A, 0x0A, 0x11),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x0F, 0x1F, 0x06),
    Color::from_rgb(0x00, 0x10, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x1F, 0x10, 0x10),
    Color::from_rgb(0x12, 0x07, 0x07),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x14, 0x14, 0x14),
    Color::from_rgb(0x0A, 0x0A, 0x0A),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x1F, 0x1F, 0x00),
    Color::from_rgb(0x0F, 0x09, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x0F, 0x1F, 0x00),
    Color::from_rgb(0x16, 0x0E, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x15, 0x15, 0x10),
    Color::from_rgb(0x08, 0x0E, 0x0F),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x14, 0x13, 0x1F),
    Color::from_rgb(0x1F, 0x1F, 0x00),
    Color::from_rgb(0x00, 0x0C, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x19),
    Color::from_rgb(0x0C, 0x1D, 0x1D),
    Color::from_rgb(0x13, 0x10, 0x06),
    Color::from_rgb(0x0B, 0x0B, 0x0B),
    Color::from_rgb(0x16, 0x16, 0x1F),
    Color::from_rgb(0x1F, 0x1F, 0x12),
    Color::from_rgb(0x15, 0x0B, 0x08),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x14),
    Color::from_rgb(0x1F, 0x12, 0x12),
    Color::from_rgb(0x12, 0x12, 0x1F),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x13),
    Color::from_rgb(0x12, 0x16, 0x1F),
    Color::from_rgb(0x0C, 0x12, 0x0E),
    Color::from_rgb(0x00, 0x07, 0x07),
    Color::from_rgb(0x0D, 0x1F, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x1F, 0x0A, 0x09),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x0A, 0x1B, 0x00),
    Color::from_rgb(0x1F, 0x10, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x1F, 0x0E, 0x00),
    Color::from_rgb(0x12, 0x08, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x18, 0x08),
    Color::from_rgb(0x1F, 0x1A, 0x00),
    Color::from_rgb(0x12, 0x07, 0x00),
    Color::from_rgb(0x09, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x0A, 0x1F, 0x00),
    Color::from_rgb(0x1F, 0x08, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x0C, 0x0A),
    Color::from_rgb(0x1A, 0x00, 0x00),
    Color::from_rgb(0x0C, 0x00, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x1F, 0x13, 0x00),
    Color::from_rgb(0x1F, 0x00, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x00, 0x1F, 0x00),
    Color::from_rgb(0x06, 0x10, 0x00),
    Color::from_rgb(0x00, 0x09, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x0B, 0x17, 0x1F),
    Color::from_rgb(0x1F, 0x00, 0x00),
    Color::from_rgb(0x00, 0x00, 0x1F),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x1F, 0x1F, 0x0F),
    Color::from_rgb(0x00, 0x10, 0x1F),
    Color::from_rgb(0x1F, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x1F, 0x1F, 0x00),
    Color::from_rgb(0x1F, 0x00, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x00),
    Color::from_rgb(0x1F, 0x00, 0x00),
    Color::from_rgb(0x0C, 0x00, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x1F, 0x19, 0x00),
    Color::from_rgb(0x13, 0x0C, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x00, 0x10, 0x10),
    Color::from_rgb(0x1F, 0x1B, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x0C, 0x14, 0x1F),
    Color::from_rgb(0x00, 0x00, 0x1F),
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0x1F, 0x1F, 0x1F),
    Color::from_rgb(0x0F, 0x1F, 0x06),
    Color::from_rgb(0x00, 0x0C, 0x18),
    Color::from_rgb(0x00, 0x00, 0x00),
  ];

  const TITLE_FOURTH_LETTERS: [char; 29] = [
    'B', 'E', 'F', 'A', 'A', 'R', 'B', 'E', 'K', 'E', 'K', ' ', 'R', '-',
    'U', 'R', 'A', 'R', ' ', 'I', 'N', 'A', 'I', 'L', 'I', 'C', 'E', ' ',
    'R'
  ];

  pub fn get_compatibility_palettes(rom: Rc<RefCell<dyn MBC>>) -> CompatibilityPalettes {
    let borrowed_rom = (*rom).borrow();
    let palette_id = if borrowed_rom.is_licensed_by_nintendo() {
      let title_checksum = borrowed_rom.title_checksum();
      if let Some(checksum_index) = CompatibilityPaletteLoader::TITLE_CHECKSUMS.into_iter().position(|value| value == title_checksum) {
        if checksum_index <= 64 {
          checksum_index
        } else {
          // Do a 4th letter check
          let fourth_letter = borrowed_rom.fourth_title_letter();
          let offset_checksum_index = checksum_index - 65;
          if let Some(row) = CompatibilityPaletteLoader::TITLE_FOURTH_LETTERS.into_iter()
            .skip(offset_checksum_index)
            .step_by(14)
            .position(|letter| letter as u8 == fourth_letter) {
            checksum_index + (14 * row)
          } else {
            0x00
          }
        }
      } else {
        0x00
      }
    } else {
      0x00
    };
    let palette_index_index_and_flags = CompatibilityPaletteLoader::PALETTE_INDEX_INDEXES_AND_FLAGS[palette_id as usize];
    let palette_index_index = (palette_index_index_and_flags & 0x1F) as usize;
    let shuffle_flags = (palette_index_index_and_flags & 0xE0) >> 5;
    let palette_index_offset = palette_index_index * 3;
    let palette_indexes = &CompatibilityPaletteLoader::PALETTE_INDEXES[palette_index_offset..palette_index_offset + 3];
    // Divide index by number of bytes per Color
    let bgp_index = (palette_indexes[2] as usize) / 2;
    let obj0_index = ((if shuffle_flags.get_bit(0) { palette_indexes[0] } else { palette_indexes[2] }) as usize) / 2;
    let obj1_index = ((if shuffle_flags.get_bit(2) { palette_indexes[1] } else if shuffle_flags.get_bit(1) { palette_indexes[0] } else { palette_indexes[2] }) as usize) / 2;
    return CompatibilityPalettes {
      bgp: (&CompatibilityPaletteLoader::PALETTES[bgp_index..(bgp_index + 4)]).try_into().unwrap(),
      obj0: (&CompatibilityPaletteLoader::PALETTES[obj0_index..(obj0_index + 4)]).try_into().unwrap(),
      obj1: (&CompatibilityPaletteLoader::PALETTES[obj1_index..(obj1_index + 4)]).try_into().unwrap(),
    };
  }
}

#[cfg(test)]
mod tests {
  use crate::memory::mbc1::MBC1;
  use crate::memory::mbc::MockROM;
  use crate::memory::memory::{RAMSize, ROMSize};
  use super::*;

  #[test]
  fn get_pokemon_red_compatibility_palette() {
    let mut rom = MockROM::new();
    rom.expect_is_licensed_by_nintendo().once().return_const(true);
    rom.expect_title_checksum().once().return_const(0x14);
    rom.expect_fourth_title_letter().never();
    let boxed_rom = Rc::new(RefCell::new(rom));
    let result = CompatibilityPaletteLoader::get_compatibility_palettes(boxed_rom);
    assert_eq!(result.bgp[0], Color::from_rgb(0xFF, 0xFF, 0xFF).to_rgb555());
    assert_eq!(result.bgp[1], Color::from_rgb(0xFF, 0x84, 0x84).to_rgb555());
    assert_eq!(result.bgp[2], Color::from_rgb(0x94, 0x3A, 0x3A).to_rgb555());
    assert_eq!(result.bgp[3], Color::from_rgb(0x00, 0x00, 0x00).to_rgb555());
    assert_eq!(result.obj0[0], Color::from_rgb(0xFF, 0xFF, 0xFF).to_rgb555());
    assert_eq!(result.obj0[1], Color::from_rgb(0x7B, 0xFF, 0x31).to_rgb555());
    assert_eq!(result.obj0[2], Color::from_rgb(0x00, 0x84, 0x00).to_rgb555());
    assert_eq!(result.obj0[3], Color::from_rgb(0x00, 0x00, 0x00).to_rgb555());
    assert_eq!(result.obj1[0], Color::from_rgb(0xFF, 0xFF, 0xFF).to_rgb555());
    assert_eq!(result.obj1[1], Color::from_rgb(0xFF, 0x84, 0x84).to_rgb555());
    assert_eq!(result.obj1[2], Color::from_rgb(0x94, 0x3A, 0x3A).to_rgb555());
    assert_eq!(result.obj1[3], Color::from_rgb(0x00, 0x00, 0x00).to_rgb555());
  }

  #[test]
  fn get_loz_links_awakening_compatibility_palette() {
    let mut rom = MockROM::new();
    rom.expect_is_licensed_by_nintendo().once().return_const(true);
    rom.expect_title_checksum().once().return_const(0x70);
    rom.expect_fourth_title_letter().never();
    let boxed_rom = Rc::new(RefCell::new(rom));
    let result = CompatibilityPaletteLoader::get_compatibility_palettes(boxed_rom);
    assert_eq!(result.bgp[0], Color::from_rgb(0xFF, 0xFF, 0xFF).to_rgb555());
    assert_eq!(result.bgp[1], Color::from_rgb(0xFF, 0x84, 0x84).to_rgb555());
    assert_eq!(result.bgp[2], Color::from_rgb(0x94, 0x3A, 0x3A).to_rgb555());
    assert_eq!(result.bgp[3], Color::from_rgb(0x00, 0x00, 0x00).to_rgb555());
    assert_eq!(result.obj0[0], Color::from_rgb(0xFF, 0xFF, 0xFF).to_rgb555());
    assert_eq!(result.obj0[1], Color::from_rgb(0x00, 0xFF, 0x00).to_rgb555());
    assert_eq!(result.obj0[2], Color::from_rgb(0x31, 0x84, 0x00).to_rgb555());
    assert_eq!(result.obj0[3], Color::from_rgb(0x00, 0x4A, 0x00).to_rgb555());
    assert_eq!(result.obj1[0], Color::from_rgb(0xFF, 0xFF, 0xFF).to_rgb555());
    assert_eq!(result.obj1[1], Color::from_rgb(0x63, 0xA5, 0xFF).to_rgb555());
    assert_eq!(result.obj1[2], Color::from_rgb(0x00, 0x00, 0xFF).to_rgb555());
    assert_eq!(result.obj1[3], Color::from_rgb(0x00, 0x00, 0x00).to_rgb555());
  }

  #[test]
  fn get_kirby_dream_land_compatibility_palette() {
    let mut rom = MockROM::new();
    rom.expect_is_licensed_by_nintendo().once().return_const(true);
    rom.expect_title_checksum().once().return_const(0xB3);
    rom.expect_fourth_title_letter().once().return_const(0x42); // 'B'
    let boxed_rom = Rc::new(RefCell::new(rom));
    let result = CompatibilityPaletteLoader::get_compatibility_palettes(boxed_rom);
    assert_eq!(result.bgp[0], Color::from_rgb(0xA5, 0x9C, 0xFF).to_rgb555());
    assert_eq!(result.bgp[1], Color::from_rgb(0xFF, 0xFF, 0x00).to_rgb555());
    assert_eq!(result.bgp[2], Color::from_rgb(0x00, 0x63, 0x00).to_rgb555());
    assert_eq!(result.bgp[3], Color::from_rgb(0x00, 0x00, 0x00).to_rgb555());
  }
}