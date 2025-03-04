use image::{ImageBuffer, Rgb, RgbImage};

use crate::{PhotometricInterpretation, PixelDataDefinition};

/// A color image that stores integer color values for each pixel. The colors
/// may be RGB or YBR.
///
#[derive(Clone, Debug, PartialEq)]
pub enum ColorImage {
  Uint8(ImageBuffer<Rgb<u8>, Vec<u8>>),
  Uint16(ImageBuffer<Rgb<u16>, Vec<u16>>),
  Uint32(ImageBuffer<Rgb<u32>, Vec<u32>>),
}

impl ColorImage {
  /// Returns the width in pixels of this color image.
  ///
  pub fn width(&self) -> u32 {
    match self {
      ColorImage::Uint8(data) => data.width(),
      ColorImage::Uint16(data) => data.width(),
      ColorImage::Uint32(data) => data.width(),
    }
  }

  /// Returns the height in pixels of this color image.
  ///
  pub fn height(&self) -> u32 {
    match self {
      ColorImage::Uint8(data) => data.height(),
      ColorImage::Uint16(data) => data.height(),
      ColorImage::Uint32(data) => data.height(),
    }
  }

  /// Returns the total number of pixels in this color image.
  ///
  pub fn pixel_count(&self) -> usize {
    self.width() as usize * self.height() as usize
  }

  /// Converts this color image to an RGB8 image.
  ///
  pub fn to_rgb_u8_image(self, definition: &PixelDataDefinition) -> RgbImage {
    if definition.bits_stored == 8 {
      if let ColorImage::Uint8(img) = self {
        return img;
      }
    }

    let mut rgb_pixels = Vec::with_capacity(self.pixel_count() * 3);

    let scale = 255.0 / (((1u64 << definition.bits_stored as u64) - 1) as f64);

    match &self {
      ColorImage::Uint8(data) => {
        for pixel in data.pixels() {
          rgb_pixels.push((pixel.0[0] as f64 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel.0[1] as f64 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel.0[2] as f64 * scale).min(255.0) as u8);
        }
      }

      ColorImage::Uint16(data) => {
        if let PhotometricInterpretation::PaletteColor { rgb_luts } =
          &definition.photometric_interpretation
        {
          let (red_lut, green_lut, blue_lut) = rgb_luts;

          for pixel in data.pixels() {
            let r = pixel.0[0] as f32 * red_lut.normalization_scale * 255.0;
            let g = pixel.0[1] as f32 * green_lut.normalization_scale * 255.0;
            let b = pixel.0[2] as f32 * blue_lut.normalization_scale * 255.0;

            rgb_pixels.push(r.min(255.0) as u8);
            rgb_pixels.push(g.min(255.0) as u8);
            rgb_pixels.push(b.min(255.0) as u8);
          }
        } else {
          for pixel in data.pixels() {
            rgb_pixels.push((pixel.0[0] as f64 * scale).min(255.0) as u8);
            rgb_pixels.push((pixel.0[1] as f64 * scale).min(255.0) as u8);
            rgb_pixels.push((pixel.0[2] as f64 * scale).min(255.0) as u8);
          }
        }
      }

      ColorImage::Uint32(data) => {
        for pixel in data.pixels() {
          rgb_pixels.push((pixel.0[0] as f64 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel.0[1] as f64 * scale).min(255.0) as u8);
          rgb_pixels.push((pixel.0[2] as f64 * scale).min(255.0) as u8);
        }
      }
    }

    RgbImage::from_raw(self.width(), self.height(), rgb_pixels).unwrap()
  }

