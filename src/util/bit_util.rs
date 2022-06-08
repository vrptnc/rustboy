pub trait BitUtil {
  fn compose(bits: &[(bool, u8)]) -> Self;
  fn get_bit(&self, bit: u8) -> bool;
  fn set_bit(&self, bit: u8) -> Self;
  fn reset_bit(&self, bit: u8) -> Self;
  fn get_lower_byte(&self) -> u8;
  fn get_upper_byte(&self) -> u8;
}

impl BitUtil for u8 {
  fn compose(bits: &[(bool, u8)]) -> Self {
    bits.iter().map(|a| {
      (a.0 as u8) << a.1
    }).reduce(|a, b| {
      a | b
    }).unwrap()
  }

  fn get_bit(&self, bit: u8) -> bool {
    (self & (1u8 << bit)) != 0
  }

  fn set_bit(&self, bit: u8) -> Self {
    self | (1u8 << bit)
  }

  fn reset_bit(&self, bit: u8) -> Self {
    self & !(1u8 << bit)
  }

  fn get_lower_byte(&self) -> u8 {
    *self
  }

  fn get_upper_byte(&self) -> u8 {
    0
  }
}

impl BitUtil for u16 {
  fn compose(bits: &[(bool, u8)]) -> Self {
    bits.iter().map(|a| {
      (a.0 as u16) << a.1
    }).reduce(|a, b| {
      a | b
    }).unwrap()
  }

  fn get_bit(&self, bit: u8) -> bool {
    (self & (1u16 << bit)) != 0
  }

  fn set_bit(&self, bit: u8) -> Self {
    self | (1u16 << bit)
  }

  fn reset_bit(&self, bit: u8) -> Self {
    self & !(1u16 << bit)
  }

  fn get_lower_byte(&self) -> u8 {
    *self as u8
  }

  fn get_upper_byte(&self) -> u8 {
    (*self >> 8) as u8
  }
}

impl BitUtil for usize {
  fn compose(bits: &[(bool, u8)]) -> Self {
    bits.iter().map(|a| {
      (a.0 as usize) << a.1
    }).reduce(|a, b| {
      a | b
    }).unwrap()
  }

  fn get_bit(&self, bit: u8) -> bool {
    (self & ((1 as usize) << bit)) != 0
  }

  fn set_bit(&self, bit: u8) -> Self {
    self | ((1 as usize) << bit)
  }

  fn reset_bit(&self, bit: u8) -> Self {
    self & !((1 as usize) << bit)
  }

  fn get_lower_byte(&self) -> u8 {
    *self as u8
  }

  fn get_upper_byte(&self) -> u8 {
    (*self >> 8) as u8
  }
}