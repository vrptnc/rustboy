use std::cmp;

use js_sys::Array;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext, AudioParamMap, AudioWorkletNode, BiquadFilterNode, BiquadFilterType, console, GainNode, window};

use crate::audio::audio_driver::{AudioDriver, Channel, CustomWaveOptions, DutyCycle, NoiseOptions, PulseOptions};

pub struct WebAudioDriver {
  context: AudioContext,
  ch1_node: AudioWorkletNode,
  ch2_node: AudioWorkletNode,
  ch3_node: AudioWorkletNode,
  ch4_node: AudioWorkletNode,
  ch1_gain_node: GainNode,
  ch2_gain_node: GainNode,
  ch3_gain_node: GainNode,
  ch4_gain_node: GainNode,
  mixer_node: GainNode,
  high_pass_filter_node: BiquadFilterNode
}

impl WebAudioDriver {
  pub fn new(context: AudioContext) -> WebAudioDriver {
    let destination = context.destination();
    let ch1_node = AudioWorkletNode::new(&context, "pwm-processor").unwrap();
    let ch2_node = AudioWorkletNode::new(&context, "pwm-processor").unwrap();
    let ch3_node = AudioWorkletNode::new(&context, "waveform-processor").unwrap();
    let ch4_node = AudioWorkletNode::new(&context, "white-noise-processor").unwrap();
    let ch1_gain_node = context.create_gain().unwrap();
    ch1_gain_node.gain().set_value(0.25);
    let ch2_gain_node = context.create_gain().unwrap();
    ch2_gain_node.gain().set_value(0.25);
    let ch3_gain_node = context.create_gain().unwrap();
    ch3_gain_node.gain().set_value(0.25);
    let ch4_gain_node = context.create_gain().unwrap();
    ch4_gain_node.gain().set_value(0.25);

    let mixer_node = context.create_gain().unwrap();
    let high_pass_filter_node = context.create_biquad_filter().unwrap();
    high_pass_filter_node.set_type(BiquadFilterType::Highpass);
    high_pass_filter_node.frequency().set_value(20.0f32);

    ch1_node.connect_with_audio_node(&ch1_gain_node);
    ch1_gain_node.connect_with_audio_node(&mixer_node);
    ch2_node.connect_with_audio_node(&ch2_gain_node);
    ch2_gain_node.connect_with_audio_node(&mixer_node);
    ch3_node.connect_with_audio_node(&ch3_gain_node);
    ch3_gain_node.connect_with_audio_node(&mixer_node);
    ch4_node.connect_with_audio_node(&ch4_gain_node);
    ch4_gain_node.connect_with_audio_node(&mixer_node);
    mixer_node.connect_with_audio_node(&high_pass_filter_node);
    high_pass_filter_node.connect_with_audio_node(&destination);


    WebAudioDriver {
      context,
      ch1_node,
      ch2_node,
      ch3_node,
      ch4_node,
      ch1_gain_node,
      ch2_gain_node,
      ch3_gain_node,
      ch4_gain_node,
      mixer_node,
      high_pass_filter_node
    }
  }
}

impl AudioDriver for WebAudioDriver {
  fn play_pulse(&mut self, channel: Channel, pulse_options: PulseOptions) {
    let parameters: AudioParamMap = match channel {
      Channel::CH1 => self.ch1_node.parameters().unwrap(),
      Channel::CH2 => self.ch2_node.parameters().unwrap(),
      _ => panic!("Can't play pulse on channel other than 1 or 2")
    };
    let frequency_param = parameters.get("frequency").unwrap();
    frequency_param.set_value(pulse_options.frequency);
    let duty_cycle_param = parameters.get("dutyCycle").unwrap();
    duty_cycle_param.set_value(pulse_options.duty_cycle);
    let trigger_param = parameters.get("trigger").unwrap();
    trigger_param.set_value(1.0);
  }

  fn play_custom_wave(&mut self, channel: Channel, wave_options: CustomWaveOptions) {
    match channel {
      Channel::CH3 => {
        let parameters = self.ch3_node.parameters().unwrap();
        (0..8usize).for_each(|index| {
          let offset = 2 * index;
          let value = (wave_options.data[offset] as u32 +
          ((wave_options.data[offset + 1] as u32) << 8)) as f32;
          parameters.get(format!("data{}", index).as_str()).unwrap().set_value(value);
        });
        let trigger_param = parameters.get("trigger").unwrap();
        trigger_param.set_value(1.0);
      }
      _ => panic!("Can't play pulse on channel other than 3")
    }
  }

  fn play_noise(&mut self, channel: Channel, noise_options: NoiseOptions) {
    match channel {
      Channel::CH4 => {
        let parameters = self.ch4_node.parameters().unwrap();
        let frequency_param = parameters.get("frequency").unwrap();
        frequency_param.set_value(44100.0f32.min(noise_options.frequency));
        let width_param = parameters.get("width").unwrap();
        width_param.set_value(if noise_options.short { 1.0 } else { 0.0 });
        let trigger_param = parameters.get("trigger").unwrap();
        trigger_param.set_value(1.0);
      }
      _ => panic!("Can't play noise on channel other than 4")
    }
  }

  fn stop(&mut self, channel: Channel) {
    let parameters = match channel {
      Channel::CH1 => {
        self.ch1_node.parameters().unwrap()
      }
      Channel::CH2 => {
        self.ch2_node.parameters().unwrap()
      }
      Channel::CH3 => {
        self.ch3_node.parameters().unwrap()
      }
      Channel::CH4 => {
        self.ch4_node.parameters().unwrap()
      }
    };
    let trigger_param = parameters.get("trigger").unwrap();
    trigger_param.set_value(0.0);
  }

  fn set_gain(&mut self, channel: Channel, gain: f32) {
    let parameters = match channel {
      Channel::CH1 => {
        self.ch1_node.parameters().unwrap()
      }
      Channel::CH2 => {
        self.ch2_node.parameters().unwrap()
      }
      Channel::CH3 => {
        self.ch3_node.parameters().unwrap()
      }
      Channel::CH4 => {
        self.ch4_node.parameters().unwrap()
      }
    };
    let gain_param = parameters.get("gain").unwrap();
    gain_param.set_value(gain);
  }

  fn set_frequency(&mut self, channel: Channel, frequency: f32) {
    match channel {
      Channel::CH3 => {
        let parameters = self.ch3_node.parameters().unwrap();
        let frequency_param = parameters.get("frequency").unwrap();
        frequency_param.set_value(frequency);
      }
      _ => panic!("Can only change frequency for channel 3")
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