  /// Converts this color image to an RGB F32 image where each value is in the
  /// range 0-1.
  ///
  pub fn to_rgb_f32_image(
    self,
    definition: &PixelDataDefinition,
  ) -> ImageBuffer<Rgb<f32>, Vec<f32>> {
    let mut rgb_pixels = Vec::with_capacity(self.pixel_count() * 3);

    let scale = 1.0 / (((1u64 << definition.bits_stored as u64) - 1) as f64);

    match &self {
      ColorImage::Uint8(data) => {
        for pixel in data.pixels() {
          rgb_pixels.push((pixel.0[0] as f64 * scale) as f32);
          rgb_pixels.push((pixel.0[1] as f64 * scale) as f32);
          rgb_pixels.push((pixel.0[2] as f64 * scale) as f32);
        }
      }

      ColorImage::Uint16(data) => {
        if let PhotometricInterpretation::PaletteColor { rgb_luts } =
          &definition.photometric_interpretation
        {
          let (red_lut, green_lut, blue_lut) = rgb_luts;

          for pixel in data.pixels() {
            rgb_pixels.push(pixel.0[0] as f32 * red_lut.normalization_scale);
            rgb_pixels.push(pixel.0[1] as f32 * green_lut.normalization_scale);
            rgb_pixels.push(pixel.0[2] as f32 * blue_lut.normalization_scale);
          }
        } else {
          for pixel in data.pixels() {
            rgb_pixels.push((pixel.0[0] as f64 * scale) as f32);
            rgb_pixels.push((pixel.0[1] as f64 * scale) as f32);
            rgb_pixels.push((pixel.0[2] as f64 * scale) as f32);
          }
        }
      }

      ColorImage::Uint32(data) => {
        for pixel in data.pixels() {
          rgb_pixels.push((pixel.0[0] as f64 * scale) as f32);
          rgb_pixels.push((pixel.0[1] as f64 * scale) as f32);
          rgb_pixels.push((pixel.0[2] as f64 * scale) as f32);
        }
      }
    }

    ImageBuffer::from_raw(self.width(), self.height(), rgb_pixels).unwrap()
  }

  /// Converts YBR colors in the range 0-1 to RGB colors in the range 0-1.
  ///
  pub fn convert_ybr_to_rgb(&mut self, definition: &PixelDataDefinition) {
    let scale = ((1u64 << definition.bits_stored as u64) - 1) as f64;
    let one_over_scale = 1.0 / scale;

    match self {
      ColorImage::Uint8(data) => {
        for pixel in data.pixels_mut() {
          let y = pixel.0[0] as f64 * one_over_scale;
          let cb = pixel.0[1] as f64 * one_over_scale;
          let cr = pixel.0[2] as f64 * one_over_scale;

          let [r, g, b] = ybr_to_rgb(y, cb, cr);

          pixel.0[0] = (r * scale).clamp(0.0, u8::MAX as f64) as u8;
          pixel.0[1] = (g * scale).clamp(0.0, u8::MAX as f64) as u8;
          pixel.0[2] = (b * scale).clamp(0.0, u8::MAX as f64) as u8;
        }
      }

      ColorImage::Uint16(data) => {
        for pixel in data.pixels_mut() {
          let y = pixel.0[0] as f64 * one_over_scale;
          let cb = pixel.0[1] as f64 * one_over_scale;
          let cr = pixel.0[2] as f64 * one_over_scale;

          let [r, g, b] = ybr_to_rgb(y, cb, cr);

          pixel.0[0] = (r * scale).clamp(0.0, u16::MAX as f64) as u16;
          pixel.0[1] = (g * scale).clamp(0.0, u16::MAX as f64) as u16;
          pixel.0[2] = (b * scale).clamp(0.0, u16::MAX as f64) as u16;
        }
      }

      ColorImage::Uint32(data) => {
        for pixel in data.pixels_mut() {
          let y = pixel.0[0] as f64 * one_over_scale;
          let cb = pixel.0[1] as f64 * one_over_scale;
          let cr = pixel.0[2] as f64 * one_over_scale;

          let [r, g, b] = ybr_to_rgb(y, cb, cr);

          pixel.0[0] = (r * scale).clamp(0.0, u32::MAX as f64) as u32;
          pixel.0[1] = (g * scale).clamp(0.0, u32::MAX as f64) as u32;
          pixel.0[2] = (b * scale).clamp(0.0, u32::MAX as f64) as u32;
        }
      }
    }
  }
}

fn ybr_to_rgb(y: f64, cb: f64, cr: f64) -> [f64; 3] {
  let r = y + 1.402 * (cr - 0.5);
  let g = y - 0.3441362862 * (cb - 0.5) - 0.7141362862 * (cr - 0.5);
  let b = y + 1.772 * (cb - 0.5);

  [r, g, b]
}
