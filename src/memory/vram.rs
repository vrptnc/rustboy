use std::cell::RefCell;
use std::iter;
use std::iter::{Map, Rev, Skip};
use std::ops::Range;
use std::rc::Rc;
use mockall::automock;
use crate::memory::memory::Memory;
use crate::renderer::renderer::{ColorIndex, Point, TileAddressingMode, TileMapIndex};
use crate::util::bit_util::{BitUtil, ByteUtil, UnsignedCrumbIterator};
use crate::util::iterator::SizedIterator;

#[derive(Copy, Clone)]
pub struct TileAttributes(u8);

impl TileAttributes {
  pub fn bg_and_window_priority_over_oam(&self) -> bool {
    self.0.get_bit(7)
  }

  pub fn flip_vertical(&self) -> bool {
    self.0.get_bit(6)
  }

  pub fn flip_horizontal(&self) -> bool {
    self.0.get_bit(5)
  }

  pub fn tile_bank_index(&self) -> u8 {
    self.0.get_bit(3) as u8
  }

  pub fn palette_index(&self) -> u8 {
    self.0 & 0x7
  }
}

#[derive(Copy, Clone)]
pub struct Tile {
  pub chr_code: u8,
  pub attributes: TileAttributes,
}

#[derive(Copy, Clone)]
pub struct TileData<'a> {
  bytes: &'a [u8],
}

impl<'a> TileData<'a> {
  pub fn get_color_indices(&self, row_offset: u8, flip_horizontal: bool, flip_vertical: bool) -> impl Iterator<Item=u8> + 'a {
    let (byte1, byte2) = match (flip_horizontal, flip_vertical) {
      (false, false) => (self.bytes[2 * row_offset as usize], self.bytes[2 * row_offset as usize + 1]),
      (false, true) => (self.bytes[14 - 2 * row_offset as usize], self.bytes[15 - 2 * row_offset as usize]),
      (true, false) => (self.bytes[2 * row_offset as usize + 1], self.bytes[2 * row_offset as usize]),
      (true, true) => (self.bytes[15 - 2 * row_offset as usize], self.bytes[14 - 2 * row_offset as usize]),
    };
    byte1.interleave_with(byte2).crumbs().rev()
  }
}

pub struct TileDataView<'a> {
  block_1: [&'a [u8]; 2],
  block_2: [&'a [u8]; 2],
}

impl<'a> TileDataView<'a> {
  pub fn get_tile_data(&self, tile_bank_index: u8, tile_index: u8) -> TileData {
    match tile_index {
      0..=127 => {
        let byte_offset = 16 * tile_index as usize;
        TileData {
          bytes: &self.block_1[tile_bank_index as usize][byte_offset..byte_offset + 16]
        }
      }
      128..=255 => {
        let byte_offset = 16 * (tile_index - 128) as usize;
        TileData {
          bytes: &self.block_2[tile_bank_index as usize][byte_offset..byte_offset + 16]
        }
      }
      _ => panic!("Can't access tile data for tile index {}", tile_index)
    }
  }
}

pub struct TileMapView<'a> {
  bytes: [&'a [u8]; 2],
}

impl<'a> TileMapView<'a> {
  const TILES_PER_ROW: u8 = 32;
  const TILES_PER_COLUMN: u8 = 32;
  const TILE_WIDTH: u8 = 8;
  const TILE_HEIGHT: u8 = 8;
  const TILES_PER_SCANLINE: u8 = 20;
  const FRAME_ROWS: u8 = 144;
  const FRAME_COLUMNS: u8 = 160;

  pub fn row(&'a self, row: u8) -> impl Iterator<Item=Tile> + Clone + 'a {
    let tile_offset = (row * TileMapView::TILES_PER_ROW) as usize;

    (0..TileMapView::TILES_PER_ROW)
      .map(move |tile_index| Tile {
        chr_code: self.bytes[0][tile_offset + tile_index as usize],
        attributes: TileAttributes(self.bytes[1][tile_offset + tile_index as usize]),
      })
  }
}

#[automock]
pub trait VRAM {
  fn tile_map<'a>(&'a self, tile_map_index: TileMapIndex) -> TileMapView<'a>;
  fn tile_data<'a>(&'a self, addressing_mode: TileAddressingMode) -> TileDataView<'a>;
}

