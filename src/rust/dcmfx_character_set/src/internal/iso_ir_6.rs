/// Decodes the next codepoint from the given ISO IR 6 bytes. This is the DICOM
/// default character set.
///
/// The bytes are actually decoded as if they were ISO IR 100, because ISO IR 6
/// is bit-compatible with ISO IR 100 and it is common to encounter DICOM data
/// sets that implicitly use the default character set and incorrectly assume it
/// will be ISO IR 100 rather than ISO IR 6.
///
pub fn decode_next_codepoint(bytes: &[u8]) -> Result<(char, &[u8]), ()> {
  crate::internal::iso_8859_1::decode_next_codepoint(bytes)
}
