use std::cmp;

use js_sys::{Array, Number};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AnalyserNode, AudioBuffer, AudioBufferSourceNode, AudioContext, AudioParamMap, AudioWorkletNode, AudioWorkletNodeOptions, BiquadFilterNode, BiquadFilterType, CanvasRenderingContext2d, console, GainNode, window};

use crate::audio::audio_driver::{AudioDriver, Channel, CustomWaveOptions, DutyCycle, NoiseOptions, PulseOptions, StereoChannel};
use crate::renderer::canvas_renderer::CanvasRenderer;

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
  // ch1_analyser_node: AnalyserNode,
  // ch2_analyser_node: AnalyserNode,
  // ch3_analyser_node: AnalyserNode,
  // ch4_analyser_node: AnalyserNode,
  ch1_canvas_context: CanvasRenderingContext2d,
  ch2_canvas_context: CanvasRenderingContext2d,
  ch3_canvas_context: CanvasRenderingContext2d,
  ch4_canvas_context: CanvasRenderingContext2d,
  mixer_node: GainNode,
  high_pass_filter_node: BiquadFilterNode,
}

impl WebAudioDriver {
  const FFT_SIZE: u32 = 128u32;

  pub fn draw(&mut self) {
    // self.draw_channel(Channel::CH1);
    // self.draw_channel(Channel::CH2);
    // self.draw_channel(Channel::CH3);
    // self.draw_channel(Channel::CH4);
  }

  // fn draw_channel(&mut self, channel: Channel) {
  //   let (context, analyser) = match channel {
  //     Channel::CH1 => (&self.ch1_canvas_context, &self.ch1_analyser_node),
  //     Channel::CH2 => (&self.ch2_canvas_context, &self.ch2_analyser_node),
  //     Channel::CH3 => (&self.ch3_canvas_context, &self.ch3_analyser_node),
  //     Channel::CH4 => (&self.ch4_canvas_context, &self.ch4_analyser_node),
  //   };
  //   let mut audio_data = [0.0f32;WebAudioDriver::FFT_SIZE as usize];
  //   analyser.get_float_time_domain_data(&mut audio_data);
  //   context.clear_rect(0.0, 0.0, 200.0, 100.0);
  //   context.begin_path();
  //   for sample_index in 0..WebAudioDriver::FFT_SIZE as usize {
  //     let y = 50.0 + 50.0 * (audio_data[sample_index] as f64);
  //     if sample_index == 0 {
  //       context.move_to(0.0, y);
  //     } else {
  //       context.line_to(sample_index as f64 * 200.0 / (WebAudioDriver::FFT_SIZE as f64), y);
  //     }
  //     context.stroke();
  //   }
  // }