pub struct VRAMImpl {
  bank_index: u8,
  bytes: [[u8; VRAMImpl::BANK_SIZE]; 2],
}

impl VRAMImpl {
  const START_ADDRESS: u16 = 0x8000;
  const END_ADDRESS: u16 = 0x9FFF;
  const BANK_INDEX_ADDRESS: u16 = 0xFF4F;
  const BANK_SIZE: usize = 0x2000;

  pub fn new() -> VRAMImpl {
    VRAMImpl {
      bank_index: 0,
      bytes: [[0; VRAMImpl::BANK_SIZE]; 2],
    }
  }
}

impl VRAM for VRAMImpl {
  fn tile_map(&self, tile_map_index: TileMapIndex) -> TileMapView {
    match tile_map_index {
      TileMapIndex::TileMap1 => TileMapView {
        bytes: [&self.bytes[0][0x1800..0x1C00], &self.bytes[1][0x1800..0x1C00]]
      },
      TileMapIndex::TileMap2 => TileMapView {
        bytes: [&self.bytes[0][0x1C00..0x2000], &self.bytes[1][0x1C00..0x2000]]
      }
    }
  }

  fn tile_data(&self, addressing_mode: TileAddressingMode) -> TileDataView {
    match addressing_mode {
      TileAddressingMode::Mode8000 => TileDataView {
        block_1: [&self.bytes[0][0..0x800], &self.bytes[1][0..0x800]],
        block_2: [&self.bytes[0][0x800..0x1000], &self.bytes[1][0x800..0x1000]],
      },
      TileAddressingMode::Mode8800 => TileDataView {
        block_1: [&self.bytes[0][0x1000..0x1800], &self.bytes[1][0x1000..0x1800]],
        block_2: [&self.bytes[0][0x800..0x1000], &self.bytes[1][0x800..0x1000]],
      }
    }
  }
}

impl Memory for VRAMImpl {
  fn read(&self, address: u16) -> u8 {
    match address {
      VRAMImpl::START_ADDRESS..=VRAMImpl::END_ADDRESS => {
        self.bytes[self.bank_index as usize][(address - VRAMImpl::START_ADDRESS) as usize]
      }
      VRAMImpl::BANK_INDEX_ADDRESS => self.bank_index,
      _ => panic!("Can't read address {} from VRAM", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      VRAMImpl::START_ADDRESS..=VRAMImpl::END_ADDRESS => {
        self.bytes[self.bank_index as usize][(address - VRAMImpl::START_ADDRESS) as usize] = value
      }
      VRAMImpl::BANK_INDEX_ADDRESS => self.bank_index = value & 0x01,
      _ => panic!("Can't write to address {} in VRAM", address)
    }
  }
}

#[cfg(test)]
pub mod tests {
  use assert_hex::assert_eq_hex;
  use super::*;

  #[test]
  fn set_vram_bank() {
    let mut vram = VRAMImpl::new();
    vram.write(VRAMImpl::BANK_INDEX_ADDRESS, 0);
    vram.write(VRAMImpl::START_ADDRESS, 0xAB);
    vram.write(VRAMImpl::BANK_INDEX_ADDRESS, 1);
    vram.write(VRAMImpl::START_ADDRESS, 0xCD);
    assert_eq_hex!(vram.read(VRAMImpl::START_ADDRESS), 0xCD);
    vram.write(VRAMImpl::BANK_INDEX_ADDRESS, 0);
    assert_eq_hex!(vram.read(VRAMImpl::START_ADDRESS), 0xAB);
  }
  //
  // #[test]
  // fn get_tile_data_view() {
  //   let mut vram = VRAMImpl::new();
  // }
}

