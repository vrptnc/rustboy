#[derive(Copy, Clone)]
pub struct PulseOptions {
  pub frequency: f32,
  pub duty_cycle: DutyCycle,
}

#[derive(Copy, Clone)]
pub struct CustomWaveOptions {
  pub data: [f32;32],
  pub frequency: f32,
  pub gain: f32
}

#[derive(Copy, Clone)]
pub enum Channel {
  CH1,
  CH2,
  CH3,
  CH4,
}

#[derive(Copy, Clone)]
pub enum DutyCycle {
  Duty125,
  Duty250,
  Duty500,
  Duty750,
}

pub trait AudioDriver {
  fn play_pulse(&mut self, channel: Channel, pulse_options: PulseOptions);
  fn play_custom_wave(&mut self, channel: Channel, wave_options: CustomWaveOptions);
  fn stop(&mut self, channel: Channel);
  fn set_gain(&mut self, channel: Channel, gain: f32);

  fn mute_all(&mut self);
  fn unmute_all(&mut self);
  fn set_master_volume(&mut self, value: u8);
}