use core::fmt::Formatter;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{SeqAccess, Visitor};
use serde::ser::{SerializeSeq, SerializeStruct};
use super::memory::Memory;

// #[derive(Serialize, Deserialize)]
pub struct LinearMemory<const Size: usize, const StartAddress: u16> {
  bytes: [u8; Size],
}

impl<const Size: usize, const StartAddress: u16> Serialize for LinearMemory<Size, StartAddress> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    let mut seq = serializer.serialize_seq(Some(Size))?;
    for byte in &self.bytes[..] {
      seq.serialize_element(byte)?;
    }
    seq.end()
  }
}

impl<'de, const Size: usize, const StartAddress: u16> Deserialize<'de> for LinearMemory<Size, StartAddress> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
    struct LinearMemoryVisitor<const Size: usize, const StartAddress: u16> {
    }

    impl<'de, const Size: usize, const StartAddress: u16> Visitor<'de> for LinearMemoryVisitor<Size, StartAddress> {
      type Value = LinearMemory<Size, StartAddress>;

      fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
        write!(formatter, "a sequence of bytes of length {}", Size)
      }

      fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
        let mut memory = LinearMemory::new();
        for i in 0..Size {
          memory.bytes[i] = seq.next_element()?.unwrap()
        }
        Ok(memory)
      }
    }

    deserializer.deserialize_seq(LinearMemoryVisitor {})
  }
}

impl<const Size: usize, const StartAddress: u16> Memory for LinearMemory<Size, StartAddress> {
  fn read(&self, address: u16) -> u8 {
    self.bytes[address as usize - StartAddress as usize]
  }

  fn write(&mut self, address: u16, value: u8) {
    self.bytes[address as usize - StartAddress as usize] = value
  }
}

impl<const Size: usize, const StartAddress: u16> LinearMemory<Size, StartAddress> {
  pub fn new() -> LinearMemory<Size, StartAddress> {
    LinearMemory {
      bytes: [0; Size],
    }
  }
}