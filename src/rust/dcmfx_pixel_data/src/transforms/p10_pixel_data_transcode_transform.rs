#[cfg(not(feature = "std"))]
use alloc::{
  boxed::Box,
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use dcmfx_core::{
  DataElementTag, DataError, DataSetPath, DcmfxError, IodModule, RcByteSlice,
  TransferSyntax, ValueRepresentation, dictionary, transfer_syntax,
};
use dcmfx_p10::{
  P10CustomTypeTransform, P10CustomTypeTransformError, P10Error,
  P10FilterTransform, P10InsertTransform, P10Token,
};

use crate::{
  ColorImage, MonochromeImage, P10PixelDataFrameTransform,
  P10PixelDataFrameTransformError, PixelDataDecodeError, PixelDataEncodeConfig,
  PixelDataEncodeError, PixelDataFrame, PixelDataRenderer, decode, encode,
  iods::image_pixel_module::{BitsAllocated, ImagePixelModule},
};

/// This transform takes a stream of DICOM P10 tokens and converts it to use a
/// different transfer syntax. This is done by decoding and encoding frames of
/// pixel data as they stream through, as well as updating parts of the Image
/// Pixel Module that may need to be altered, such as the photometric
/// interpretation.
///
pub struct P10PixelDataTranscodeTransform {
  /// The transfer syntax of the incoming P10 token stream. This is set when the
  /// File Meta Information token is received.
  input_transfer_syntax: &'static TransferSyntax,

  /// The output transfer syntax being transcoded to.
  output_transfer_syntax: &'static TransferSyntax,

  /// Configuration for pixel data encoding.
  encode_config: PixelDataEncodeConfig,

  /// User-provided functions that are able to alter the Image Pixel Module as
  /// well as the image data for decoded frames prior to them being encoded into
  /// the output transfer syntax.
  image_data_functions: TranscodeImageDataFunctions,

  /// Transform that extracts a `PixelDataRenderer` from the token stream so
  /// that incoming frames can be decoded.
  pixel_data_renderer_transform: P10CustomTypeTransform<PixelDataRenderer>,

  /// Transform that extracts `PixelDataFrame`s from the token stream one by one
  /// as they become available.
  p10_pixel_data_frame_transform: P10PixelDataFrameTransform,

  /// Filter that removes the existing '(7FE0,0010) Pixel Data' data element
  /// from the main data set so it can be replaced with a transcoded one.
  pixel_data_remove_filter: P10FilterTransform,

  /// When transcoding to a transfer syntax that uses native pixel data, the
  /// number of bytes of pixel data still to be transcoded. This is reduced with
  /// every frame of pixel data that's emitted.
  native_pixel_data_bytes_remaining: u32,

  /// Tokens that are buffered while waiting for the Image Pixel Module tokens
  /// to be fully received. These tokens may then altered in the output to
  /// change the value of, e.g. the photometric interpretation.
  image_pixel_module_transform_token_buffer: Option<Vec<P10Token>>,

  /// The Image Pixel Module for images following decoding and alteration by
  /// the relevant image data function (if specified). This is set once the
  /// Image Pixel Module is received via the incoming stream of tokens.
  decoded_image_pixel_module: Option<ImagePixelModule>,

  /// The Image Pixel Module for the output pixel data following encoding.
  /// This is set once the Image Pixel Module is received via the incoming
  /// stream of tokens.
  output_image_pixel_module: Option<ImagePixelModule>,
}

/// Holds three user-provided functions that can alter the Image Pixel Module as
/// well as the image data for decoded frames prior to them being encoded into
/// the output transfer syntax.
///
/// These functions allow for arbitrary modifications to frame structure and
/// content during transcoding, e.g. changing color space, resizing, cropping,
/// etc.
///
/// The functions must work together to produce well-formed output, e.g. if
/// [`TranscodeImageDataFunctions::process_color_image()`] changes the color
/// space under certain circumstances then
/// [`TranscodeImageDataFunctions::process_image_pixel_module()`] must match its
/// behavior exactly with corresponding changes to the Image Pixel Module.
///
pub struct TranscodeImageDataFunctions {
  pub process_image_pixel_module: Box<FnProcessImagePixelModule>,
  pub process_monochrome_image: Box<FnProcessImage<MonochromeImage>>,
  pub process_color_image: Box<FnProcessImage<ColorImage>>,
}

pub type FnProcessImagePixelModule =
  dyn FnMut(
    &mut ImagePixelModule,
  ) -> Result<(), P10PixelDataTranscodeTransformError>;

pub type FnProcessImage<T> =
  dyn FnMut(
    &mut T,
    &ImagePixelModule,
  ) -> Result<(), P10PixelDataTranscodeTransformError>;

impl Default for TranscodeImageDataFunctions {
  fn default() -> Self {
    Self {
      process_image_pixel_module: Box::new(|_| Ok(())),
      process_monochrome_image: Box::new(|_, _| Ok(())),
      process_color_image: Box::new(|_, _| Ok(())),
    }
  }
}

impl P10PixelDataTranscodeTransform {
  /// Creates a new pixel data transcode transform for converting a stream of
  /// DICOM P10 tokens into a different transfer syntax.
  ///
  pub fn new(
    output_transfer_syntax: &'static TransferSyntax,
    encode_config: PixelDataEncodeConfig,
    image_data_functions: Option<TranscodeImageDataFunctions>,
  ) -> Self {
    Self {
      input_transfer_syntax: &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN,
      output_transfer_syntax,
      encode_config,
      image_data_functions: image_data_functions.unwrap_or_default(),
      pixel_data_renderer_transform:
        P10CustomTypeTransform::<PixelDataRenderer>::new_for_iod_module(),
      pixel_data_remove_filter: P10FilterTransform::new(Box::new(
        |tag, _vr, _length, path| {
          !path.is_root() || tag != dictionary::PIXEL_DATA.tag
        },
      )),
      p10_pixel_data_frame_transform: P10PixelDataFrameTransform::new(),
      native_pixel_data_bytes_remaining: 0,
      image_pixel_module_transform_token_buffer: None,
      decoded_image_pixel_module: None,
      output_image_pixel_module: None,
    }
  }

  /// Adds the next token to the P10 pixel data transcode transform and outputs
  /// an altered token stream containing the transcoded data set.
  ///
  pub fn add_token(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<P10Token>, P10PixelDataTranscodeTransformError> {
    // Store the input transfer syntax if one is specified in the File Meta
    // Information
    if let P10Token::FileMetaInformation { data_set } = token {
      if data_set.has(dictionary::TRANSFER_SYNTAX_UID.tag) {
        self.input_transfer_syntax = data_set
          .get_transfer_syntax()
          .map_err(P10PixelDataTranscodeTransformError::DataError)?;
      }
    }

    // Change the Transfer Syntax UID in the File Meta Information token
    let updated_token =
      if let P10Token::FileMetaInformation { data_set } = token {
        let mut data_set = data_set.clone();
        data_set
          .insert_string_value(
            &dictionary::TRANSFER_SYNTAX_UID,
            &[self.output_transfer_syntax.uid],
          )
          .unwrap();

        P10Token::FileMetaInformation { data_set }
      } else {
        token.clone()
      };

    // Pass the token through pixel data renderer transform used to decode
    // incoming frames of pixel data
    self
      .pixel_data_renderer_transform
      .add_token(token)
      .map_err(map_p10_custom_type_transform_error)?;

    // Pass the token through the pixel data frames transform, receiving any
    // raw frames of pixel data that are now available
    let input_frames = self
      .p10_pixel_data_frame_transform
      .add_token(token)
      .map_err(map_p10_pixel_data_frame_transform_error)?;

    // Perform any required alterations to the Image Pixel Module as it streams
    // through
    let transformed_tokens =
      self.add_token_to_image_pixel_module_transform(&updated_token)?;

    let mut output_tokens = vec![];

    // Remove the original '(7FE0,0010) Pixel Data' data element from the
    // incoming tokens. It will be replaced with the transcoded pixel data.
    for token in transformed_tokens {
      if self
        .pixel_data_remove_filter
        .add_token(&token)
        .map_err(P10PixelDataTranscodeTransformError::P10Error)?
      {
        output_tokens.push(token);
      }
    }

    // Iterate over the available pixel data frames and convert them into the
    // target transfer syntax, appending the resulting tokens to the vector
    for mut input_frame in input_frames {
      let encoded_frame = self.transcode_frame(&mut input_frame)?;

      let frame_index = input_frame.index().unwrap();

      if self.output_transfer_syntax.is_encapsulated {
        output_tokens.extend(
          self.encapsulated_pixel_data_tokens(frame_index, encoded_frame)?,
        );
      } else {
        output_tokens
          .extend(self.native_pixel_data_tokens(frame_index, encoded_frame)?);
      }
    }

    Ok(output_tokens)
  }

  /// Adds the next token to the process that transforms values, e.g. the
  /// photometric interpretation, in the Image Pixel Module as part of the
  /// transcoding transform.
  ///
  /// Tokens for the range of data elements used by the Image Pixel Module are
  /// buffered until all have been received, and then emitted with any
  /// required alterations made.
  ///
  fn add_token_to_image_pixel_module_transform(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<P10Token>, P10PixelDataTranscodeTransformError> {
    fn is_image_pixel_module_data_element(tag: DataElementTag) -> bool {
      tag >= dictionary::SAMPLES_PER_PIXEL.tag
        && tag <= ImagePixelModule::iod_module_highest_tag()
    }

    // Check if token buffering is currently in progress
    if let Some(token_buffer) =
      &mut self.image_pixel_module_transform_token_buffer
    {
      match token {
        P10Token::DataElementHeader { tag, path, .. }
        | P10Token::SequenceStart { tag, path, .. }
          if !is_image_pixel_module_data_element(*tag) && path.is_root() =>
        {
          // Make any required changes/updates to the tokens for the Image Pixel
          // Module
          let (
            mut tokens,
            decoded_image_pixel_module,
            output_image_pixel_module,
          ) = Self::transform_image_pixel_module_tokens(
            token_buffer,
            self.input_transfer_syntax,
            self.output_transfer_syntax,
            &self.encode_config,
            &mut self.image_data_functions,
          )?;

          self.decoded_image_pixel_module = Some(decoded_image_pixel_module);
          self.output_image_pixel_module = Some(output_image_pixel_module);

          tokens.push(token.clone());

          // Token buffering for the Image Pixel Module is now complete
          self.image_pixel_module_transform_token_buffer = None;

          Ok(tokens)
        }

        // This token is within the bounds of the Image Pixel Module, i.e.
        // buffering is still in progress, so accumulate it into the token
        // buffer
        _ => {
          token_buffer.push(token.clone());
          Ok(vec![])
        }
      }
    } else {
      // Token buffering isn't currently active, so start buffering if this is
      // the first data element for the Image Pixel Module
      match token {
        P10Token::DataElementHeader { tag, path, .. }
          if is_image_pixel_module_data_element(*tag) && path.is_root() =>
        {
          self.image_pixel_module_transform_token_buffer =
            Some(vec![token.clone()]);

          Ok(vec![])
        }

        _ => Ok(vec![token.clone()]),
      }
    }
  }

  /// Once the full set of tokens for the range of data elements used by the
  /// Image Pixel Module has been gathered, this function is called to apply
  /// any required updates to the content of the Image Pixel Module, e.g.
  /// a change to the Photometric Interpretation.
  ///
  fn transform_image_pixel_module_tokens(
    token_buffer: &[P10Token],
    input_transfer_syntax: &'static TransferSyntax,
    output_transfer_syntax: &'static TransferSyntax,
    encode_config: &PixelDataEncodeConfig,
    image_data_functions: &mut TranscodeImageDataFunctions,
  ) -> Result<
    (Vec<P10Token>, ImagePixelModule, ImagePixelModule),
    P10PixelDataTranscodeTransformError,
  > {
    // Construct the Image Pixel Module from the tokens in the buffer
    let mut image_pixel_module_transform =
      P10CustomTypeTransform::<ImagePixelModule>::new_for_iod_module();
    for token in token_buffer.iter() {
      image_pixel_module_transform
        .add_token(token)
        .map_err(map_p10_custom_type_transform_error)?;
    }

    image_pixel_module_transform
      .add_token(&P10Token::End)
      .map_err(map_p10_custom_type_transform_error)?;

    let mut image_pixel_module =
      image_pixel_module_transform.get_output().unwrap().clone();

    // Transcoding of frames that aren't a whole number of bytes isn't
    // supported. This is an extremely rare occurrence as it requires
    // multi-frame data, bits allocated of one, and a pixel count that's not a
    // multiple of eight.
    if image_pixel_module.bits_allocated() == BitsAllocated::One
      && image_pixel_module.pixel_count() % 8 != 0
    {
      return Err(P10PixelDataTranscodeTransformError::PixelDataDecodeError(
        PixelDataDecodeError::NotSupported {
          details: "Transcoding of unaligned bitmap data is not supported"
            .to_string(),
        },
      ));
    }

    // Determine the photometric interpretation after decoding
    let decoded_photometric_interpretation =
      decode::decode_photometric_interpretation(
        image_pixel_module.photometric_interpretation(),
        input_transfer_syntax,
      )
      .map_err(P10PixelDataTranscodeTransformError::PixelDataDecodeError)?;

    // Determine the output Image Pixel Module after encoding
    image_pixel_module.set_photometric_interpretation(
      decoded_photometric_interpretation.clone(),
    );

    // Pass through the relevant image data function
    (image_data_functions.process_image_pixel_module)(&mut image_pixel_module)?;

    let output_image_pixel_module = encode::encode_image_pixel_module(
      image_pixel_module.clone(),
      output_transfer_syntax,
      encode_config,
    )
    .map_err(P10PixelDataTranscodeTransformError::PixelDataEncodeError)?;

    // Create filter transform for excluding the previous Image Pixel Module
    let mut filter_transform =
      P10FilterTransform::new(Box::new(|tag, vr, length, path| {
        !ImagePixelModule::is_iod_module_data_element(tag, vr, length, path)
      }));

    // Create insert transform for adding the new Image Pixel Module
    let mut insert_transform = P10InsertTransform::new(
      output_image_pixel_module
        .to_data_set()
        .map_err(P10PixelDataTranscodeTransformError::DataError)?,
    );

    // Pass the buffered tokens through the above two transforms
    let mut transformed_tokens = Vec::with_capacity(token_buffer.len());
    for token in token_buffer {
      if filter_transform
        .add_token(token)
        .map_err(P10PixelDataTranscodeTransformError::P10Error)?
      {
        transformed_tokens.extend(
          insert_transform
            .add_token(token)
            .map_err(P10PixelDataTranscodeTransformError::P10Error)?,
        );
      }
    }

    insert_transform.flush(&mut transformed_tokens);

    Ok((
      transformed_tokens,
      image_pixel_module,
      output_image_pixel_module,
    ))
  }

  /// Transcodes a single [`PixelDataFrame`] into a frame for the target
  /// transfer syntax.
  ///
  fn transcode_frame(
    &mut self,
    input_frame: &mut PixelDataFrame,
  ) -> Result<RcByteSlice, P10PixelDataTranscodeTransformError> {
    let pixel_data_renderer =
      self.pixel_data_renderer_transform.get_output().unwrap();

    let output_frame = if pixel_data_renderer.image_pixel_module.is_color() {
      // Decode using the input Image Pixel Module
      let mut image = crate::decode::decode_color(
        input_frame,
        self.input_transfer_syntax,
        &pixel_data_renderer.image_pixel_module,
      )
      .map_err(P10PixelDataTranscodeTransformError::DataError)?;

      // Pass through the relevant image data function
      (self.image_data_functions.process_color_image)(
        &mut image,
        self.output_image_pixel_module.as_ref().unwrap(),
      )?;

      // Encode using the output Image Pixel Module
      crate::encode::encode_color(
        &image,
        self.output_image_pixel_module.as_ref().unwrap(),
        self.output_transfer_syntax,
        &self.encode_config,
      )
    } else {
      // Decode using the input Image Pixel Module
      let mut image = crate::decode::decode_monochrome(
        input_frame,
        self.input_transfer_syntax,
        &pixel_data_renderer.image_pixel_module,
      )
      .map_err(P10PixelDataTranscodeTransformError::DataError)?;

      // Pass through the relevant image data function
      (self.image_data_functions.process_monochrome_image)(
        &mut image,
        &pixel_data_renderer.image_pixel_module,
      )?;

      // Encode using the output Image Pixel Module
      crate::encode::encode_monochrome(
        &image,
        self.output_image_pixel_module.as_ref().unwrap(),
        self.output_transfer_syntax,
        &self.encode_config,
      )
    }
    .map_err(P10PixelDataTranscodeTransformError::PixelDataEncodeError)?;

    Ok(output_frame.to_bytes())
  }

  /// Returns the DICOM P10 tokens for the next transcoded frame of native pixel
  /// data.
  ///
  fn native_pixel_data_tokens(
    &mut self,
    frame_index: usize,
    encoded_frame: RcByteSlice,
  ) -> Result<Vec<P10Token>, P10PixelDataTranscodeTransformError> {
    // Get the Image Pixel Module. This is safe to unwrap because it must have
    // been fully received by the time pixel data is encountered.
    let image_pixel_module = self.decoded_image_pixel_module.as_ref().unwrap();

    let vr = if u8::from(image_pixel_module.bits_allocated()) <= 8 {
      ValueRepresentation::OtherByteString
    } else {
      ValueRepresentation::OtherWordString
    };

    let mut tokens = vec![];

    // On the first frame, calculate the total size of the native pixel data
    // that will be output and emit the data element header
    if frame_index == 0 {
      let pixel_data_value_length = (image_pixel_module.frame_size_in_bits()
        * self.p10_pixel_data_frame_transform.get_number_of_frames() as u64)
        .div_ceil(8);

      if pixel_data_value_length > u64::from(u32::MAX - 1) {
        return Err(P10PixelDataTranscodeTransformError::DataError(
          DataError::new_value_length_invalid(
            vr,
            pixel_data_value_length,
            "Native pixel data length exceeds 2^32 - 1".to_string(),
          )
          .with_path(&DataSetPath::new_with_data_element(
            dictionary::PIXEL_DATA.tag,
          )),
        ));
      }

      self.native_pixel_data_bytes_remaining = pixel_data_value_length as u32;

      tokens.push(P10Token::DataElementHeader {
        tag: dictionary::PIXEL_DATA.tag,
        vr,
        length: self.native_pixel_data_bytes_remaining,
        path: DataSetPath::new(),
      });
    }

    // Check that the encoded frame doesn't exceed the number of native pixel
    // data bytes left to emit. If this happens it likely indicates a bug in
    // either the decoding or encoding.
    if encoded_frame.len() > self.native_pixel_data_bytes_remaining as usize {
      return Err(P10PixelDataTranscodeTransformError::P10Error(
        P10Error::OtherError {
          error_type: "Transcoded pixel data too large".to_string(),
          details: format!(
            "Frame {} of length {} exceeds the remaining size of the native \
             pixel data {}",
            frame_index,
            encoded_frame.len(),
            self.native_pixel_data_bytes_remaining
          ),
        },
      ));
    }

    // Deduct this frame's size from the total native pixel data size
    self.native_pixel_data_bytes_remaining -= encoded_frame.len() as u32;

    tokens.push(P10Token::DataElementValueBytes {
      tag: dictionary::PIXEL_DATA.tag,
      vr,
      data: encoded_frame.clone(),
      bytes_remaining: self.native_pixel_data_bytes_remaining,
    });

    Ok(tokens)
  }

  /// Returns the DICOM P10 tokens for the next transcoded frame of encapsulated
  /// pixel data.
  ///
  fn encapsulated_pixel_data_tokens(
    &self,
    frame_index: usize,
    encoded_frame: RcByteSlice,
  ) -> Result<Vec<P10Token>, P10PixelDataTranscodeTransformError> {
    let mut tokens = vec![];

    // On the first frame, emit tokens for the start of the pixel data sequence
    // as well as an empty basic offset table
    if frame_index == 0 {
      tokens.push(P10Token::SequenceStart {
        tag: dictionary::PIXEL_DATA.tag,
        vr: ValueRepresentation::OtherByteString,
        path: DataSetPath::new_with_data_element(dictionary::PIXEL_DATA.tag),
      });
      tokens.push(P10Token::PixelDataItem {
        index: 0,
        length: 0,
      });
      tokens.push(P10Token::DataElementValueBytes {
        tag: dictionary::ITEM.tag,
        vr: ValueRepresentation::OtherByteString,
        data: RcByteSlice::empty(),
        bytes_remaining: 0,
      });
    }

    // Check the length of the encoded frame is a valid u32 length
    if encoded_frame.len() > (u32::MAX - 1) as usize {
      return Err(P10PixelDataTranscodeTransformError::DataError(
        DataError::new_value_length_invalid(
          ValueRepresentation::OtherByteString,
          encoded_frame.len() as u64,
          "Encoded frame length exceeds 2^32 - 1".to_string(),
        )
        .with_path(&DataSetPath::new_with_data_element(
          dictionary::PIXEL_DATA.tag,
        )),
      ));
    }

    let mut encoded_frame = encoded_frame.into_vec();
    if encoded_frame.len() & 1 == 1 {
      encoded_frame.push(0);
    }

    tokens.push(P10Token::PixelDataItem {
      index: frame_index,
      length: encoded_frame.len() as u32,
    });

    tokens.push(P10Token::DataElementValueBytes {
      tag: dictionary::ITEM.tag,
      vr: ValueRepresentation::OtherByteString,
      data: encoded_frame.into(),
      bytes_remaining: 0,
    });

    // On the last frame, emit a sequence delimiter
    if frame_index + 1
      == self.p10_pixel_data_frame_transform.get_number_of_frames()
    {
      tokens.push(P10Token::SequenceDelimiter {
        tag: dictionary::PIXEL_DATA.tag,
      })
    }

    Ok(tokens)
  }
}

/// An error that occurred in the process of transcoding pixel data.
///
#[derive(Clone, Debug, PartialEq)]
pub enum P10PixelDataTranscodeTransformError {
  DataError(DataError),
  P10Error(P10Error),
  PixelDataDecodeError(PixelDataDecodeError),
  PixelDataEncodeError(PixelDataEncodeError),
}

impl core::fmt::Display for P10PixelDataTranscodeTransformError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::DataError(e) => e.fmt(f),
      Self::P10Error(e) => e.fmt(f),
      Self::PixelDataDecodeError(e) => e.fmt(f),
      Self::PixelDataEncodeError(e) => e.fmt(f),
    }
  }
}

impl DcmfxError for P10PixelDataTranscodeTransformError {
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    match self {
      Self::DataError(e) => e.to_lines(task_description),
      Self::P10Error(e) => e.to_lines(task_description),
      Self::PixelDataDecodeError(e) => e.to_lines(task_description),
      Self::PixelDataEncodeError(e) => e.to_lines(task_description),
    }
  }
}

fn map_p10_custom_type_transform_error(
  e: P10CustomTypeTransformError,
) -> P10PixelDataTranscodeTransformError {
  match e {
    P10CustomTypeTransformError::DataError(e) => {
      P10PixelDataTranscodeTransformError::DataError(e)
    }
    P10CustomTypeTransformError::P10Error(e) => {
      P10PixelDataTranscodeTransformError::P10Error(e)
    }
  }
}

fn map_p10_pixel_data_frame_transform_error(
  e: P10PixelDataFrameTransformError,
) -> P10PixelDataTranscodeTransformError {
  match e {
    P10PixelDataFrameTransformError::DataError(e) => {
      P10PixelDataTranscodeTransformError::DataError(e)
    }
    P10PixelDataFrameTransformError::P10Error(e) => {
      P10PixelDataTranscodeTransformError::P10Error(e)
    }
  }
}
