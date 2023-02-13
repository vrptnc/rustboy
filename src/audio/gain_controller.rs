use crate::audio::audio_driver::{AudioDriver, Channel};

pub enum GainControllerTickResult {
  Ok,
  DacShutOff
}

#[derive(Copy, Clone)]
pub struct GainControllerSettings {
  pub initial_value: u8,
  pub pace: u8,
  pub ascending: bool,
}

impl GainControllerSettings {
  pub fn new() -> Self {
    GainControllerSettings {
      initial_value: 0,
      pace: 0,
      ascending: false,
    }
  }
}

pub struct GainController {
  channel: Channel,
  current_tick: u8,
  current_value: u8,
  current_settings: GainControllerSettings,
  pub new_settings: GainControllerSettings,
  active: bool,
}

impl GainController {
  pub fn new(channel: Channel) -> Self {
    GainController {
      channel,
      current_tick: 0,
      current_value: 0,
      current_settings: GainControllerSettings::new(),
      new_settings: GainControllerSettings::new(),
      active: false,
    }
  }

  pub fn stop(&mut self) {
    self.active = false;
  }

  pub fn trigger(&mut self) {
    self.current_settings = self.new_settings;
    self.current_tick = 0;
    self.current_value = self.current_settings.initial_value;
    self.active = true;
  }

  fn dac_shut_off(&self) -> bool {
    self.current_settings.initial_value == 0 && !self.current_settings.ascending
  }

  pub fn tick(&mut self, audio_driver: &mut dyn AudioDriver) -> GainControllerTickResult {
    if !self.active {
      return GainControllerTickResult::Ok;
    }
    if self.dac_shut_off() {
      return GainControllerTickResult::DacShutOff;
    }
    if self.current_settings.pace == 0 {
      return GainControllerTickResult::Ok;
    }
    self.current_tick = (self.current_tick + 1) % self.current_settings.pace;
    if self.current_tick == 0 {
      if self.current_settings.ascending && self.current_value < 0xF {
        self.current_value += 1;
      } else if !self.current_settings.ascending && self.current_value > 0 {
        self.current_value -= 1;
      }
    }
    audio_driver.set_gain(self.channel, (self.current_value as f32) / 15.0);
    GainControllerTickResult::Ok
  }
}