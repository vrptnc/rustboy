use std::cell::RefCell;
use std::rc::Rc;
use crate::cpu::interrupts::{InterruptController, InterruptControllerRef};
use crate::features::dma::{DMA, DMARef};
use crate::features::oam::OAM;
use crate::features::timer::{Timer, TimerRef};
use crate::memory::memory::MemoryRef;
use crate::MemoryBus;

pub struct Emulator {

}

impl Emulator {

  pub fn run() {
    let interrupt_controller = Rc::new(RefCell::new(InterruptController::new()));
    let timer = Rc::new(RefCell::new(Timer::new(Rc::clone(&interrupt_controller))));
    let oam = Rc::new(RefCell::new(OAM::new()));
    let dma: DMARef = Rc::new(RefCell::new(DMA::new()));


  }

}