use crate::CPURef;
use crate::features::lcd::LCDRef;
use crate::memory::memory::{Memory, MemoryRef};
use crate::time::time::ClockAware;
use crate::util::bit_util::BitUtil;

#[derive(PartialEq)]
enum DMATransfer {
  Legacy {
    source_address: u16,
    bytes_transferred: u8,
  },
  GeneralPurpose {
    source_address: u16,
    destination_address: u16,
    bytes_transferred: u8,
    bytes_to_transfer: u8,
    in_progress: bool,
  },
  HBlank {
    source_address: u16,
    destination_address: u16,
    bytes_transferred: u8,
    bytes_to_transfer: u8,
    in_progress: bool,
  },
}

struct DMA {
  memory: Option<MemoryRef>,
  cpu: Option<CPURef>,
  dma: u8,
  high_source_address: u8,
  low_source_address: u8,
  high_destination_address: u8,
  low_destination_address: u8,
  hdma5: u8,
  active_transfer: Option<DMATransfer>,
  double_speed_toggle: bool,
}

impl DMA {
  pub fn new() -> DMA {
    DMA {
      memory: None,
      cpu: None,
      dma: 0,
      high_source_address: 0,
      low_source_address: 0,
      high_destination_address: 0,
      low_destination_address: 0,
      hdma5: 0xFF,
      active_transfer: None,
      double_speed_toggle: true,
    }
  }

  pub fn set_memory(&mut self, memory: MemoryRef) {
    self.memory = Some(memory);
  }

  pub fn set_cpu(&mut self, cpu: CPURef) {
    self.cpu = Some(cpu);
  }
}

impl ClockAware for DMA {
  fn handle_tick(&mut self, double_speed: bool) {
    if let Some(ref mut active_transfer) = self.active_transfer {
      let memory = self.memory.as_ref().unwrap();
      match *active_transfer {
        DMATransfer::Legacy {
          source_address,
          ref mut bytes_transferred
        } => {
          let current_byte = memory.borrow().read(source_address + (*bytes_transferred as u16));
          memory.borrow_mut().write(0xFE00 + (*bytes_transferred as u16), current_byte);
          *bytes_transferred += 1;
          if *bytes_transferred == 160 {
            self.active_transfer = None
          }
        }
        DMATransfer::GeneralPurpose {
          source_address,
          destination_address,
          ref mut bytes_transferred,
          bytes_to_transfer,
          ref mut in_progress
        } => {
          if double_speed {
            self.double_speed_toggle = !self.double_speed_toggle;
            if self.double_speed_toggle {
              return;
            }
          }
          let cpu = self.cpu.as_ref().unwrap();
          if !*in_progress {
            *in_progress = true;
            cpu.borrow_mut().disable();
          }
          let current_byte = memory.borrow().read(source_address + (*bytes_transferred as u16));
          memory.borrow_mut().write(destination_address + (*bytes_transferred as u16), current_byte);
          *bytes_transferred += 1;
          if *bytes_transferred == bytes_to_transfer {
            self.active_transfer = None;
            self.hdma5 = 0xFF;
            cpu.borrow_mut().enable();
          }
        }
        DMATransfer::HBlank {
          source_address,
          destination_address,
          ref mut bytes_transferred,
          bytes_to_transfer,
          ref mut in_progress
        } => {
          let in_hblank = (self.memory.as_ref().unwrap().borrow().read(0xFF41) & 0x03) == 0;
          let cpu = self.cpu.as_ref().unwrap();
          if in_hblank {
            if double_speed {
              self.double_speed_toggle = !self.double_speed_toggle;
              if self.double_speed_toggle {
                return;
              }
            }
            if !*in_progress {
              *in_progress = true;
              cpu.borrow_mut().disable();
            }
            let current_byte = memory.borrow().read(source_address + (*bytes_transferred as u16));
            memory.borrow_mut().write(destination_address + (*bytes_transferred as u16), current_byte);
            *bytes_transferred += 1;
            if *bytes_transferred == bytes_to_transfer {
              cpu.borrow_mut().enable();
              self.active_transfer = None;
              self.hdma5 = 0xFF;
            } else {
              let lines_to_transfer = (bytes_to_transfer / 16);
              let lines_transferred = (*bytes_transferred / 16);
              let lines_remaining = lines_to_transfer - lines_transferred;
              self.hdma5 = lines_remaining - 1;
            }
          } else if *in_progress {
            *in_progress = false;
            cpu.borrow_mut().enable();
          }
        }
      }
    }
  }
}

