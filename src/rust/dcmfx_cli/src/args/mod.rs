use dcmfx::{
  core::{DataElementTag, DataSet},
  json::DataSetJsonExtensions,
};

pub mod decoder_args;
pub mod frame_selection_arg;
pub mod input_args;
pub mod photometric_interpretation_arg;
pub mod planar_configuration_arg;
pub mod standard_color_palette_arg;
pub mod transfer_syntax_arg;
pub mod transform_arg;

pub fn parse_data_element_tag(s: &str) -> Result<DataElementTag, String> {
  DataElementTag::from_hex_string(s)
    .map_err(|_| "Invalid data element tag".to_string())
}

pub fn parse_dicom_json_data_set(s: &str) -> Result<DataSet, String> {
  DataSet::from_json(s).map_err(|e| e.to_string())
}
