use mockall::automock;
use web_sys::console;

use crate::audio::audio_driver::{AudioDriver, Channel, CustomWaveOptions, DutyCycle, PulseOptions};
use crate::controllers::timer::TimerController;
use crate::memory::memory::{Memory, MemoryAddress};
use crate::util::bit_util::BitUtil;

//Note: Frequencies expressed in binary in the register can be converted to Hz using the formula:
// f = 131072 / (2048 - X)

#[derive(Copy, Clone)]
pub struct WavelengthSweeperSettings {
  initial_value: u16,
  shift: u8,
  pace: u8,
  decrease: bool,
  duty_cycle: DutyCycle,
}

impl WavelengthSweeperSettings {
  pub fn new() -> Self {
    WavelengthSweeperSettings {
      initial_value: 0,
      shift: 0,
      pace: 0,
      decrease: false,
      duty_cycle: DutyCycle::Duty125,
    }
  }

  pub fn get_lower_wavelength_bits(&self) -> u8 {
    (self.initial_value & 0xFF) as u8
  }

  pub fn get_upper_wavelength_bits(&self) -> u8 {
    ((self.initial_value & 0xFF00) >> 8) as u8
  }

  pub fn set_lower_wavelength_bits(&mut self, value: u8) {
    self.initial_value = (self.initial_value & 0xFF00) | (value as u16);
  }

  pub fn set_upper_wavelength_bits(&mut self, value: u8) {
    self.initial_value = (self.initial_value & 0x00FF) | ((value as u16 & 0x7) << 8);
  }
}

pub struct WavelengthSweeper {
  channel: Channel,
  triggered: bool,
  current_tick: u8,
  current_value: u16,
  current_settings: WavelengthSweeperSettings,
  new_settings: WavelengthSweeperSettings,
  operational: bool
}

impl WavelengthSweeper {
  pub fn new(channel: Channel) -> Self {
    WavelengthSweeper {
      channel,
      triggered: false,
      current_tick: 0,
      current_value: 0,
      current_settings: WavelengthSweeperSettings::new(),
      new_settings: WavelengthSweeperSettings::new(),
      operational: false
    }
  }

  pub fn trigger(&mut self) {
    self.triggered = true;
    self.current_settings = self.new_settings;
    self.current_tick = 0;
    self.current_value = self.current_settings.initial_value;
    self.operational = true
  }

  pub fn tick_and_check_if_wavelength_overflowed(&mut self, audio_driver: &mut dyn AudioDriver) -> bool {
    if self.operational {
      if self.triggered {
        self.triggered = false;
        audio_driver.play_pulse(self.channel, PulseOptions {
          frequency: 131072.0f32 / (2048.0 - self.current_value as f32),
          duty_cycle: self.current_settings.duty_cycle,
        });
      }
      if self.current_settings.pace != 0 && self.current_settings.shift != 0 {
        self.current_tick = (self.current_tick + 1) % self.current_settings.pace;
        if self.current_tick == 0 {
          if self.current_settings.decrease {
            self.current_value -= (self.current_value >> self.current_settings.shift);
          } else {
            self.current_value += (self.current_value >> self.current_settings.shift);
          }
          audio_driver.play_pulse(self.channel, PulseOptions {
            frequency: 131072.0f32 / (2048.0 - self.current_value as f32),
            duty_cycle: self.current_settings.duty_cycle,
          });
        }
      }
      self.current_value > 0x7FF
    } else {
      false
    }
  }
}

#[derive(Copy, Clone)]
pub struct LengthTimerSettings {
  initial_value: u16,
}

impl LengthTimerSettings {
  pub fn new() -> Self {
    LengthTimerSettings {
      initial_value: 0
    }
  }
}

pub struct LengthTimer {
  channel: Channel,
  current_value: u16,
  max_value: u16,
  current_settings: LengthTimerSettings,
  new_settings: LengthTimerSettings,
  enabled: bool,
  operational: bool
}

impl LengthTimer {
  pub fn new(channel: Channel, max_value: u16) -> Self {
    LengthTimer {
      channel,
      current_value: 0,
      max_value,
      current_settings: LengthTimerSettings::new(),
      new_settings: LengthTimerSettings::new(),
      enabled: false,
      operational: false
    }
  }

  pub fn trigger(&mut self) {
    self.current_settings = self.new_settings;
    self.current_value = self.current_settings.initial_value;
    self.operational = true;
  }

