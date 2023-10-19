use core::fmt::Formatter;

use serde::de::{Error, SeqAccess, Visitor};

pub struct ByteSliceVisitor<'a>(&'a mut [u8]);

impl<'a> ByteSliceVisitor<'a> {
  pub fn new(bytes: &'a mut [u8]) -> Self {
    ByteSliceVisitor(bytes)
  }
}

impl<'a, 'de> Visitor<'de> for ByteSliceVisitor<'a> {
  type Value = ();

  fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
    write!(formatter, "a slice of bytes of at most length {}", self.0.len())
  }

  fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> where E: Error {
    self.0.copy_from_slice(v);
    Ok(())
  }
}

pub struct ByteVisitor<'a>(&'a mut u8);

impl<'a> ByteVisitor<'a> {
  pub fn new(byte: &'a mut u8) -> Self {
    ByteVisitor(byte)
  }
}

impl<'a, 'de> Visitor<'de> for ByteVisitor<'a> {
  type Value = ();

  fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
    write!(formatter, "a single byte")
  }

  fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E> where E: Error {
    *self.0 = v;
    Ok(())
  }

  fn visit_bytes<E>(mut self, v: &[u8]) -> Result<Self::Value, E> where E: Error {
    *self.0 = v[0];
    Ok(())
  }
}

