//// Defines the type used to describe errors that can occur when reading and
//// writing DICOM P10 data.

import dcmfx_core/data_set_path.{type DataSetPath}
import dcmfx_p10/p10_token.{type P10Token}
import file_streams/file_stream_error
import gleam/int
import gleam/io
import gleam/list
import gleam/string

/// An error that occurred when reading or writing DICOM P10 data.
///
pub type P10Error {
  /// This error occurs when a DICOM P10 read or write context is supplied data
  /// that specifies a DICOM transfer syntax that isn't supported.
  TransferSyntaxNotSupported(transfer_syntax_uid: String)

  /// This error occurs when a DICOM P10 read context is supplied data that
  /// contains a *'(0008,0005) SpecificCharacterSet'* data element that is
  /// invalid and unable to be decoded.
  ///
  /// This error will never occur on valid DICOM P10 data because all character
  /// sets defined by the DICOM standard are supported.
  SpecificCharacterSetInvalid(specific_character_set: String, details: String)

  /// This error occurs when a DICOM P10 read context requires more data to be
  /// added to it before the next token can be read.
  DataRequired(when: String)

  /// This error occurs when a DICOM P10 read context reaches the end of its
  /// data while reading the next token, and no more data is able to be added.
  /// This means the provided data is malformed or truncated.
  DataEndedUnexpectedly(when: String, path: DataSetPath, offset: Int)

  /// This error occurs when a DICOM P10 read context is unable to read the next
  /// DICOM P10 token because the supplied data is invalid, and also when a
  /// DICOM P10 write context is unable to serialize a token written to it.
  DataInvalid(when: String, details: String, path: DataSetPath, offset: Int)

  /// This error occurs when one of the configured maximums for a DICOM P10 read
  /// context is exceeded during reading of the supplied data. These maximums
  /// are used to control memory usage when reading.
  MaximumExceeded(details: String, path: DataSetPath, offset: Int)

  /// This error occurs when a stream of `P10Token`s is being ingested and a
  /// token is received that is invalid at the current location in the token
  /// stream. E.g. a `DataElementValueBytes` token that does not follow a
  /// `DataElementHeader`.
  TokenStreamInvalid(when: String, details: String, token: P10Token)

  /// This error occurs when bytes are written to a DICOM P10 read context after
  /// its final bytes have already been written.
  WriteAfterCompletion

  /// This error occurs when there is an error with an underlying file stream.
  FileStreamError(when: String, error: file_stream_error.FileStreamError)

  /// A fallback/general-purpose error for cases not covered by the other error
  /// variants.
  OtherError(error_type: String, details: String)
}

/// Returns lines of text that describe a DICOM P10 error in a human-readable
/// format.
pub fn print(error: P10Error, task_description: String) -> Nil {
  io.println_error("")
  io.println_error("-----")

  to_lines(error, task_description)
  |> list.each(io.println_error)

  io.println_error("")
}

/// Returns the name of the error as a human-readable string.
///
pub fn name(error: P10Error) -> String {
  case error {
    TransferSyntaxNotSupported(..) -> "Transfer syntax not supported"
    SpecificCharacterSetInvalid(..) -> "Specific character set invalid"
    DataRequired(..) -> "Data required"
    DataEndedUnexpectedly(..) -> "Unexpected end of data"
    DataInvalid(..) -> "Invalid data"
    MaximumExceeded(..) -> "Maximum exceeded"
    TokenStreamInvalid(..) -> "P10 token stream invalid"
    WriteAfterCompletion(..) -> "Write after completion"
    FileStreamError(..) -> "File stream I/O failure"
    OtherError(error_type: error_type, ..) -> error_type
  }
}

/// Returns the `details`` field of the error, if one exists.
///
pub fn details(error: P10Error) -> String {
  case error {
    TransferSyntaxNotSupported(..) -> ""
    SpecificCharacterSetInvalid(details:, ..) -> details
    DataRequired(..) -> ""
    DataEndedUnexpectedly(..) -> ""
    DataInvalid(details:, ..) -> details
    MaximumExceeded(details:, ..) -> details
    TokenStreamInvalid(details:, ..) -> details
    WriteAfterCompletion(..) -> ""
    FileStreamError(..) -> ""
    OtherError(details:, ..) -> details
  }
}

/// Returns lines of text that describe a DICOM P10 error in a human-readable
/// format.
pub fn to_lines(error: P10Error, task_description: String) -> List(String) {
  let lines = ["", "DICOM P10 error " <> task_description]

  // Add the name of the error
  let lines = ["  Error: " <> name(error), ..lines]

  // Add the 'when' if it is present
  let lines = case error {
    DataRequired(when: when)
    | DataEndedUnexpectedly(when: when, ..)
    | DataInvalid(when: when, ..)
    | TokenStreamInvalid(when: when, ..)
    | FileStreamError(when: when, ..) -> ["  When: " <> when, ..lines]
    _ -> lines
  }

  // Add the details if present
  let lines = case error {
    TransferSyntaxNotSupported(uid) -> [
      "  Transfer syntax UID: " <> uid,
      ..lines
    ]

    SpecificCharacterSetInvalid(charset, details) ->
      list.flatten([
        case details {
          "" -> []
          _ -> ["  Details: " <> details]
        },
        ["  Specific character set: " <> charset],
        lines,
      ])

    TokenStreamInvalid(details:, token:, ..) -> [
      "  Token: " <> p10_token.to_string(token),
      "  Details: " <> details,
      ..lines
    ]

    FileStreamError(error: error, ..) -> [
      "  Details: " <> string.inspect(error),
      ..lines
    ]

    DataInvalid(details: details, ..)
    | MaximumExceeded(details: details, ..)
    | OtherError(details: details, ..) -> ["  Details: " <> details, ..lines]

    _ -> lines
  }

  // Add the path and offset if present
  let lines = case error {
    DataEndedUnexpectedly(path:, offset: offset, ..)
    | DataInvalid(path:, offset:, ..)
    | MaximumExceeded(path:, offset: offset, ..) -> [
      "  Offset: 0x" <> int.to_base16(offset),
      "  Path: " <> data_set_path.to_detailed_string(path),
      ..lines
    ]
    _ -> lines
  }

  lines |> list.reverse
}
