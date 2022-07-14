use crate::time::time::ClockAware;
use crate::cpu::interrupts::{Interrupt, InterruptControllerRef};
use crate::memory::memory::Memory;
use crate::util::bit_util::BitUtil;

pub struct Timer {
  interrupt_controller: InterruptControllerRef,
  clock_pulse_bit: u8,
  divider: u16,
  timer_modulo: u8,
  timer_controller: u8,
  timer_counter: u8,
  enabled: bool,
}

impl ClockAware for Timer {
  fn handle_tick(&mut self, _double_speed: bool) {
    let old_div = self.divider;
    self.divider = self.divider.wrapping_add(4);
    if self.enabled {
      if !old_div.get_bit(self.clock_pulse_bit) && self.divider.get_bit(self.clock_pulse_bit) {
        let (_, tima_overflowed) = self.timer_counter.overflowing_add(1);
        if tima_overflowed {
          self.timer_counter = self.timer_modulo;
          self.interrupt_controller.borrow_mut().request_interrupt(Interrupt::TimerOverflow);
        }
      }
    }
  }
}

impl Memory for Timer {
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