impl Memory for DMA {
  fn read(&self, address: u16) -> u8 {
    match address {
      0xFF46 => self.dma,
      0xFF55 => self.hdma5,
      _ => panic!("DMA can't read from address {}", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      0xFF46 => {
        self.dma = value;
        self.active_transfer = Some(DMATransfer::Legacy {
          source_address: (value as u16) * 0x100,
          bytes_transferred: 0,
        });
      }
      0xFF51 => self.high_source_address = value,
      0xFF52 => self.low_source_address = value & 0xF0,
      0xFF53 => self.high_destination_address = (value & 0x1F) | (0x80),
      0xFF54 => self.low_destination_address = value & 0xF0,
      0xFF55 => {
        match self.active_transfer {
          None => {
            if value.get_bit(7) {
              self.active_transfer = Some(DMATransfer::HBlank {
                source_address: ((self.high_source_address as u16) << 8) | (self.low_source_address as u16),
                destination_address: ((self.high_destination_address as u16) << 8) | (self.low_destination_address as u16),
                bytes_transferred: 0,
                bytes_to_transfer: ((value & 0x7F) + 1) * 16,
                in_progress: false,
              })
            } else {
              self.active_transfer = Some(DMATransfer::GeneralPurpose {
                source_address: ((self.high_source_address as u16) << 8) | (self.low_source_address as u16),
                destination_address: ((self.high_destination_address as u16) << 8) | (self.low_destination_address as u16),
                bytes_transferred: 0,
                bytes_to_transfer: ((value & 0x7F) + 1) * 16,
                in_progress: false,
              })
            }
            self.hdma5 = 0x00;
          }
          Some(ref mut active_transfer) => {
            //If an active HBlank transfer is ongoing, and bit 7 is set to 0, cancel the transfer
            if let DMATransfer::HBlank { .. } = active_transfer {
              if !value.get_bit(7) {
                self.cpu.as_ref().unwrap().borrow_mut().enable();
                self.active_transfer = None;
                self.hdma5 = self.hdma5.set_bit(7);
              }
            }
          }
        }
      }
      _ => panic!("DMA can't write to address {}", address)
    }
  }
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;
  use std::rc::Rc;
  use assert_hex::assert_eq_hex;
  use crate::CPU;
  use crate::cpu::interrupts::InterruptController;
  use crate::memory::memory::test::MockMemory;
  use super::*;

  fn create_memory() -> MockMemory {
    let mut memory = MockMemory::new(0x10000);
    for address in 0xC000u16..0xC100u16 {
      memory.write(address, address as u8);
    }
    memory
  }

  fn create_dma() -> DMA {
    let mut dma = DMA::new();
    let memory: MemoryRef = Rc::new(RefCell::new(Box::new(create_memory())));
    let interrupt_controller = Rc::new(RefCell::new(InterruptController::new()));
    let cpu = Rc::new(RefCell::new(CPU::new(Rc::clone(&memory), interrupt_controller)));
    dma.set_cpu(cpu);
    dma.set_memory(memory);
    dma
  }

  #[test]
  fn start_legacy_dma_transfer() {
    let mut dma = create_dma();
    dma.write(0xFF46, 0xC0);
    for (index, address) in (0xFE00u16..=0xFE9Fu16).enumerate() {
      assert_eq_hex!(dma.memory.as_ref().unwrap().borrow().read(address), 0x0000);
      dma.handle_tick(false);
      assert_eq_hex!(dma.memory.as_ref().unwrap().borrow().read(address), index as u8);
    }
    dma.handle_tick(false);
    assert_eq_hex!(dma.memory.as_ref().unwrap().borrow().read(0xFEA0), 0x0000);
  }

  #[test]
  fn start_general_purpose_dma_transfer() {
    let mut dma = create_dma();
    dma.write(0xFF51, 0xC0);
    dma.write(0xFF52, 0x05); // 5 should be masked away
    dma.write(0xFF53, 0x01); // Should be masked with 0x1F so that result is 0x81
    dma.write(0xFF54, 0x23); // 3 should be masked away -> result is 0x20
    assert_eq!(dma.cpu.as_ref().unwrap().borrow().enabled(), true);
    dma.write(0xFF55, 0x06); // Transfer 7 lines = 7 x 16 byte = 112 byte
    for (index, address) in (0x8120u16..=0x818Fu16).enumerate() {
      assert_eq_hex!(dma.memory.as_ref().unwrap().borrow().read(address), 0x0000);
      dma.handle_tick(false);
      assert_eq!(dma.cpu.as_ref().unwrap().borrow().enabled(), index == 111);
      assert_eq_hex!(dma.memory.as_ref().unwrap().borrow().read(address), index as u8);
    }
    assert_eq!(dma.cpu.as_ref().unwrap().borrow().enabled(), true);
    assert_eq_hex!(dma.read(0xFF55), 0xFF);
    dma.handle_tick(false);
    assert_eq_hex!(dma.memory.as_ref().unwrap().borrow().read(0xFEA0), 0x0000);
  }

  #[test]
  fn start_hblank_dma_transfer() {
    let mut dma = create_dma();
    dma.write(0xFF51, 0xC0);
    dma.write(0xFF52, 0x05); // 5 should be masked away
    dma.write(0xFF53, 0x01); // Should be masked with 0x1F so that result is 0x81
    dma.write(0xFF54, 0x23); // 3 should be masked away -> result is 0x20
    dma.write(0xFF55, 0x86); // Transfer 7 lines = 7 x 16 byte = 112 byte
    for (index, address) in (0x8120u16..=0x818Fu16).enumerate() {
      assert_eq_hex!(dma.memory.as_ref().unwrap().borrow().read(address), 0x0000);
      dma.memory.as_ref().unwrap().borrow_mut().write(0xFF41, 0x01); // Explicitly Set LCD to mode 1; Outside of hblank, no byte should be transferred
      dma.handle_tick(false);
      assert_eq_hex!(dma.memory.as_ref().unwrap().borrow().read(address), 0x0000);
      dma.memory.as_ref().unwrap().borrow_mut().write(0xFF41, 0x00); // Explicitly Set LCD to hblank
      dma.handle_tick(false);
      assert_eq_hex!(dma.memory.as_ref().unwrap().borrow().read(address), index as u8);
    }
    assert_eq_hex!(dma.read(0xFF55), 0xFF);
    dma.handle_tick(false);
    assert_eq_hex!(dma.memory.as_ref().unwrap().borrow().read(0xFEA0), 0x0000);
  }

  #[test]
  fn cancel_hblank_dma_transfer() {
    let mut dma = create_dma();
    dma.write(0xFF51, 0xC0);
    dma.write(0xFF52, 0x05); // 5 should be masked away
    dma.write(0xFF53, 0x01); // Should be masked with 0x1F so that result is 0x81
    dma.write(0xFF54, 0x23); // 3 should be masked away -> result is 0x20
    dma.write(0xFF55, 0x86); // Transfer 7 lines = 7 x 16 byte = 112 byte
    for _ in (0..0x20).enumerate() { // Transfer only 2 lines
      dma.handle_tick(false);
    }
    // Cancel the HBlank DMA transfer
    dma.write(0xFF55, 0x00);
    assert_eq_hex!(dma.read(0xFF55), 0x84);
    dma.handle_tick(false);
    assert_eq_hex!(dma.memory.as_ref().unwrap().borrow().read(0xFEA0), 0x0000);
  }
}