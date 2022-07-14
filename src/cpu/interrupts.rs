use std::cell::RefCell;
use std::rc::Rc;
use crate::memory::memory::Memory;
use crate::util::bit_util::BitUtil;

pub type InterruptControllerRef = Rc<RefCell<InterruptController>>;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Interrupt {
  VerticalBlank,
  LCDC,
  TimerOverflow,
  SerialIOComplete,
  ButtonPressed,
}

impl Interrupt {
  pub fn get_bit(&self) -> u8 {
    match self {
      Interrupt::VerticalBlank => 0,
      Interrupt::LCDC => 1,
      Interrupt::TimerOverflow => 2,
      Interrupt::SerialIOComplete => 3,
      Interrupt::ButtonPressed => 4
    }
  }

  pub fn get_routine_address(&self) -> u16 {
    match self {
      Interrupt::VerticalBlank => 0x40,
      Interrupt::LCDC => 0x48,
      Interrupt::TimerOverflow => 0x50,
      Interrupt::SerialIOComplete => 0x58,
      Interrupt::ButtonPressed => 0x60
    }
  }

  pub fn from_bit(bit: u8) -> Self {
    match bit {
      0 => Interrupt::VerticalBlank,
      1 => Interrupt::LCDC,
      2 => Interrupt::TimerOverflow,
      3 => Interrupt::SerialIOComplete,
      4 => Interrupt::ButtonPressed,
      _ => panic!("Can't map bit {} to Interrupt", bit)
    }
  }
}

pub struct InterruptController {
  interrupt_request: u8,
  interrupt_enable: u8,
  interrupt_master_enable: bool,
}

impl InterruptController {
  pub fn new() -> InterruptController {
    InterruptController {
      interrupt_request: 0,
      interrupt_enable: 0,
      interrupt_master_enable: false,
    }
  }

  pub fn get_requested_interrupt(&self) -> Option<Interrupt> {
    if !self.interrupt_master_enable {
      Option::None
    } else {
      let masked_request = 0x1F & self.interrupt_enable & self.interrupt_request;
      if masked_request == 0 {
        Option::None
      } else {
        Option::Some(Interrupt::from_bit(masked_request.trailing_zeros() as u8))
      }
    }
  }

  pub fn interrupts_enabled(&self) -> bool {
    self.interrupt_master_enable
  }

  pub fn enable_interrupts(&mut self) {
    self.interrupt_master_enable = true;
  }

  pub fn disable_interrupts(&mut self) {
    self.interrupt_master_enable = false;
  }

  pub fn request_interrupt(&mut self, interrupt: Interrupt) {
    self.interrupt_request = self.interrupt_request.set_bit(interrupt.get_bit());
  }

  pub fn clear_interrupt(&mut self, interrupt: Interrupt) {
    self.interrupt_request = self.interrupt_request.reset_bit(interrupt.get_bit());
  }
}

impl Memory for InterruptController {
  fn read(&self, address: u16) -> u8 {
    match address {
      0xFF0F => self.interrupt_request,
      0xFFFF => self.interrupt_enable,
      _ => panic!("InterruptController can't read address {}", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      0xFF0F => self.interrupt_request = value,
      0xFFFF => self.interrupt_enable = value,
      _ => panic!("InterruptController can't write to address {}", address)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn get_requested_interrupt_returns_highest_priority() {
    let mut interrupt_controller = InterruptController::new();
    interrupt_controller.request_interrupt(Interrupt::SerialIOComplete);
    interrupt_controller.request_interrupt(Interrupt::LCDC);
    interrupt_controller.enable_interrupts();
    interrupt_controller.write(0xFFFF, 0xFF);
    assert_eq!(interrupt_controller.get_requested_interrupt(), Some(Interrupt::LCDC));
    interrupt_controller.clear_interrupt(Interrupt::LCDC);
    assert_eq!(interrupt_controller.get_requested_interrupt(), Some(Interrupt::SerialIOComplete));
    interrupt_controller.clear_interrupt(Interrupt::SerialIOComplete);
    assert_eq!(interrupt_controller.get_requested_interrupt(), None);
  }

  #[test]
  fn interrupts_are_correctly_enabled() {
    let mut interrupt_controller = InterruptController::new();
    interrupt_controller.request_interrupt(Interrupt::SerialIOComplete);
    interrupt_controller.request_interrupt(Interrupt::LCDC);
    interrupt_controller.enable_interrupts();
    interrupt_controller.write(0xFFFF, 0x08);
    assert_eq!(interrupt_controller.get_requested_interrupt(), Some(Interrupt::SerialIOComplete));
    interrupt_controller.clear_interrupt(Interrupt::SerialIOComplete);
    assert_eq!(interrupt_controller.get_requested_interrupt(), None);
    interrupt_controller.write(0xFFFF, 0x02);
    assert_eq!(interrupt_controller.get_requested_interrupt(), Some(Interrupt::LCDC));
  }

  #[test]
  fn master_enable_toggles_interrupts() {
    let mut interrupt_controller = InterruptController::new();
    interrupt_controller.request_interrupt(Interrupt::SerialIOComplete);
    interrupt_controller.request_interrupt(Interrupt::LCDC);
    interrupt_controller.write(0xFFFF, 0xFF);
    interrupt_controller.disable_interrupts();
    assert_eq!(interrupt_controller.get_requested_interrupt(), None);
    interrupt_controller.enable_interrupts();
    assert_eq!(interrupt_controller.get_requested_interrupt(), Some(Interrupt::LCDC));
  }
}

