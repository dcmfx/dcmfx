import gleam/int

/// Configuration used when reading DICOM P10 data.
///
pub type P10ReadConfig {
  P10ReadConfig(
    max_token_size: Int,
    max_string_size: Int,
    max_sequence_depth: Int,
    require_dicm_prefix: Bool,
    require_ordered_data_elements: Bool,
  )
}

/// Returns the default read config.
///
pub fn new() -> P10ReadConfig {
  P10ReadConfig(
    max_token_size: 0xFFFFFFFE,
    max_string_size: 0xFFFFFFFE,
    max_sequence_depth: 10_000,
    require_dicm_prefix: False,
    require_ordered_data_elements: True,
  )
}

/// The maximum size in bytes of a DICOM P10 token emitted by a read context.
/// This can be used to control memory usage during a streaming read, and must
/// be a multiple of 8.
///
/// The maximum token size is relevant to two specific tokens:
///
/// 1. `FileMetaInformation`, where it sets the maximum size in bytes of the
///    File Meta Information, as specified by the File Meta Information Group
///    Length value. If this size is exceeded an error will occur when reading
///    the DICOM P10 data.
///
/// 2. `DataElementValueBytes`, where it sets the maximum size in bytes of its
///    `data` (with the exception of non-UTF-8 string data, see
///    `max_string_size()` for further details). Data element values with a
///    length exceeding this size will be split across multiple
///    `DataElementValueBytes` tokens.
///
/// By default there is no limit on the maximum token size, that is, each data
/// element will have its value bytes emitted in exactly one
/// `DataElementValueBytes` token.
///
pub fn max_token_size(config: P10ReadConfig, value: Int) -> P10ReadConfig {
  P10ReadConfig(..config, max_token_size: { value / 8 } * 8)
}

/// The maximum size in bytes of non-UTF-8 strings that can be read by a read
/// context. This can be used to control memory usage during a streaming read.
///
/// The maximum string size is relevant to data elements containing string
/// values that are not encoded in UTF-8. Such string data is converted to UTF-8
/// by the read context, which requires that the whole string value be read into
/// memory.
///
/// Specifically:
///
/// 1. The maximum string size sets a hard upper limit on the size of a
///    non-UTF-8 string value that can be read. Data element values containing
///    non-UTF-8 string data larger that the maximum string size will result in
///    an error. Because of this, the maximum size should not be set too low.
///
/// 2. The maximum string size can be set larger than the maximum token size to
///    allow more leniency in regard to the size of string data that can be
///    parsed, while keeping token sizes smaller for other common cases such as
///    image data.
///
/// By default there is no limit on the maximum string size.
///
pub fn max_string_size(config: P10ReadConfig, value: Int) -> P10ReadConfig {
  P10ReadConfig(
    ..config,
    max_string_size: int.max(config.max_token_size, value),
  )
}

/// ### `max_sequence_depth: Int`
///
/// The maximum sequence depth that can be read by a read context. This can be
/// used to control memory usage during a streaming read, as well as to reject
/// malformed or malicious DICOM P10 data.
///
/// By default the maximum sequence depth is set to ten thousand, i.e. no
/// meaningful maximum is enforced.
///
pub fn max_sequence_depth(config: P10ReadConfig, value: Int) -> P10ReadConfig {
  P10ReadConfig(..config, max_sequence_depth: int.max(0, value))
}

/// Whether to require input data have 'DICM' at bytes 128-132. This is required
/// for well-formed DICOM P10 data, but it may be absent in some cases. If this
/// is set to `False` then such data will be readable.
///
/// By default the 'DICM' prefix at bytes 128-132 is not required.
///
pub fn require_dicm_prefix(config: P10ReadConfig, value: Bool) -> P10ReadConfig {
  P10ReadConfig(..config, require_dicm_prefix: value)
}

/// Whether to error if data elements are not in ascending order in the DICOM
/// P10 data. Such data is malformed but is still able to read, however doing so
/// can potentially lead to incorrect results. For example:
///
/// 1. If the *'(0008,0005) Specific Character Set'* data element appears after
///    data elements that use an encoded string VR, they will be decoded using
///    the wrong character set.
///
/// 2. If a '(gggg,00xx) Private Creator' data element appears after the data
///    elements it defines the private creator for, those data elements will
///    all be read with a VR of UN (when the transfer syntax is 'Implicit VR
///    Little Endian').
///
/// By default this requirement is enforced.
///
pub fn require_ordered_data_elements(
  config: P10ReadConfig,
  value: Bool,
) -> P10ReadConfig {
  P10ReadConfig(..config, require_ordered_data_elements: value)
}