  pub fn tick_and_check_if_expired(&mut self) -> bool {
    if self.operational && self.enabled {
      self.current_value = self.current_value.saturating_sub(1);
      self.current_value == 0
    } else {
      false
    }
  }

  pub fn set_length(&mut self, length: u8) {
    self.new_settings.initial_value = self.max_value - length as u16;
  }

  pub fn length(&self) -> u8 {
    (self.max_value - self.new_settings.initial_value) as u8
  }
}

#[derive(Copy, Clone)]
pub struct EnvelopeSweeperSettings {
  initial_value: u8,
  pace: u8,
  ascending: bool,
}

impl EnvelopeSweeperSettings {
  pub fn new() -> Self {
    EnvelopeSweeperSettings {
      initial_value: 0,
      pace: 0,
      ascending: false,
    }
  }
}

pub struct EnvelopeSweeper {
  channel: Channel,
  current_tick: u8,
  current_value: u8,
  current_settings: EnvelopeSweeperSettings,
  new_settings: EnvelopeSweeperSettings,
  operational: bool
}

impl EnvelopeSweeper {
  pub fn new(channel: Channel) -> Self {
    EnvelopeSweeper {
      channel,
      current_tick: 0,
      current_value: 0,
      current_settings: EnvelopeSweeperSettings::new(),
      new_settings: EnvelopeSweeperSettings::new(),
      operational: false
    }
  }

  pub fn trigger(&mut self) {
    self.current_settings = self.new_settings;
    self.current_tick = 0;
    self.current_value = self.current_settings.initial_value;
    self.operational = true;
  }

  pub fn tick_and_check_if_dac_shutoff(&mut self, audio_driver: &mut dyn AudioDriver) -> bool {
    if self.operational {
      if self.new_settings.initial_value == 0 && !self.new_settings.ascending {
        // Turn off DAC
        true
      } else if self.current_settings.pace != 0 {
        self.current_tick = (self.current_tick + 1) % self.current_settings.pace;
        if self.current_tick == 0 {
          if self.current_settings.ascending && self.current_value < 0xF {
            self.current_value += 1;
          } else if !self.current_settings.ascending && self.current_value > 0 {
            self.current_value -= 1;
          }
        }
        audio_driver.set_gain(self.channel, (self.current_value as f32) / 15.0);
        false
      } else {
        false
      }
    } else {
      false
    }
  }
}

pub struct CustomWavePlayer {
  channel: Channel,
  waveform: [u8; 16],
  triggered: bool,
  wavelength: u16,
  gain: u8,
  enabled: bool,
}

impl CustomWavePlayer {
  pub fn new(channel: Channel) -> Self {
    CustomWavePlayer {
      channel,
      waveform: [0; 16],
      triggered: false,
      wavelength: 0,
      gain: 0,
      enabled: false,
    }
  }

