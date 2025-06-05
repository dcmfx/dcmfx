use crate::iods::image_pixel_module::{
  ImagePixelModule, PhotometricInterpretation, PlanarConfiguration,
};

/// Returns the Image Pixel Module resulting from encoding into JPEG 2000.
///
pub fn encode_image_pixel_module(
  mut image_pixel_module: ImagePixelModule,
  quality: Option<u8>,
) -> Result<ImagePixelModule, ()> {
  match image_pixel_module.photometric_interpretation() {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. }
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull => (),

    // YBR_ICT is only permitted for lossy encodes
    PhotometricInterpretation::YbrIct => {
      if quality.is_none() {
        return Err(());
      }
    }

    // YBR_RCT and PALETTE_COLOR are only permitted for lossless encodes
    PhotometricInterpretation::YbrRct
    | PhotometricInterpretation::PaletteColor { .. } => {
      if quality.is_some() {
        return Err(());
      }
    }

    _ => return Err(()),
  };

  image_pixel_module.set_planar_configuration(PlanarConfiguration::Interleaved);

  Ok(image_pixel_module)
}
