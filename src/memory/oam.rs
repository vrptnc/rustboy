use std::cell::RefCell;
use std::rc::Rc;
use crate::memory::memory::Memory;
use crate::util::bit_util::BitUtil;

const START_ADDRESS: usize = 0xFE00;

#[derive(Copy, Clone)]
pub struct ObjectAttributes(u8);

impl ObjectAttributes {
  pub fn has_priority_over_oam(&self) -> bool {
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
pub struct OAMObject {
  lcd_y: u8,
  lcd_x: u8,
  tile_index: u8,
  attribute: u8,
}

impl OAMObject {
  fn new() -> OAMObject {
    OAMObject {
      lcd_y: 0,
      lcd_x: 0,
      tile_index: 0,
      attribute: 0,
    }
  }
}

pub type OAMRef = Rc<RefCell<OAM>>;

pub struct OAM {
  bytes: [u8; 160],
}

impl OAM {
  pub fn new() -> OAM {
    OAM {
      bytes: [0; 160]
    }
  }

  pub fn object_intersects_with_line(&self, object_index: u8, line: u8, use_8_x_16_tiles: bool) -> bool {
    let object_lcd_y = self.bytes[4 * object_index as usize];
    object_lcd_y <= line + 16 && object_lcd_y > (line + if use_8_x_16_tiles { 0 } else { 8 })
  }

  pub fn get_object(&self, object_index: u8) -> OAMObject {
    let byte_offset = 4 * object_index as usize;
    let object_bytes = &self.bytes[byte_offset..(byte_offset + 4)];
    OAMObject {
      lcd_y: object_bytes[0],
      lcd_x: object_bytes[1],
      tile_index: object_bytes[2],
      attribute: object_bytes[3],
    }
  }
}

impl Memory for OAM {
  fn read(&self, address: u16) -> u8 {
    self.bytes[address as usize - START_ADDRESS]
  }

  fn write(&mut self, address: u16, value: u8) {
    self.bytes[address as usize - START_ADDRESS] = value;
  }
}