use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::AudioContext;
use crate::audio::web_audio_driver::WebAudioDriver;
use crate::controllers::buttons::Button;
use crate::cpu::cpu::CPUInfo;
use crate::emulator::emulator::Emulator;
use crate::memory::oam::OAMObject;
use crate::renderer::canvas_renderer::CompositeCanvasRenderer;

#[wasm_bindgen]
struct WebEmulator {
  emulator: Emulator<WebAudioDriver, CompositeCanvasRenderer>
}

#[wasm_bindgen]
impl WebEmulator {
  pub fn new(rom_bytes: &[u8], audio_context: AudioContext) -> Self {
    let audio_driver = WebAudioDriver::new(audio_context);
    let renderer = CompositeCanvasRenderer::new();
    WebEmulator {
      emulator: Emulator::new(rom_bytes, audio_driver, renderer)
    }
  }

  pub fn press_button(&mut self, button: Button) {
    self.emulator.press_button(button);
  }

  pub fn release_button(&mut self, button: Button) {
    self.emulator.release_button(button);
  }

  pub fn cpu_info(&self) -> CPUInfo {
    self.emulator.cpu_info()
  }

  pub fn get_object(&self, object_index: u8) -> OAMObject {
    self.emulator.get_object(object_index)
  }

  pub fn set_tile_atlas_rendering_enabled(&mut self, enabled: bool) {
    self.emulator.set_tile_atlas_rendering_enabled(enabled);
  }

  pub fn set_object_atlas_rendering_enabled(&mut self, enabled: bool) {
    self.emulator.set_object_atlas_rendering_enabled(enabled);
  }

  pub fn run_for_nanos(&mut self, nanos: u64) {
    self.emulator.run_for_nanos(nanos);
  }
}