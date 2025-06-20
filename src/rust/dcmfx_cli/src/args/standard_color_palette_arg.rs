use clap::ValueEnum;
use dcmfx::pixel_data::{StandardColorPalette, standard_color_palettes};

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum StandardColorPaletteArg {
  HotIron,
  Pet,
  HotMetalBlue,
  Pet20Step,
  Spring,
  Summer,
  Fall,
  Winter,
}

impl StandardColorPaletteArg {
  pub fn color_palette(&self) -> &'static StandardColorPalette {
    match self {
      StandardColorPaletteArg::HotIron => &standard_color_palettes::HOT_IRON,
      StandardColorPaletteArg::Pet => &standard_color_palettes::PET,
      StandardColorPaletteArg::HotMetalBlue => {
        &standard_color_palettes::HOT_METAL_BLUE
      }
      StandardColorPaletteArg::Pet20Step => {
        &standard_color_palettes::PET_20_STEP
      }
      StandardColorPaletteArg::Spring => &standard_color_palettes::SPRING,
      StandardColorPaletteArg::Summer => &standard_color_palettes::SUMMER,
      StandardColorPaletteArg::Fall => &standard_color_palettes::FALL,
      StandardColorPaletteArg::Winter => &standard_color_palettes::WINTER,
    }
  }
}
