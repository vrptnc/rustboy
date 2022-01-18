use super::memory::Memory;

pub struct BankMemory<const BankSize: usize, const BankCount: usize> {
  bank_index: usize,
  bytes: [[u8; BankSize]; BankCount],
}

impl<const BankSize: usize, const BankCount: usize> Memory for BankMemory<BankSize, BankCount> {
  fn read(&self, address: usize) -> u8 {
    self.bytes[self.bank_index][address]
  }

  fn write(&mut self, address: usize, value: u8) {
    self.bytes[self.bank_index][address] = value;
  }
}

impl<const BankSize: usize, const BankCount: usize> BankMemory<BankSize, BankCount> {
  pub fn new() -> BankMemory<BankSize, BankCount> {
    BankMemory {
      bank_index: 0,
      bytes: [[0; BankSize]; BankCount],
    }
  }

  pub fn set_bank_index(&mut self, index: usize) {
    self.bank_index = index;
  }
}
