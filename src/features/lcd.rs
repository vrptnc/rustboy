use std::cell::RefCell;
use std::rc::Rc;
use crate::cpu::interrupts::{Interrupt, InterruptControllerRef};
use crate::memory::memory::MemoryRef;
use crate::time::time::ClockAware;

pub type LCDRef = Rc<RefCell<LCD>>;
const DotsPerFrame: u32 = 70224;

pub struct LCD {
  interrupt_controller: InterruptControllerRef,
  memory: MemoryRef,
  dot: u32
}

impl LCD {
  pub fn new(interrupt_controller: InterruptControllerRef, memory: MemoryRef) -> LCD {
    LCD {
      interrupt_controller,
      memory,
      dot: 0,
    }
  }

  pub fn current_line(&self) -> u8 {
    (self.dot / 456) as u8
  }

  pub fn current_column(&self) -> u16 {
    (self.dot % 456) as u16
  }

  pub fn horizontal_blank(&self) -> bool {
    self.current_column() >= 370
  }

  pub fn vertical_blank(&self) -> bool {
    self.current_line() >= 144
  }

  fn draw_line(&self) {
    todo!("Draw the current line")
  }
}

impl ClockAware for LCD {
  fn handle_tick(&mut self, double_speed: bool) {
    self.dot = (self.dot + 4) % DotsPerFrame;
    if self.current_line() == 144 {
      self.interrupt_controller.borrow_mut().request_interrupt(Interrupt::VerticalBlank);
    }

    self.column = (self.column + 4) % 456;
    if self.column == 0 {

      self.line = (self.line + 1) % 154;
      if self.line == 144 {
      }
    }
  }
}