extern crate alloc;
extern crate core;

mod emulator;
mod renderer;
mod util;
mod memory;
mod cpu;
mod controllers;
mod time;
mod infrastructure;
mod audio;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use cpu::cpu::*;
use web_sys::console;
use crate::infrastructure::time::clock::JSClock;


#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

