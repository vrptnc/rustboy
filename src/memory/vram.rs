use std::iter::Map;
use std::ops::Range;
use crate::memory::memory::Memory;
use crate::renderer::renderer::{ColorIndex, Point, TileAddressingMode, TileMapIndex};
use crate::util::bit_util::BitUtil;

const BANK_SIZE: usize = 0x2000;
const START_ADDRESS: u16 = 0x8000;
const END_ADDRESS: u16 = 0x9FFF;

#[derive(Copy, Clone)]
pub struct TileAttributes(u8);

impl TileAttributes {
  pub fn has_priority_over_oam(&self) -> bool {
    self.0.get_bit(7)
  }

  pub fn flip_vertical(&self) -> bool {
    self.0.get_bit(6)
  }

  pub fn flip_horizontal(&self) -> bool {
    self.0.get_bit(5)
  }

  pub fn tile_bank_number(&self) -> u8 {
    self.0.get_bit(3) as u8
  }
}

pub struct TileRow(u8, u8);

impl TileRow {
  pub fn color_index_for_pixel(&self, u8: pixel) -> ColorIndex {
    ((self.1.get_bit(7 - pixel) as u8) << 1) |
      (self.0.get_bit(7 - pixel) as u8)
  }
}

struct Tile {
  chr_code: u8,
  attributes: u8,
}

pub struct TileDataMap<'a> {
  data_block_1_bank_1: &'a [u8],
  data_block_1_bank_2: &'a [u8],
  data_block_2: &'a [u8],
}

impl<'a> TileDataMap<'a> {

}

pub struct TileMap<'a> {
  chr_codes: &'a [u8],
  attributes: &'a [u8],
}

impl<'a> TileMap<'a> {
  const TILES_PER_ROW: u8 = 32;
  const TILES_PER_COLUMN: u8 = 32;
  const TILE_WIDTH: u8 = 8;
  const TILE_HEIGHT: u8 = 8;
  const TILES_PER_SCANLINE: u8 = 20;

  pub fn background_tiles_for_line(&self, line: u8, background_origin: Point) -> impl Iterator<Item=Tile> {
    let tile_map_line = line.wrapping_add(background_origin.y);
    let tile_map_row = tile_map_line / TileMap::TILE_HEIGHT;
    let tile_map_column = background_origin.x / TileMap::TILE_WIDTH;

    (0..TileMap::TILES_PER_SCANLINE)
      .map(move |tile_offset| (tile_map_row * TileMap::TILES_PER_ROW) + ((tile_map_column + tile_offset) % TileMap::TILES_PER_ROW))
      .map(|tile_index| Tile {
        chr_code: self.chr_codes[tile_index],
        attributes: self.attributes[tile_index],
      })
  }
}

pub struct VRAM {
  bytes: [[u8; BANK_SIZE]; 2],
  bank_index: u8,
}

impl VRAM {
  pub fn new() -> VRAM {
    VRAM {
      bytes: [[0; BANK_SIZE]; 2],
      bank_index: 0,
    }
  }

  pub fn tile_map(&self, tile_map_index: u8) -> TileMap {
    let tile_map_offset: usize = match tile_map_index {
      0 => 0x1800,
      1 => 0x1C00,
      _ => panic!("Can't access tile map at index {}", tile_map_index)
    };
    TileMap {
      chr_codes: &self.bytes[0][tile_map_offset..(tile_map_offset + 0x400)],
      attributes: &self.bytes[1][tile_map_offset..(tile_map_offset + 0x400)],
    }
  }

  pub fn tile_data_map(&self, addressing_mode: TileAddressingMode) -> TileDataMap {
    match addressing_mode {
      TileAddressingMode::Mode8000 => TileDataMap {
        data_block_1: &self.bytes[..0x800],
        data_block_2: &self.bytes[0x800..0x1000],
      }
      TileAddressingMode::Mode8800 => TileDataMap {

      }
    }
  }

  pub fn get_background_line(&self, line: u8, tile_map_index: TileMapIndex, addressing_mode: TileAddressingMode) -> impl Iterator<Item=u8> + '_ {
    let tile_map_offset: usize = if tile_map_index == 0 { 0x1800 } else { 0x1C00 };
    let tile_row = (line % 8) as usize;
    (8 * tile_row..(8 * tile_row + 32))
      .map(move |tile_index| {
        let tile_chr_code = self.bytes[tile_map_offset + tile_index][0];
        let tile_attribute = TileAttributes(self.bytes[tile_map_offset + tile_index][1]);
        let tile_data_index = match addressing_mode {
          TileAddressingMode::Unsigned => tile_chr_code as usize,
          TileAddressingMode::Signed => 0x1000(tile_chr_code as i8 as u8).wrapping_add(0x1000)
        };
        tile_data_index
      })
  }
}

impl Memory for VRAM {
  fn read(&self, address: u16) -> u8 {
    match address {
      START_ADDRESS..=END_ADDRESS => {
        self.bytes[self.bank_index as usize][(address - START_ADDRESS) as usize]
      }
      0xFF4F => self.bank_index,
      _ => panic!("Can't read address {} from VRAM", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      START_ADDRESS..=END_ADDRESS => {
        self.bytes[self.bank_index as usize][(address - START_ADDRESS) as usize] = value
      }
      0xFF4F => self.bank_index = value & 0x01,
      _ => panic!("Can't write to address {} in VRAM", address)
    }
  }
}