  pub fn trigger(&mut self) {
    self.triggered = true;
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

  pub fn tick(&mut self, audio_driver: &mut dyn AudioDriver) {
    if self.triggered && self.enabled {
      self.triggered = false;
      let mut data: [f32; 32] = [0.0; 32];
      (0..32usize).for_each(|index| {
        let byte = self.waveform[index / 2];
        let nibble = if index % 2 == 0 {
          byte >> 4
        } else {
          byte & 0x0F
        };
        data[index] = -(nibble as f32) / 15.0;
      });
      let gain = match self.gain {
        1 => 1.0f32,
        2 => 0.5f32,
        3 => 0.25f32,
        _ => 0.0f32,
      };
      audio_driver.play_custom_wave(self.channel, CustomWaveOptions {
        data,
        frequency: 65536.0f32 / (2048.0 - self.wavelength as f32),
        gain,
      })
    }
    if !self.enabled {
      audio_driver.stop(self.channel)
    }
  }
}

#[automock]
pub trait AudioController {}

pub struct AudioControllerImpl {
  previous_timer_div: u8,
  div_apu: u16,
  ch1_length_timer: LengthTimer,
  ch2_length_timer: LengthTimer,
  ch3_length_timer: LengthTimer,
  ch4_length_timer: LengthTimer,
  ch1_envelope_sweeper: EnvelopeSweeper,
  ch1_wavelength_sweeper: WavelengthSweeper,
  ch2_envelope_sweeper: EnvelopeSweeper,
  ch2_wavelength_sweeper: WavelengthSweeper,
  ch3_custom_wave_player: CustomWavePlayer,
  nr41: u8,
  nr42: u8,
  nr43: u8,
  nr44: u8,
  master_volume: u8,
  mixing_control: u8,
  on_off_control: u8,
  waveform_ram: [u8; 16],
}

impl AudioControllerImpl {
  pub fn new() -> Self {
    let controller_impl = AudioControllerImpl {
      previous_timer_div: 0,
      div_apu: 0,
      ch1_length_timer: LengthTimer::new(Channel::CH1, 64),
      ch1_envelope_sweeper: EnvelopeSweeper::new(Channel::CH1),
      ch1_wavelength_sweeper: WavelengthSweeper::new(Channel::CH1),
      ch2_envelope_sweeper: EnvelopeSweeper::new(Channel::CH2),
      ch2_wavelength_sweeper: WavelengthSweeper::new(Channel::CH2),
      ch2_length_timer: LengthTimer::new(Channel::CH2, 64),
      ch3_length_timer: LengthTimer::new(Channel::CH3, 256),
      ch4_length_timer: LengthTimer::new(Channel::CH4, 64),
      ch3_custom_wave_player: CustomWavePlayer::new(Channel::CH3),
      nr41: 0,
      nr42: 0,
      nr43: 0,
      nr44: 0,
      master_volume: 0,
      mixing_control: 0,
      on_off_control: 0,
      waveform_ram: [0; 16],
    };
    controller_impl
  }

  fn length_timer_tick(&mut self, audio_driver: &mut dyn AudioDriver) {
    if self.ch1_length_timer.tick_and_check_if_expired() {
      self.stop(Channel::CH1, audio_driver);
    }
    if self.ch2_length_timer.tick_and_check_if_expired() {
      self.stop(Channel::CH2, audio_driver);
    }
    if self.ch3_length_timer.tick_and_check_if_expired() {
      self.stop(Channel::CH3, audio_driver);
    }
    if self.ch4_length_timer.tick_and_check_if_expired() {
      self.stop(Channel::CH4, audio_driver);
    }
  }

  fn envelope_sweep_tick(&mut self, audio_driver: &mut dyn AudioDriver) {
    if self.ch1_envelope_sweeper.tick_and_check_if_dac_shutoff(audio_driver) {
      self.stop(Channel::CH1, audio_driver);
    }
  }

  fn ch1_sweep_tick(&mut self, audio_driver: &mut dyn AudioDriver) {
    if self.ch1_wavelength_sweeper.tick_and_check_if_wavelength_overflowed(audio_driver) {
      self.stop(Channel::CH1, audio_driver);
    }
    if self.ch2_wavelength_sweeper.tick_and_check_if_wavelength_overflowed(audio_driver) {
      self.stop(Channel::CH2, audio_driver);
    }
    self.ch3_custom_wave_player.tick(audio_driver);
  }

  pub fn tick(&mut self, audio_driver: &mut dyn AudioDriver, timer: &dyn TimerController, double_speed: bool) {
    let new_timer_div = timer.get_divider().get_upper_byte();
    let divider_bit = if double_speed { 5 } else { 4 };
    if self.previous_timer_div.get_bit(divider_bit) && !new_timer_div.get_bit(divider_bit) {
      self.div_apu = self.div_apu.wrapping_add(1);
      if self.div_apu % 2 == 0 {
        self.length_timer_tick(audio_driver);
      }
      if self.div_apu % 4 == 0 {
        self.ch1_sweep_tick(audio_driver);
      }
      if self.div_apu % 8 == 0 {
        self.envelope_sweep_tick(audio_driver);
      }
    }
    self.previous_timer_div = new_timer_div;
  }

  fn trigger(&mut self, channel: Channel) {
    match channel {
      Channel::CH1 => {
        self.ch1_wavelength_sweeper.trigger();
        self.ch1_length_timer.trigger();
        self.ch1_envelope_sweeper.trigger();
      }
      Channel::CH2 => {
        self.ch2_wavelength_sweeper.trigger();
        self.ch2_length_timer.trigger();
        self.ch2_envelope_sweeper.trigger();
      }
      Channel::CH3 => {
        self.ch3_length_timer.trigger();
        self.ch3_custom_wave_player.trigger();
      }
      Channel::CH4 => {}
    }
  }

