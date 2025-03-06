use crate::PixelDataDefinition;

/// Converts u8 YBR data to RGB.
///
pub fn convert_u8(data: &mut [u8], definition: &PixelDataDefinition) {
  let scale = ((1u64 << definition.bits_stored as u64) - 1) as f64;
  let one_over_scale = 1.0 / scale;

  for i in 0..(data.len() / 3) {
    let y = data[i * 3] as f64 * one_over_scale;
    let cb = data[i * 3 + 1] as f64 * one_over_scale;
    let cr = data[i * 3 + 2] as f64 * one_over_scale;

    let [r, g, b] = ybr_to_rgb(y, cb, cr);

    data[i * 3] = (r * scale).clamp(0.0, u8::MAX as f64) as u8;
    data[i * 3 + 1] = (g * scale).clamp(0.0, u8::MAX as f64) as u8;
    data[i * 3 + 2] = (b * scale).clamp(0.0, u8::MAX as f64) as u8;
  }
}

/// Converts u16 YBR data to RGB.
///
pub fn convert_u16(data: &mut [u16], definition: &PixelDataDefinition) {
  let scale = ((1u64 << definition.bits_stored as u64) - 1) as f64;
  let one_over_scale = 1.0 / scale;

  for i in 0..(data.len() / 3) {
    let y = data[i * 3] as f64 * one_over_scale;
    let cb = data[i * 3 + 1] as f64 * one_over_scale;
    let cr = data[i * 3 + 2] as f64 * one_over_scale;

    let [r, g, b] = ybr_to_rgb(y, cb, cr);

    data[i * 3] = (r * scale).clamp(0.0, u16::MAX as f64) as u16;
    data[i * 3 + 1] = (g * scale).clamp(0.0, u16::MAX as f64) as u16;
    data[i * 3 + 2] = (b * scale).clamp(0.0, u16::MAX as f64) as u16;
  }
}

/// Converts u32 YBR data to RGB.
///
pub fn convert_u32(data: &mut [u32], definition: &PixelDataDefinition) {
  let scale = ((1u64 << definition.bits_stored as u64) - 1) as f64;
  let one_over_scale = 1.0 / scale;

  for i in 0..(data.len() / 3) {
    let y = data[i * 3] as f64 * one_over_scale;
    let cb = data[i * 3 + 1] as f64 * one_over_scale;
    let cr = data[i * 3 + 2] as f64 * one_over_scale;

    let [r, g, b] = ybr_to_rgb(y, cb, cr);

    data[i * 3] = (r * scale).clamp(0.0, u32::MAX as f64) as u32;
    data[i * 3 + 1] = (g * scale).clamp(0.0, u32::MAX as f64) as u32;
    data[i * 3 + 2] = (b * scale).clamp(0.0, u32::MAX as f64) as u32;
  }
}

fn ybr_to_rgb(y: f64, cb: f64, cr: f64) -> [f64; 3] {
  let r = y + 1.402 * (cr - 0.5);
  let g = y - 0.3441362862 * (cb - 0.5) - 0.7141362862 * (cr - 0.5);
  let b = y + 1.772 * (cb - 0.5);

  [r, g, b]
}
