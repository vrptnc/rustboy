extern crate alloc;
extern crate core;
extern crate serde;

mod emulator;
mod renderer;
mod util;
mod memory;
mod cpu;
mod controllers;
mod infrastructure;
mod audio;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use cpu::cpu::*;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

