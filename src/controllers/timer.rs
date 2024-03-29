use std::cell::RefCell;
use std::rc::Rc;
use crate::time::time::ClockAware;
use crate::cpu::interrupts::{Interrupt, InterruptController, InterruptControllerRef};
use crate::memory::memory::Memory;
use crate::util::bit_util::BitUtil;

pub trait TimerController {
  fn tick(&mut self, interrupt_controller: &mut dyn InterruptController);
}

pub struct TimerControllerImpl {
  clock_pulse_bit: u8,
  divider: u16,
  timer_modulo: u8,
  timer_controller: u8,
  timer_counter: u8,
  enabled: bool,
}

impl TimerControllerImpl {
  pub fn new() -> TimerControllerImpl {
    TimerControllerImpl {
      clock_pulse_bit: 0,
      divider: 0,
      timer_modulo: 0,
      timer_controller: 0,
      timer_counter: 0,
      enabled: false,
    }
  }
}

impl TimerController for TimerControllerImpl {
  fn tick(&mut self, interrupt_controller: &mut dyn InterruptController) {
    let old_div = self.divider;
    self.divider = self.divider.wrapping_add(4);
    if self.enabled {
      if old_div.get_bit(self.clock_pulse_bit) ^ self.divider.get_bit(self.clock_pulse_bit) {
        let (new_timer_counter, tima_overflowed) = self.timer_counter.overflowing_add(1);
        if tima_overflowed {
          self.timer_counter = self.timer_modulo;
          interrupt_controller.request_interrupt(Interrupt::TimerOverflow);
        } else {
          self.timer_counter = new_timer_counter;
        }
      }
    }
  }
}

impl Memory for TimerControllerImpl {
  fn read(&self, address: u16) -> u8 {
    match address {
      0xFF04 => self.divider.get_upper_byte(),
      0xFF05 => self.timer_counter,
      0xFF06 => self.timer_modulo,
      0xFF07 => self.timer_controller,
      _ => panic!("Can't read address {} on timer", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      0xFF04 => self.divider = 0,
      0xFF05 => self.timer_counter = value,
      0xFF06 => self.timer_modulo = value,
      0xFF07 => {
        self.enabled = value.get_bit(2);
        self.clock_pulse_bit = match value & 0x03 {
          0x00 => 10,
          0x01 => 4,
          0x02 => 6,
          0x03 => 8,
          _ => 10
        };
        self.timer_controller = value
      }
      _ => panic!("Can't write to address {} on timer", address)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use test_case::test_case;
  use crate::cpu::interrupts::InterruptControllerImpl;

  fn timer_ticks(timer: &mut dyn TimerController, interrupt_controller: &mut dyn InterruptController, ticks: usize) {
    for _ in 0..ticks {
      timer.tick(interrupt_controller);
    }
  }

  #[test]
  fn read_divider() {
    let mut interrupt_controller = InterruptControllerImpl::new();
    let mut timer = TimerControllerImpl::new();
    // It takes 64 ticks to increment the DIV register by one, so 320 ticks should increment it by 5
    timer_ticks(&mut timer, &mut interrupt_controller, 320);
    assert_eq!(timer.read(0xFF04), 5);
  }

  #[test_case(0x04, 256; "Timer @ 4096 Hz")]
  #[test_case(0x05, 4; "Timer @ 262144 Hz")]
  #[test_case(0x06, 16; "Timer @ 65536 Hz")]
  #[test_case(0x07, 64; "Timer @ 16384 Hz")]
  fn read_tima(tac_register: u8, ticks_per_timer_increment: usize) {
    let mut interrupt_controller = InterruptControllerImpl::new();
    let mut timer = TimerControllerImpl::new();
    timer.write(0xFF07, tac_register);
    timer_ticks(&mut timer, &mut interrupt_controller, ticks_per_timer_increment - 1);
    assert_eq!(timer.read(0xFF05), 0u8);
    timer.tick(&mut interrupt_controller);
    assert_eq!(timer.read(0xFF05), 1u8);
    timer_ticks(&mut timer, &mut interrupt_controller, ticks_per_timer_increment);
    assert_eq!(timer.read(0xFF05), 2u8);
  }

  #[test_case(0x04, 0x10000; "4096 Hz")]
  #[test_case(0x05, 0x00400; "262144 Hz")]
  #[test_case(0x06, 0x01000; "65536 Hz")]
  #[test_case(0x07, 0x04000; "16384 Hz")]
  fn timer_overflow(tac_register: u8, ticks_per_overflow: usize) {
    let mut interrupt_controller = InterruptControllerImpl::new();
    interrupt_controller.enable_interrupts();
    interrupt_controller.write(0xFFFF, 0x04);
    let mut timer = TimerControllerImpl::new();
    timer.write(0xFF07, tac_register);
    timer_ticks(&mut timer, &mut interrupt_controller, ticks_per_overflow - 1);
    assert!(interrupt_controller.get_requested_interrupt().is_none());
    timer.tick(&mut interrupt_controller);
    assert!(matches!(interrupt_controller.get_requested_interrupt().unwrap(), Interrupt::TimerOverflow));
    interrupt_controller.clear_interrupt(Interrupt::TimerOverflow);
    assert!(interrupt_controller.get_requested_interrupt().is_none());
    timer_ticks(&mut timer, &mut interrupt_controller, ticks_per_overflow);
    assert!(matches!(interrupt_controller.get_requested_interrupt().unwrap(), Interrupt::TimerOverflow));
  }

  #[test_case(0x04, 0x10000; "4096 Hz")]
  #[test_case(0x05, 0x00400; "262144 Hz")]
  #[test_case(0x06, 0x01000; "65536 Hz")]
  #[test_case(0x07, 0x04000; "16384 Hz")]
  fn timer_modulo(tac_register: u8, ticks_per_overflow: usize) {
    let mut interrupt_controller = InterruptControllerImpl::new();
    let mut timer = TimerControllerImpl::new();
    timer.write(0xFF06, 0xAB);
    timer.write(0xFF07, tac_register);
    timer_ticks(&mut timer, &mut interrupt_controller, ticks_per_overflow - 1);
    assert_eq!(timer.read(0xFF05), 0xFF);
    timer.tick(&mut interrupt_controller);
    assert_eq!(timer.read(0xFF05), 0xAB);
  }
}