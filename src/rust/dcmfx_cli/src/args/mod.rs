use dcmfx::core::DataElementTag;

pub mod default_transfer_syntax_arg;
pub mod file_list_arg;
pub mod jpeg_xl_decoder_arg;
pub mod photometric_interpretation_arg;
pub mod planar_configuration_arg;
pub mod standard_color_palette_arg;
pub mod transfer_syntax_arg;
pub mod transform_arg;

pub fn validate_data_element_tag(s: &str) -> Result<DataElementTag, String> {
  DataElementTag::from_hex_string(s)
    .map_err(|_| "Invalid data element tag".to_string())
}
