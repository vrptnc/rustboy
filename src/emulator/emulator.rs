use std::cell::RefCell;
use std::rc::Rc;
use crate::cpu::interrupts::{InterruptController, InterruptControllerRef};
use crate::controllers::dma::{DMAController, DMAControllerRef};
use crate::memory::oam::OAM;
use crate::controllers::timer::{TimerController, TimerControllerRef};
use crate::memory::memory::MemoryRef;
use crate::MemoryBus;

pub struct Emulator {

}

impl Emulator {

  pub fn run() {
    let interrupt_controller = Rc::new(RefCell::new(InterruptController::new()));
    let timer = Rc::new(RefCell::new(TimerController::new(Rc::clone(&interrupt_controller))));
    let oam = Rc::new(RefCell::new(OAM::new()));
    let dma: DMAControllerRef = Rc::new(RefCell::new(DMAController::new()));


  }

}