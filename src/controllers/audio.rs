use mockall::automock;

use crate::memory::memory::{Memory, MemoryAddress};
use crate::util::bit_util::BitUtil;

#[automock]
pub trait AudioController {}

pub struct AudioControllerImpl {
  ch1_sweep: u8,
  ch1_length_timer: u8,
  ch1_envelope: u8,
  ch1_freq_data_low: u8,
  ch1_freq_data_and_trigger: u8,
  ch2_length_timer: u8,
  ch2_envelope: u8,
  ch2_freq_data_low: u8,
  ch2_freq_data_and_trigger: u8,
  ch3_on_off: u8,
  ch3_length_timer: u8,
  ch3_volume: u8,
  ch3_freq_data_low: u8,
  ch3_freq_data_and_trigger: u8,
  ch4_length_timer: u8,
  ch4_envelope: u8,
  ch4_counter: u8,
  ch4_trigger: u8,
  master_volume: u8,
  mixing_control: u8,
  on_off_control: u8,
  waveform_ram: [u8; 16],
}

impl AudioControllerImpl {
  pub fn new() -> Self {
    AudioControllerImpl {
      ch1_sweep: 0,
      ch1_length_timer: 0,
      ch1_envelope: 0,
      ch1_freq_data_low: 0,
      ch1_freq_data_and_trigger: 0,
      ch2_length_timer: 0,
      ch2_envelope: 0,
      ch2_freq_data_low: 0,
      ch2_freq_data_and_trigger: 0,
      ch3_on_off: 0,
      ch3_length_timer: 0,
      ch3_volume: 0,
      ch3_freq_data_low: 0,
      ch3_freq_data_and_trigger: 0,
      ch4_length_timer: 0,
      ch4_envelope: 0,
      ch4_counter: 0,
      ch4_trigger: 0,
      master_volume: 0,
      mixing_control: 0,
      on_off_control: 0,
      waveform_ram: [0;16]
    }
  }
}

impl AudioController for AudioControllerImpl {}

impl Memory for AudioControllerImpl {
  fn read(&self, address: u16) -> u8 {
    match address {
      MemoryAddress::NR10 => self.ch1_sweep,
      MemoryAddress::NR11 => self.ch1_length_timer,
      MemoryAddress::NR12 => self.ch1_envelope,
      MemoryAddress::NR13 => self.ch1_freq_data_low,
      MemoryAddress::NR14 => self.ch1_freq_data_and_trigger,
      0xFF15 => 0,
      MemoryAddress::NR21 => self.ch2_length_timer,
      MemoryAddress::NR22 => self.ch2_envelope,
      MemoryAddress::NR23 => self.ch2_freq_data_low,
      MemoryAddress::NR24 => self.ch2_freq_data_and_trigger,
      MemoryAddress::NR30 => self.ch3_on_off,
      MemoryAddress::NR31 => self.ch3_length_timer,
      MemoryAddress::NR32 => self.ch3_volume,
      MemoryAddress::NR33 => self.ch3_freq_data_low,
      MemoryAddress::NR34 => self.ch3_freq_data_and_trigger,
      0xFF1F => 0,
      MemoryAddress::NR41 => self.ch4_length_timer,
      MemoryAddress::NR42 => self.ch4_envelope,
      MemoryAddress::NR43 => self.ch4_counter,
      MemoryAddress::NR44 => self.ch4_trigger,
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
      MemoryAddress::NR10 => self.ch1_sweep = value,
      MemoryAddress::NR11 => self.ch1_length_timer = value,
      MemoryAddress::NR12 => self.ch1_envelope = value,
      MemoryAddress::NR13 => self.ch1_freq_data_low = value,
      MemoryAddress::NR14 => self.ch1_freq_data_and_trigger = value,
      0xFF15 => {},
      MemoryAddress::NR21 => self.ch2_length_timer = value,
      MemoryAddress::NR22 => self.ch2_envelope = value,
      MemoryAddress::NR23 => self.ch2_freq_data_low = value,
      MemoryAddress::NR24 => self.ch2_freq_data_and_trigger = value,
      MemoryAddress::NR30 => self.ch3_on_off = value,
      MemoryAddress::NR31 => self.ch3_length_timer = value,
      MemoryAddress::NR32 => self.ch3_volume = value,
      MemoryAddress::NR33 => self.ch3_freq_data_low = value,
      MemoryAddress::NR34 => self.ch3_freq_data_and_trigger = value,
      0xFF1F => {},
      MemoryAddress::NR41 => self.ch4_length_timer = value,
      MemoryAddress::NR42 => self.ch4_envelope = value,
      MemoryAddress::NR43 => self.ch4_counter = value,
      MemoryAddress::NR44 => self.ch4_trigger = value,
      MemoryAddress::NR50 => self.master_volume = value,
      MemoryAddress::NR51 => self.mixing_control = value,
      MemoryAddress::NR52 => self.on_off_control = (self.on_off_control & 0x7F) | (value & 0x80),
      0xFF27..=0xFF2F => {},
      0xFF30..=0xFF3F => self.waveform_ram[address as usize - 0xFF30] = value,
      _ => panic!("AudioController can't write to address {}", address)
    }
  }
}