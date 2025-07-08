use dcmfx::{
  core::{TransferSyntax, transfer_syntax},
  p10::P10ReadConfig,
};

pub const HELP: &str = "The transfer syntax to assume for DICOM P10 data that \
  doesn't specify '(0002,0010) Transfer Syntax UID' in its File Meta \
  Information, or that doesn't have any File Meta Information.\n\
  \n\
  Defaults to '1.2.840.10008.1.2', i.e. 'Implicit VR Little Endian'";

pub fn validate(s: &str) -> Result<&'static TransferSyntax, String> {
  TransferSyntax::from_uid(s)
    .map_err(|_| "Unrecognized transfer syntax UID".to_string())
}

pub fn get_read_config(
  default_transfer_syntax: &Option<&'static TransferSyntax>,
) -> P10ReadConfig {
  P10ReadConfig::default().default_transfer_syntax(
    default_transfer_syntax
      .unwrap_or(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN),
  )
}