  fn stop(&mut self, channel: Channel, audio_driver: &mut dyn AudioDriver) {
    match channel {
      Channel::CH1 => {
        self.ch1_length_timer.operational = false;
        self.ch1_envelope_sweeper.operational = false;
        self.ch1_wavelength_sweeper.operational = false;
      }
      Channel::CH2 => {
        self.ch2_length_timer.operational = false;
      }
      Channel::CH3 => {
        self.ch3_length_timer.operational = false;
      }
      Channel::CH4 => {
        self.ch4_length_timer.operational = false;
      }
    }
    audio_driver.stop(channel)
  }
}

impl AudioController for AudioControllerImpl {}

impl Memory for AudioControllerImpl {
  fn read(&self, address: u16) -> u8 {
    match address {
      MemoryAddress::NR10 => {
        self.ch1_wavelength_sweeper.new_settings.shift |
          ((self.ch1_wavelength_sweeper.new_settings.decrease as u8) << 3) |
          (self.ch1_wavelength_sweeper.new_settings.pace << 4)
      }
      MemoryAddress::NR11 => {
        let duty_cycle_bits: u8 = match self.ch1_wavelength_sweeper.new_settings.duty_cycle {
          DutyCycle::Duty125 => 0,
          DutyCycle::Duty250 => 1,
          DutyCycle::Duty500 => 2,
          DutyCycle::Duty750 => 3
        };
        (duty_cycle_bits << 6) | self.ch1_length_timer.length()
      }
      MemoryAddress::NR12 => {
        self.ch1_envelope_sweeper.new_settings.pace |
          ((self.ch1_envelope_sweeper.new_settings.ascending as u8) << 3) |
          (self.ch1_envelope_sweeper.new_settings.initial_value << 4)
      }
      MemoryAddress::NR13 => self.ch1_wavelength_sweeper.new_settings.get_lower_wavelength_bits(),
      MemoryAddress::NR14 => {
        self.ch1_wavelength_sweeper.new_settings.get_upper_wavelength_bits() |
          ((self.ch1_length_timer.enabled as u8) << 6)
      }
      0xFF15 => 0,
      MemoryAddress::NR21 => {
        let duty_cycle_bits: u8 = match self.ch2_wavelength_sweeper.new_settings.duty_cycle {
          DutyCycle::Duty125 => 0,
          DutyCycle::Duty250 => 1,
          DutyCycle::Duty500 => 2,
          DutyCycle::Duty750 => 3
        };
        (duty_cycle_bits << 6) | self.ch2_length_timer.length()
      }
      MemoryAddress::NR22 => {
        self.ch2_envelope_sweeper.new_settings.pace |
          ((self.ch2_envelope_sweeper.new_settings.ascending as u8) << 3) |
          (self.ch2_envelope_sweeper.new_settings.initial_value << 4)
      }
      MemoryAddress::NR23 => self.ch2_wavelength_sweeper.new_settings.get_lower_wavelength_bits(),
      MemoryAddress::NR24 => {
        self.ch2_wavelength_sweeper.new_settings.get_upper_wavelength_bits() |
          ((self.ch2_length_timer.enabled as u8) << 6)
      }
      MemoryAddress::NR30 => if self.ch3_custom_wave_player.enabled { 0x80 } else { 0 },
      MemoryAddress::NR31 => self.ch3_length_timer.length(),
      MemoryAddress::NR32 => self.ch3_custom_wave_player.gain << 5,
      MemoryAddress::NR33 => self.ch3_custom_wave_player.get_lower_wavelength_bits(),
      MemoryAddress::NR34 => {
        self.ch3_custom_wave_player.get_upper_wavelength_bits() |
          ((self.ch3_length_timer.enabled as u8) << 6)
      },
      0xFF1F => 0,
      MemoryAddress::NR41 => self.nr41,
      MemoryAddress::NR42 => self.nr42,
      MemoryAddress::NR43 => self.nr43,
      MemoryAddress::NR44 => self.nr44,
      MemoryAddress::NR50 => self.master_volume,
      MemoryAddress::NR51 => self.mixing_control,
      MemoryAddress::NR52 => self.on_off_control,
      0xFF27..=0xFF2F => 0,
      0xFF30..=0xFF3F => self.waveform_ram[address as usize - 0xFF30],
      _ => panic!("AudioController can't read from address {}", address)
    }
  }

