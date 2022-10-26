use std::cell::RefCell;
use std::rc::Rc;
use crate::cpu::interrupts::{InterruptControllerImpl, InterruptControllerRef};
use crate::controllers::dma::{DMAControllerImpl};
use crate::memory::oam::OAMImpl;
use crate::controllers::timer::TimerController;
use crate::MainMemory;

pub struct Emulator {

}

impl Emulator {

  pub fn run() {
    // let interrupt_controller = Rc::new(RefCell::new(InterruptController::new()));
    // let timer = Rc::new(RefCell::new(TimerController::new(Rc::clone(&interrupt_controller))));
    // let oam = Rc::new(RefCell::new(OAM::new()));
    // let dma: DMAControllerRef = Rc::new(RefCell::new(DMAController::new()));


  }

}