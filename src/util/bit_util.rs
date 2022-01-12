pub trait BitUtil {
  fn compose(bits: &[(bool, u32)]) -> Self;
  fn get_bit(&self, bit: u32) -> bool;
}

impl BitUtil for u8 {
  fn compose(bits: &[(bool, u32)]) -> Self {
    bits.iter().map(|a| {
      (if a.0 { 1u8 } else { 0u8 }).wrapping_shl(a.1)
    }).reduce(|a, b| {
      a | b
    }).unwrap()
  }

  fn get_bit(&self, bit: u32) -> bool {
    self.wrapping_shr(bit) & 0x01 == 0x01
  }
}

impl BitUtil for u16 {
  fn compose(bits: &[(bool, u32)]) -> Self {
    bits.iter().map(|a| {
      (if a.0 { 1u16 } else { 0u16 }).wrapping_shl(a.1)
    }).reduce(|a, b| {
      a | b
    }).unwrap()
  }

  fn get_bit(&self, bit: u32) -> bool {
    self.wrapping_shr(bit) & 0x0001 == 0x0001
  }
}