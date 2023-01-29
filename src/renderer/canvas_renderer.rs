use js_sys::Object;
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, console, HtmlCanvasElement, ImageData, Window};

use crate::renderer::renderer::{Color, Renderer};

pub struct CanvasRenderer {
  ctx: CanvasRenderingContext2d,
  pixel_data: [u8; 92160],
  priorities: [u8; 23040],
}

impl CanvasRenderer {
  pub fn new() -> Self {
    let canvas: HtmlCanvasElement = web_sys::window()
      .and_then(|window: Window| window.document())
      .and_then(|document| document.get_element_by_id("main-canvas"))
      .map(|canvas_element| canvas_element.dyn_into::<HtmlCanvasElement>())
      .unwrap()
      .unwrap();
    let context: CanvasRenderingContext2d = canvas.get_context("2d")
      .map(|optional_context: Option<Object>| optional_context
        .and_then(|context: Object| context.dyn_into::<CanvasRenderingContext2d>().ok())
        .unwrap()
      )
      .unwrap();
    CanvasRenderer {
      ctx: context,
      pixel_data: [0xFF; 92160],
      priorities: [0; 23040],
    }
  }
}

impl Renderer for CanvasRenderer {

  fn draw_pixel(&mut self, x: u8, y: u8, color: Color, drawing_priority: u8) {
    if !color.transparent {
      let color_8_bit = color.to_rgb888();
      let pixel_offset = 160 * y as usize + x as usize;
      let channel_offset = 4 * (pixel_offset);
      if self.priorities[pixel_offset] <= drawing_priority {
        self.pixel_data[channel_offset] = color_8_bit.red;
        self.pixel_data[channel_offset + 1] = color_8_bit.green;
        self.pixel_data[channel_offset + 2] = color_8_bit.blue;
        self.pixel_data[channel_offset + 3] = 0xFF;
        self.priorities[pixel_offset] = drawing_priority;
      }
    }
  }

  fn flush(&mut self) {
    let image_data = ImageData::new_with_u8_clamped_array(Clamped(&self.pixel_data[..]), 160).unwrap();
    self.ctx.put_image_data(&image_data, 0.0, 0.0);
    self.priorities = [0;23040];
    self.pixel_data = [0xFF;92160];
  }
}