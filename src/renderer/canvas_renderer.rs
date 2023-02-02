use std::iter;
use js_sys::Object;
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, console, HtmlCanvasElement, ImageData, Window};

use crate::renderer::renderer::{Color, Renderer};

pub struct CanvasRenderer {
  ctx: CanvasRenderingContext2d,
  background_color: Color,
  pixel_data: Vec<u8>,
  priorities: Vec<u8>,
  width: usize,
  height: usize
}

impl CanvasRenderer {
  pub fn new(canvas_id: &str, background_color: Color, width: usize, height: usize) -> Self {
    let canvas: HtmlCanvasElement = web_sys::window()
      .and_then(|window: Window| window.document())
      .and_then(|document| document.get_element_by_id(canvas_id))
      .map(|canvas_element| canvas_element.dyn_into::<HtmlCanvasElement>())
      .unwrap()
      .unwrap();
    let context: CanvasRenderingContext2d = canvas.get_context("2d")
      .map(|optional_context: Option<Object>| optional_context
        .and_then(|context: Object| context.dyn_into::<CanvasRenderingContext2d>().ok())
        .unwrap()
      )
      .unwrap();
    let mut renderer = CanvasRenderer {
      ctx: context,
      background_color,
      width,
      height,
      pixel_data: Vec::with_capacity(4 * width * height),
      priorities: Vec::with_capacity(width * height),
    };
    renderer.clear_canvas();
    renderer
  }

  fn clear_canvas(&mut self) {
    self.priorities.clear();
    let number_of_pixels = self.width * self.height;
    self.priorities.extend(iter::repeat(0).take(number_of_pixels));
    self.pixel_data.clear();
    for _ in 0..number_of_pixels {
      self.pixel_data.push(self.background_color.red);
      self.pixel_data.push(self.background_color.green);
      self.pixel_data.push(self.background_color.blue);
      self.pixel_data.push(if self.background_color.transparent { 0x00 } else { 0xFF });
    }
  }
}

impl Renderer for CanvasRenderer {

  fn draw_pixel(&mut self, x: usize, y: usize, color: Color, drawing_priority: u8) {
    if !color.transparent {
      let color_8_bit = color.to_rgb888();
      let pixel_offset = self.width * y + x;
      let channel_offset = 4 * pixel_offset;
      if drawing_priority == 0xFF || self.priorities[pixel_offset] <= drawing_priority {
        self.pixel_data[channel_offset] = color_8_bit.red;
        self.pixel_data[channel_offset + 1] = color_8_bit.green;
        self.pixel_data[channel_offset + 2] = color_8_bit.blue;
        self.pixel_data[channel_offset + 3] = 0xFF;
        self.priorities[pixel_offset] = if drawing_priority == 0xFF { self.priorities[pixel_offset] + 1 } else { drawing_priority };
      }
    }
  }

  fn flush(&mut self) {
    let image_data = ImageData::new_with_u8_clamped_array(Clamped(&self.pixel_data[..]), self.width as u32).unwrap();
    self.ctx.put_image_data(&image_data, 0.0, 0.0);
    self.clear_canvas();
  }
}