  fn write(&mut self, address: u16, value: u8) {
    match address {
      MemoryAddress::NR10 => {
        self.ch1_wavelength_sweeper.new_settings.shift = value & 0x7;
        self.ch1_wavelength_sweeper.new_settings.decrease = value.get_bit(3);
        self.ch1_wavelength_sweeper.new_settings.pace = (value >> 4) & 0x7;
      }
      MemoryAddress::NR11 => {
        let duty_cycle_bits = value >> 6;
        self.ch1_wavelength_sweeper.new_settings.duty_cycle = match duty_cycle_bits {
          0 => DutyCycle::Duty125,
          1 => DutyCycle::Duty250,
          2 => DutyCycle::Duty500,
          _ => DutyCycle::Duty750,
        };
        self.ch1_length_timer.set_length(value & 0x3F);
      }
      MemoryAddress::NR12 => {
        self.ch1_envelope_sweeper.new_settings.pace = value & 0x7;
        self.ch1_envelope_sweeper.new_settings.ascending = value.get_bit(3);
        self.ch1_envelope_sweeper.new_settings.initial_value = value >> 4;
      }
      MemoryAddress::NR13 => {
        self.ch1_wavelength_sweeper.new_settings.set_lower_wavelength_bits(value);
      }
      MemoryAddress::NR14 => {
        self.ch1_wavelength_sweeper.new_settings.set_upper_wavelength_bits(value);
        self.ch1_length_timer.enabled = value.get_bit(6);
        if value.get_bit(7) {
          self.trigger(Channel::CH1);
        }
      }
      0xFF15 => {}
      MemoryAddress::NR21 => {
        let duty_cycle_bits = value >> 6;
        self.ch2_wavelength_sweeper.new_settings.duty_cycle = match duty_cycle_bits {
          0 => DutyCycle::Duty125,
          1 => DutyCycle::Duty250,
          2 => DutyCycle::Duty500,
          _ => DutyCycle::Duty750,
        };
        self.ch2_length_timer.set_length(value & 0x3F);
      }
      MemoryAddress::NR22 => {
        self.ch2_envelope_sweeper.new_settings.pace = value & 0x7;
        self.ch2_envelope_sweeper.new_settings.ascending = value.get_bit(3);
        self.ch2_envelope_sweeper.new_settings.initial_value = value >> 4;
      }
      MemoryAddress::NR23 => {
        self.ch2_wavelength_sweeper.new_settings.set_lower_wavelength_bits(value);
      }
      MemoryAddress::NR24 => {
        self.ch2_wavelength_sweeper.new_settings.set_upper_wavelength_bits(value);
        self.ch2_length_timer.enabled = value.get_bit(6);
        if value.get_bit(7) {
          self.trigger(Channel::CH2);
        }
      }
      MemoryAddress::NR30 => {
        self.ch3_custom_wave_player.enabled = value.get_bit(7);
      }
      MemoryAddress::NR31 => {
        self.ch3_length_timer.set_length(value);
      }
      MemoryAddress::NR32 => {
        self.ch3_custom_wave_player.gain = (value >> 5) & 0x3;
      }
      MemoryAddress::NR33 => {
        self.ch3_custom_wave_player.set_lower_wavelength_bits(value);
      }
      MemoryAddress::NR34 => {
        self.ch3_custom_wave_player.set_upper_wavelength_bits(value);
        self.ch3_length_timer.enabled = value.get_bit(6);
        if value.get_bit(7) {
          self.trigger(Channel::CH3);
        }
      }
      0xFF1F => {}
      MemoryAddress::NR41 => self.nr41 = value,
      MemoryAddress::NR42 => self.nr42 = value,
      MemoryAddress::NR43 => self.nr43 = value,
      MemoryAddress::NR44 => self.nr44 = value,
      MemoryAddress::NR50 => self.master_volume = value,
      MemoryAddress::NR51 => self.mixing_control = value,
      MemoryAddress::NR52 => self.on_off_control = (self.on_off_control & 0x7F) | (value & 0x80),
      0xFF27..=0xFF2F => {}
      0xFF30..=0xFF3F => self.ch3_custom_wave_player.waveform[address as usize - 0xFF30] = value,
      _ => panic!("AudioController can't write to address {}", address)
    }
  }
}