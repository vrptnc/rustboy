use std::{iter, vec};

use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData, Window};

use crate::renderer::renderer::{Color, Renderer, RenderTarget};

pub struct CanvasRenderer {
  context: CanvasRenderingContext2d,
  background_color: Color,
  color_buffer: Vec<u8>,
  depth_buffer: Vec<u8>,
  width: usize,
  height: usize,
}

pub struct CompositeCanvasRenderer {
  main_renderer: Option<CanvasRenderer>,
  tile_atlas_renderer: Option<CanvasRenderer>,
  object_atlas_renderer: Option<CanvasRenderer>,
}

impl CompositeCanvasRenderer {
  pub fn new() -> Self {
    CompositeCanvasRenderer {
      main_renderer: Some(CanvasRenderer::new("main-canvas", Color::white())),
      tile_atlas_renderer: None,
      object_atlas_renderer: None,
    }
  }
}

impl Renderer for CompositeCanvasRenderer {
  fn set_render_target_enabled(&mut self, target: RenderTarget, enabled: bool) {
    match target {
      RenderTarget::Main => {
        if self.main_renderer.is_none() && enabled {
          self.main_renderer = Some(CanvasRenderer::new("main-canvas", Color::white()));
        } else if !enabled {
          self.main_renderer = None;
        }
      }
      RenderTarget::ObjectAtlas => {
        if self.object_atlas_renderer.is_none() && enabled {
          self.object_atlas_renderer = Some(CanvasRenderer::new("object-atlas-canvas", Color::transparent()));
        } else if !enabled {
          self.object_atlas_renderer = None;
        }
      }
      RenderTarget::TileAtlas => {
        if self.tile_atlas_renderer.is_none() && enabled {
          self.tile_atlas_renderer = Some(CanvasRenderer::new("tile-atlas-canvas", Color::transparent()));
        } else if !enabled {
          self.tile_atlas_renderer = None;
        }
      }
    }
  }

  fn draw_pixel(&mut self, x: usize, y: usize, z: u8, color: Color, target: RenderTarget) {
    match target {
      RenderTarget::Main => if let Some(ref mut renderer) = self.main_renderer {
        renderer.draw_pixel(x, y, z, color);
      },
      RenderTarget::ObjectAtlas => if let Some(ref mut renderer) = self.object_atlas_renderer {
        renderer.draw_pixel(x, y, z, color);
      },
      RenderTarget::TileAtlas => if let Some(ref mut renderer) = self.tile_atlas_renderer {
        renderer.draw_pixel(x, y, z, color);
      }
    }
  }

  fn flush(&mut self) {
    if let Some(ref mut renderer) = self.main_renderer {
      renderer.flush();
    }
    if let Some(ref mut renderer) = self.object_atlas_renderer {
      renderer.flush();
    }
    if let Some(ref mut renderer) = self.tile_atlas_renderer {
      renderer.flush();
    }
  }
}

impl CanvasRenderer {
  pub fn find_canvas(canvas_id: &str) -> HtmlCanvasElement {
    web_sys::window()
      .and_then(|window: Window| window.document())
      .and_then(|document| document.get_element_by_id(canvas_id))
      .and_then(|element| element.dyn_into::<HtmlCanvasElement>().ok())
      .unwrap()
  }

  pub fn get_context(canvas: HtmlCanvasElement) -> CanvasRenderingContext2d {
    canvas.get_context("2d").unwrap()
      .and_then(|object| object.dyn_into::<CanvasRenderingContext2d>().ok())
      .unwrap()
  }

  pub fn new(canvas_id: &str, background_color: Color) -> Self {
    let canvas = CanvasRenderer::find_canvas(canvas_id);
    let width = canvas.width() as usize;
    let height = canvas.height() as usize;
    let context = CanvasRenderer::get_context(canvas);
    let mut renderer = CanvasRenderer {
      context,
      background_color,
      width,
      height,
      color_buffer: vec![0; 4 * width * height],
      depth_buffer: vec![0; width * height],
    };
    renderer.clear_canvas();
    renderer
  }

  fn clear_canvas(&mut self) {
    let number_of_pixels = self.width * self.height;
    self.depth_buffer.clear();
    self.depth_buffer.extend(iter::repeat(0).take(number_of_pixels));
    self.color_buffer.clear();
    for _ in 0..number_of_pixels {
      self.color_buffer.push(self.background_color.red);
      self.color_buffer.push(self.background_color.green);
      self.color_buffer.push(self.background_color.blue);
      self.color_buffer.push(if self.background_color.transparent { 0x00 } else { 0xFF });
    }
  }

  fn draw_pixel(&mut self, x: usize, y: usize, z: u8, color: Color) {
    if !color.transparent {
      let color_8_bit = color.to_rgb888();
      let pixel_offset = self.width * y + x;
      let channel_offset = 4 * pixel_offset;
      if z == 0xFF || self.depth_buffer[pixel_offset] <= z {
        self.color_buffer[channel_offset] = color_8_bit.red;
        self.color_buffer[channel_offset + 1] = color_8_bit.green;
        self.color_buffer[channel_offset + 2] = color_8_bit.blue;
        self.color_buffer[channel_offset + 3] = 0xFF;
        self.depth_buffer[pixel_offset] = if z == 0xFF { self.depth_buffer[pixel_offset] + 1 } else { z };
      }
    }
  }

  fn flush(&mut self) {
    let image_data = ImageData::new_with_u8_clamped_array(Clamped(&self.color_buffer[..]), self.width as u32).unwrap();
    self.context.put_image_data(&image_data, 0.0, 0.0);
    self.clear_canvas();
  }
}