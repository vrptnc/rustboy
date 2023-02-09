use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext, BiquadFilterNode, BiquadFilterType, console, GainNode, window};

use crate::audio::audio_driver::{AudioDriver, Channel, CustomWaveOptions, DutyCycle, PulseOptions};

pub struct WebAudioDriver {
  context: AudioContext,
  buffer_125: AudioBuffer,
  buffer_250: AudioBuffer,
  buffer_500: AudioBuffer,
  ch1_node: Option<AudioBufferSourceNode>,
  ch2_node: Option<AudioBufferSourceNode>,
  ch3_buffer: AudioBuffer,
  ch3_node: Option<AudioBufferSourceNode>,
  ch1_gain_node: GainNode,
  ch2_gain_node: GainNode,
  ch3_gain_node: GainNode,
  ch4_gain_node: GainNode,
  mixer_node: GainNode,
  low_pass_filter: BiquadFilterNode
}

impl WebAudioDriver {
  pub fn new() -> WebAudioDriver {
    let context = AudioContext::new().unwrap();
    let destination = context.destination();

    let buffer_125 = context.create_buffer(1, 3000, 3000.0).unwrap();
    let buffer_250 = context.create_buffer(1, 3000, 3000.0).unwrap();
    let buffer_500 = context.create_buffer(1, 3000, 3000.0).unwrap();
    let mut data: [f32; 3000] = [0.0; 3000];
    (0..375usize).for_each(|index| data[index] = -1.0);
    buffer_125.copy_to_channel(&data[..], 0);
    (375..750usize).for_each(|index| data[index] = -1.0);
    buffer_250.copy_to_channel(&data[..], 0);
    (750..1500usize).for_each(|index| data[index] = -1.0);
    buffer_500.copy_to_channel(&data[..], 0);
    let ch3_buffer = context.create_buffer(1, 3100, 3100.0).unwrap();

    let mixer_node = context.create_gain().unwrap();
    let ch1_gain_node = context.create_gain().unwrap();
    ch1_gain_node.gain().set_value(0.25);
    let ch2_gain_node = context.create_gain().unwrap();
    ch2_gain_node.gain().set_value(0.25);
    let ch3_gain_node = context.create_gain().unwrap();
    let ch4_gain_node = context.create_gain().unwrap();
    let low_pass_filter = context.create_biquad_filter().unwrap();
    low_pass_filter.set_type(BiquadFilterType::Lowpass);
    low_pass_filter.frequency().set_value(15000.0f32);


    ch1_gain_node.connect_with_audio_node(&mixer_node);
    ch2_gain_node.connect_with_audio_node(&mixer_node);
    ch3_gain_node.connect_with_audio_node(&mixer_node);
    ch4_gain_node.connect_with_audio_node(&mixer_node);
    mixer_node.connect_with_audio_node(&low_pass_filter);
    low_pass_filter.connect_with_audio_node(&destination);


    WebAudioDriver {
      context,
      buffer_125,
      buffer_250,
      buffer_500,
      ch3_buffer,
      ch1_node: None,
      ch2_node: None,
      ch3_node: None,
      ch1_gain_node,
      ch2_gain_node,
      ch3_gain_node,
      ch4_gain_node,
      mixer_node,
      low_pass_filter
    }
  }
}

impl AudioDriver for WebAudioDriver {
  fn play_pulse(&mut self, channel: Channel, pulse_options: PulseOptions) {
    self.stop(channel);
    let buffer = match pulse_options.duty_cycle {
      DutyCycle::Duty125 => &self.buffer_125,
      DutyCycle::Duty250 => &self.buffer_250,
      DutyCycle::Duty500 => &self.buffer_500,
      DutyCycle::Duty750 => &self.buffer_250
    };
    let new_node = self.context.create_buffer_source().unwrap();
    new_node.set_loop(true);
    new_node.set_buffer(Some(buffer));
    new_node.playback_rate().set_value(pulse_options.frequency);
    match channel {
      Channel::CH1 => {
        new_node.connect_with_audio_node(&self.ch1_gain_node);
        new_node.start();
        self.ch1_node = Some(new_node);
      }
      Channel::CH2 => {
        new_node.connect_with_audio_node(&self.ch2_gain_node);
        new_node.start();
        self.ch2_node = Some(new_node);
      }
      _ => panic!("Can't play pulse on channel other than 1 or 2")
    }
  }

  fn play_custom_wave(&mut self, channel: Channel, wave_options: CustomWaveOptions) {
    self.stop(channel);
    match channel {
      Channel::CH3 => {
        let mut data: [f32;3100] = [0.0;3100];
        (0..3100usize).for_each(|index| {
          let lower = wave_options.data[index / 100];
          let upper = wave_options.data[(index / 100) + 1];
          let average = ((index % 100) as f32) * (upper - lower) / 100.0;
          data[index] = lower + average;
        });
        self.ch3_buffer.copy_to_channel(&data[..], 0);
        let new_node = self.context.create_buffer_source().unwrap();
        new_node.set_loop(true);
        new_node.set_buffer(Some(&self.ch3_buffer));
        new_node.playback_rate().set_value(wave_options.frequency);
        new_node.connect_with_audio_node(&self.ch3_gain_node);
        new_node.start();
        self.ch3_node = Some(new_node);
        self.ch3_gain_node.gain().set_value(0.25 * wave_options.gain);
      }
      _ => panic!("Can't play pulse on channel other than 3")
    }
  }

  fn stop(&mut self, channel: Channel) {
    match channel {
      Channel::CH1 => {
        if let Some(node) = self.ch1_node.as_ref() {
          node.stop();
          node.disconnect();
        }
      }
      Channel::CH2 => {
        if let Some(node) = self.ch2_node.as_ref() {
          node.stop();
          node.disconnect();
        }
      }
      Channel::CH3 => {
        if let Some(node) = self.ch3_node.as_ref() {
          node.stop();
          node.disconnect();
        }
      }
      Channel::CH4 => {}
    }
  }

  fn set_gain(&mut self, channel: Channel, gain: f32) {
    match channel {
      Channel::CH1 => {
        self.ch1_gain_node.gain().set_value(0.25 * gain);
      }
      Channel::CH2 => {
        self.ch2_gain_node.gain().set_value(0.25 * gain);
      }
      Channel::CH3 => {
        self.ch3_gain_node.gain().set_value(gain);
      }
      Channel::CH4 => {
        self.ch4_gain_node.gain().set_value(gain);
      }
    }
  }

  fn mute_all(&mut self) {
    todo!()
  }

  fn unmute_all(&mut self) {
    todo!()
  }

  fn set_master_volume(&mut self, value: u8) {
    todo!()
  }
}