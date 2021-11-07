use super::memory::Memory;

pub struct BankMemory<const BankSize: usize, const BankCount: usize> {
  start_address: u16,
  bank_index: u8,
  bytes: [[u8; BankSize]; BankCount],
}

impl<const BankSize: usize, const BankCount: usize> Memory for BankMemory<BankSize, BankCount> {
  fn read(&self, address: u16) -> u8 {
    self.bytes[self.bank_index][address - self.start_address]
  }

  fn write(&mut self, address: u16, value: u8) {
    self.bytes[self.bank_index][address - self.start_address] = value;
  }
}

impl<const BankSize: usize, const BankCount: usize> BankMemory<BankSize, BankCount> {
  pub fn new(start_address: u16) -> BankMemory<BankSize, BankCount> {
    BankMemory {
      start_address,
      bank_index: 0,
      bytes: [[0; BankSize]; BankCount],
    }
  }
}
