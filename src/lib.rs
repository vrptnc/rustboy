extern crate core;

mod util;
mod memory;
mod cpu;
mod features;
mod time;
mod infrastructure;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use cpu::cpu::*;
use memory::main::*;
use web_sys::console;
use crate::infrastructure::time::clock::JSClock;
use crate::time::duration::Duration;
use crate::time::time::Clock;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn run_emulator() {
  let clock = JSClock::new();
  let mut previous = clock.now();
  loop {
    let current = clock.now();
    let delta = current - previous;
    let mut ticks_to_execute = delta / Duration::from_nanoseconds(1000);
    while ticks_to_execute > 0 {

      ticks_to_execute -= 1;
    }

  }
  console::log_1(&"Emulator is now running".into());
}
