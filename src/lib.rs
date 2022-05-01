mod util;
mod memory;
mod cpu;
mod features;
mod context;
mod time;

use wasm_bindgen::prelude::*;

use cpu::cpu::*;
use memory::main::*;
use web_sys::console;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn run_emulator() {
  console::log_1(&"Emulator is now running".into());
  // let mut cpu = CPU::new();
  // let mut memory = MainMemory::new()
  //
  // loop {
  //   //TODO: Introduce some logic that throttles the ticks so that they are executed at the correct pace
  //
  //
  // }
}