  pub fn new(context: AudioContext) -> WebAudioDriver {
    let destination = context.destination();
    let mut worklet_node_options = AudioWorkletNodeOptions::new();
    let output_channel_counts = Array::new();
    output_channel_counts.push(&JsValue::from(2));
    worklet_node_options.number_of_inputs(0);
    worklet_node_options.number_of_outputs(1);
    worklet_node_options.output_channel_count(&output_channel_counts);
    let ch1_node = AudioWorkletNode::new_with_options(&context, "pwm-processor", &worklet_node_options).unwrap();
    let ch2_node = AudioWorkletNode::new_with_options(&context, "pwm-processor", &worklet_node_options).unwrap();
    let ch3_node = AudioWorkletNode::new_with_options(&context, "waveform-processor", &worklet_node_options).unwrap();
    let ch4_node = AudioWorkletNode::new_with_options(&context, "white-noise-processor", &worklet_node_options).unwrap();
    let ch1_gain_node = context.create_gain().unwrap();
    ch1_gain_node.gain().set_value(0.25);
    let ch2_gain_node = context.create_gain().unwrap();
    ch2_gain_node.gain().set_value(0.25);
    let ch3_gain_node = context.create_gain().unwrap();
    ch3_gain_node.gain().set_value(0.25);
    let ch4_gain_node = context.create_gain().unwrap();
    ch4_gain_node.gain().set_value(0.25);
    // let ch1_analyser_node = context.create_analyser().unwrap();
    // ch1_analyser_node.set_fft_size(WebAudioDriver::FFT_SIZE);
    // let ch2_analyser_node = context.create_analyser().unwrap();
    // ch2_analyser_node.set_fft_size(WebAudioDriver::FFT_SIZE);
    // let ch3_analyser_node = context.create_analyser().unwrap();
    // ch3_analyser_node.set_fft_size(WebAudioDriver::FFT_SIZE);
    // let ch4_analyser_node = context.create_analyser().unwrap();
    // ch4_analyser_node.set_fft_size(WebAudioDriver::FFT_SIZE);
    let black_style = JsValue::from_str("rgb(0, 0, 0");
    let ch1_canvas_context = CanvasRenderer::get_context("ch1-canvas");
    ch1_canvas_context.set_line_width(2.0);
    ch1_canvas_context.set_stroke_style(&black_style);
    let ch2_canvas_context = CanvasRenderer::get_context("ch2-canvas");
    ch2_canvas_context.set_line_width(2.0);
    ch2_canvas_context.set_stroke_style(&black_style);
    let ch3_canvas_context = CanvasRenderer::get_context("ch3-canvas");
    ch3_canvas_context.set_line_width(2.0);
    ch3_canvas_context.set_stroke_style(&black_style);
    let ch4_canvas_context = CanvasRenderer::get_context("ch4-canvas");
    ch4_canvas_context.set_line_width(2.0);
    ch4_canvas_context.set_stroke_style(&black_style);

    let mixer_node = context.create_gain().unwrap();
    let high_pass_filter_node = context.create_biquad_filter().unwrap();
    high_pass_filter_node.set_type(BiquadFilterType::Highpass);
    high_pass_filter_node.frequency().set_value(20.0f32);

    ch1_node.connect_with_audio_node(&ch1_gain_node);
    // ch1_analyser_node.connect_with_audio_node(&ch1_gain_node);
    ch1_gain_node.connect_with_audio_node(&mixer_node);
    ch2_node.connect_with_audio_node(&ch2_gain_node);
    // ch2_analyser_node.connect_with_audio_node(&ch2_gain_node);
    ch2_gain_node.connect_with_audio_node(&mixer_node);
    ch3_node.connect_with_audio_node(&ch3_gain_node);
    // ch3_analyser_node.connect_with_audio_node(&ch3_gain_node);
    ch3_gain_node.connect_with_audio_node(&mixer_node);
    ch4_node.connect_with_audio_node(&ch4_gain_node);
    // ch4_analyser_node.connect_with_audio_node(&ch4_gain_node);
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
      // ch1_analyser_node,
      // ch2_analyser_node,
      // ch3_analyser_node,
      // ch4_analyser_node,
      ch1_canvas_context,
      ch2_canvas_context,
      ch3_canvas_context,
      ch4_canvas_context,
      mixer_node,
      high_pass_filter_node,
    }
  }

  fn get_parameters(&self, channel: Channel) -> AudioParamMap {
    match channel {
      Channel::CH1 => self.ch1_node.parameters().unwrap(),
      Channel::CH2 => self.ch2_node.parameters().unwrap(),
      Channel::CH3 => self.ch3_node.parameters().unwrap(),
      Channel::CH4 => self.ch4_node.parameters().unwrap(),
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
    let parameters = self.get_parameters(channel);
    (0..8usize).for_each(|index| {
      let offset = 2 * index;
      let value = (wave_options.data[offset] as u32 +
        ((wave_options.data[offset + 1] as u32) << 8)) as f32;
      parameters.get(format!("data{}", index).as_str()).unwrap().set_value(value);
    });
    let trigger_param = parameters.get("trigger").unwrap();
    trigger_param.set_value(1.0);
  }

  fn play_noise(&mut self, channel: Channel, noise_options: NoiseOptions) {
    let parameters = self.get_parameters(channel);
    let frequency_param = parameters.get("frequency").unwrap();
    frequency_param.set_value(44100.0f32.min(noise_options.frequency));
    let width_param = parameters.get("width").unwrap();
    width_param.set_value(if noise_options.short { 1.0 } else { 0.0 });
    let trigger_param = parameters.get("trigger").unwrap();
    trigger_param.set_value(1.0);
  }

  fn stop(&mut self, channel: Channel) {
    let parameters = self.get_parameters(channel);
    let trigger_param = parameters.get("trigger").unwrap();
    trigger_param.set_value(0.0);
  }

  fn set_gain(&mut self, channel: Channel, gain: f32) {
    let parameters = self.get_parameters(channel);
    let gain_param = parameters.get("gain").unwrap();
    gain_param.set_value(gain);
  }

  fn set_stereo_gain(&mut self, channel: Channel, stereo_channel: StereoChannel, gain: f32) {
    let parameters = self.get_parameters(channel);
    let stereo_gain_param = match stereo_channel {
      StereoChannel::Left => parameters.get("leftChannelGain").unwrap(),
      StereoChannel::Right => parameters.get("rightChannelGain").unwrap()
    };
    stereo_gain_param.set_value(gain);
  }

  fn set_frequency(&mut self, channel: Channel, frequency: f32) {
    let parameters = self.get_parameters(channel);
    let frequency_param = parameters.get("frequency").unwrap();
    frequency_param.set_value(frequency);
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