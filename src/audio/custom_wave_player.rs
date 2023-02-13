use crate::audio::audio_driver::{AudioDriver, Channel, CustomWaveOptions};
use crate::util::request_flag::RequestFlag;

pub enum CustomWavePlayerTickResult {
  Ok,
  DacShutOff
}

pub struct CustomWavePlayer {
  channel: Channel,
  pub waveform: [u8; 16],
  triggered: RequestFlag,
  pub wavelength: u16,
  pub gain: u8,
  pub playing: bool,
  pub dac_enabled: bool
}

impl CustomWavePlayer {
  pub fn new(channel: Channel) -> Self {
    CustomWavePlayer {
      channel,
      waveform: [0; 16],
      triggered: RequestFlag::new(),
      wavelength: 0,
      gain: 0,
      playing: false,
      dac_enabled: false
    }
  }

  pub fn trigger(&mut self) {
    self.triggered.set();
  }

  pub fn stop(&mut self) {
    self.playing = false;
  }

  pub fn get_lower_wavelength_bits(&self) -> u8 {
    (self.wavelength & 0xFF) as u8
  }

  pub fn get_upper_wavelength_bits(&self) -> u8 {
    ((self.wavelength & 0xFF00) >> 8) as u8
  }

  pub fn set_lower_wavelength_bits(&mut self, value: u8) {
    self.wavelength = (self.wavelength & 0xFF00) | (value as u16);
  }

  pub fn set_upper_wavelength_bits(&mut self, value: u8) {
    self.wavelength = (self.wavelength & 0x00FF) | ((value as u16 & 0x7) << 8);
  }

  pub fn tick(&mut self, audio_driver: &mut dyn AudioDriver) -> CustomWavePlayerTickResult {
    if !self.dac_enabled {
      return CustomWavePlayerTickResult::DacShutOff;
    }
    let frequency = 65536.0f32 / (2048.0 - self.wavelength as f32);
    audio_driver.set_frequency(self.channel, frequency);
    if self.triggered.get_and_clear() {
      self.playing = true;
      audio_driver.play_custom_wave(self.channel, CustomWaveOptions {
        data: self.waveform,
      });
    }
    let gain = match self.gain {
      1 => 1.0f32,
      2 => 0.5f32,
      3 => 0.25f32,
      _ => 0.0f32,
    };
    audio_driver.set_gain(self.channel, gain);
    CustomWavePlayerTickResult::Ok
  }
}