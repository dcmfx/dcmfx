#[cfg(not(feature = "std"))]
use alloc::{
  boxed::Box,
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use dcmfx_core::{
  DataElementValue, DataError, DataSet, DataSetPath, DcmfxError, IodModule, Rc,
  RcByteSlice, TransferSyntax, ValueRepresentation, dictionary,
  transfer_syntax,
};
use dcmfx_p10::{
  P10CustomTypeTransform, P10CustomTypeTransformError, P10Error,
  P10FilterTransform, P10InsertTransform, P10Token,
};

use crate::{
  ColorImage, MonochromeImage, P10PixelDataFrameTransform,
  P10PixelDataFrameTransformError, PixelDataDecodeConfig, PixelDataDecodeError,
  PixelDataEncodeConfig, PixelDataEncodeError, PixelDataFrame, decode, encode,
  iods::image_pixel_module::{
    ImagePixelModule, PhotometricInterpretation, PlanarConfiguration,
  },
  transforms::CropRect,
};

/// This transform takes a stream of DICOM P10 tokens and transcodes its pixel
/// data into a different transfer syntax. This is done by decoding and encoding
/// frames of pixel data as they stream through, as well as updating parts of
/// the Image Pixel Module that may need to be changed, such as the photometric
/// interpretation.
///
/// Other pixel data modifications can also be applied, e.g. cropping.
///
/// If the input stream of DICOM P10 tokens does not contain the data elements
/// for an Image Pixel Module then the pixel data transcode will not alter the
/// token stream.
///
pub struct P10PixelDataTranscodeTransform {
  /// The transfer syntax of the incoming P10 token stream. This is set when the
  /// File Meta Information token is received.
  input_transfer_syntax: &'static TransferSyntax,

  /// The output transfer syntax being transcoded to.
  output_transfer_syntax: &'static TransferSyntax,

  /// Configuration for pixel data decoding.
  decode_config: PixelDataDecodeConfig,

  /// Configuration for pixel data encoding.
  encode_config: PixelDataEncodeConfig,

  /// User-provided functions that are able to alter the Image Pixel Module as
  /// well as the image data for decoded frames prior to them being encoded into
  /// the output transfer syntax.
  image_data_functions: TranscodeImageDataFunctions,

  /// Tokens buffered from the start of the transcode while waiting for the
  /// Image Pixel Module's tokens to be fully received. These tokens may then be
  /// altered before being output in order to change the value of the transfer
  /// syntax, photometric interpretation, planar configuration, and so on.
  initial_token_buffer: Option<Vec<P10Token>>,

  /// Transform that extracts the input Image Pixel Module from the token stream
  /// so that incoming frames can be decoded.
  input_image_pixel_module_transform: P10CustomTypeTransform<ImagePixelModule>,

  /// The Image Pixel Module for images following decoding and alteration by
  /// the relevant image data function. This is set once the Image Pixel Module
  /// is received via the incoming stream of tokens.
  decoded_image_pixel_module: Option<ImagePixelModule>,

  /// The Image Pixel Module for the output pixel data following encoding.
  /// This is set once the Image Pixel Module is received via the incoming
  /// stream of tokens.
  output_image_pixel_module: Option<ImagePixelModule>,

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

  /// If the output transfer syntax is lossy, this is an insert transform that
  /// inserts the '(0028,2110) Lossy Image Compression' data element.
  lossy_image_compression_insert_transform: Option<P10InsertTransform>,
}

/// Holds user-provided functions that can alter the Image Pixel Module and
/// mutate Pixel Data for individual frames prior to them being encoded into
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
/// Note that when transcoding from 'JPEG Baseline 8-bit' to 'JPEG XL JPEG
/// Recompression' none of these image data functions are called.
///
pub struct TranscodeImageDataFunctions {
  pub is_encode_decode_cycle_required: Box<IsEncodeDecodeCycleRequiredFn>,
  pub process_image_pixel_module: Box<ProcessImagePixelModuleFn>,
  pub process_monochrome_image: Box<ProcessImageFn<MonochromeImage>>,
  pub process_color_image: Box<ProcessImageFn<ColorImage>>,
}

pub type IsEncodeDecodeCycleRequiredFn = dyn Fn(&ImagePixelModule) -> bool;

pub type ProcessImagePixelModuleFn =
  dyn Fn(
    &mut ImagePixelModule,
  ) -> Result<(), P10PixelDataTranscodeTransformError>;

pub type ProcessImageFn<T> =
  dyn Fn(
    &mut T,
    &ImagePixelModule,
  ) -> Result<(), P10PixelDataTranscodeTransformError>;

impl P10PixelDataTranscodeTransform {
  /// Creates a new pixel data transcode transform for converting a stream of
  /// DICOM P10 tokens into a different transfer syntax.
  ///
  pub fn new(
    output_transfer_syntax: &'static TransferSyntax,
    decode_config: PixelDataDecodeConfig,
    encode_config: PixelDataEncodeConfig,
    image_data_functions: Option<TranscodeImageDataFunctions>,
  ) -> Self {
    Self {
      input_transfer_syntax: &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN,
      output_transfer_syntax,
      decode_config,
      encode_config,
      image_data_functions: image_data_functions.unwrap_or_default(),
      initial_token_buffer: Some(vec![]),
      input_image_pixel_module_transform:
        P10CustomTypeTransform::new_for_iod_module(),
      decoded_image_pixel_module: None,
      output_image_pixel_module: None,
      pixel_data_remove_filter: P10FilterTransform::new(Box::new(
        |tag, _vr, _length, path| {
          !path.is_root() || tag != dictionary::PIXEL_DATA.tag
        },
      )),
      p10_pixel_data_frame_transform: P10PixelDataFrameTransform::new(),
      native_pixel_data_bytes_remaining: 0,
      lossy_image_compression_insert_transform:
        Self::lossy_image_compression_insert_transform(output_transfer_syntax),
    }
  }

  /// Returns the input transfer syntax for this pixel data transcode
  /// transform. This is determined by the File Meta Information in the incoming
  /// DICOM P10 token stream.
  ///
  pub fn input_transfer_syntax(&self) -> &'static TransferSyntax {
    self.input_transfer_syntax
  }

  /// Returns the output transfer syntax for this pixel data transcode
  /// transform, as specified when it was created.
  ///
  pub fn output_transfer_syntax(&self) -> &'static TransferSyntax {
    self.output_transfer_syntax
  }

  /// Whether this pixel data transcode is active. A pixel data transcode
  /// becomes inactive if its determined that the incoming token stream does not
  /// contain a valid Image Pixel Module. Such a token stream will then not be
  /// altered by this transform, and all tokens pass through unchanged.
  ///
  pub fn is_active(&self) -> bool {
    self.initial_token_buffer.is_some()
      || self
        .input_image_pixel_module_transform
        .get_output()
        .is_some()
  }

  /// Adds the next token to the P10 pixel data transcode transform and outputs
  /// an altered token stream containing the transcoded data set. Some internal
  /// buffering of tokens is expected.
  ///
  pub fn add_token(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<P10Token>, P10PixelDataTranscodeTransformError> {
    if !self.is_active() {
      return Ok(vec![token.clone()]);
    }

    // Store the input transfer syntax if one is specified in the File Meta
    // Information
    if let P10Token::FileMetaInformation { data_set } = token {
      if data_set.has(dictionary::TRANSFER_SYNTAX_UID.tag) {
        self.input_transfer_syntax = data_set
          .get_transfer_syntax()
          .map_err(P10PixelDataTranscodeTransformError::DataError)?;
      }
    }

    // Pass the token through the transform that extracts the Image Pixel Module
    // in the incoming data set
    match self.input_image_pixel_module_transform.add_token(token) {
      Ok(()) => (),

      // If the Image Pixel Module couldn't be created due to a missing data
      // element then the incoming P10 token stream is considered not to contain
      // pixel data. If this occurs, this transcode transform becomes a no-op,
      // and all the initially buffered tokens are returned unchanged.
      Err(P10CustomTypeTransformError::DataError(
        DataError::TagNotPresent { .. },
      )) => {
        let mut initial_token_buffer =
          core::mem::take(self.initial_token_buffer.as_mut().unwrap());
        initial_token_buffer.push(token.clone());
        self.initial_token_buffer = None;
        return Ok(initial_token_buffer);
      }

      // Otherwise, return the error as the Image Pixel Module is malformed in
      // some way
      Err(e) => return Err(map_p10_custom_type_transform_error(e)),
    }

    // Pass the token through the pixel data frames transform, receiving any
    // raw frames of pixel data that are now available
    let input_frames = self
      .p10_pixel_data_frame_transform
      .add_token(token)
      .map_err(map_p10_pixel_data_frame_transform_error)?;

    // Buffer initial tokens until the Image Pixel Module tokens are complete,
    // and then assess its content and apply alterations as required and as
    // determined by the relevant image data function.
    let Some(transformed_tokens) =
      self.add_token_to_initial_token_buffer(token)?
    else {
      return Ok(vec![]);
    };

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

    // Pass through the lossy image compression insert transform if defined
    if let Some(lossy_image_compression_insert_transform) =
      &mut self.lossy_image_compression_insert_transform
    {
      let mut new_tokens = vec![];

      for token in output_tokens {
        new_tokens.extend(
          lossy_image_compression_insert_transform
            .add_token(&token)
            .map_err(P10PixelDataTranscodeTransformError::P10Error)?,
        );
      }

      output_tokens = new_tokens;
    }

    Ok(output_tokens)
  }

  /// Adds the next token to the buffer of initial tokens that buffers all
  /// tokens until the Image Pixel Module has been fully received. The initial
  /// tokens are then transformed as required, modifying the Image Pixel Module
  /// for the desired transcode, and the new set of initial tokens is returned.
  ///
  /// After th above process has completed, the initial token buffer is removed
  /// and this function passes through all future tokens unchanged.
  ///
  fn add_token_to_initial_token_buffer(
    &mut self,
    token: &P10Token,
  ) -> Result<Option<Vec<P10Token>>, P10PixelDataTranscodeTransformError> {
    // If initial token buffering is complete then return the token unchanged
    let Some(initial_token_buffer) = &mut self.initial_token_buffer else {
      return Ok(Some(vec![token.clone()]));
    };

    let Some(image_pixel_module) =
      self.input_image_pixel_module_transform.get_output()
    else {
      // This token is prior to the end of the Image Pixel Module, so accumulate
      // it into the initial token buffer
      initial_token_buffer.push(token.clone());
      return Ok(None);
    };

    let initial_token_buffer = core::mem::take(initial_token_buffer);

    // Initial token buffering is now complete
    self.initial_token_buffer = None;

    // Make any required changes/updates to the initial tokens needed for
    // changes to the Image Pixel Module
    let (mut tokens, decoded_image_pixel_module, output_image_pixel_module) =
      Self::transform_image_pixel_module_tokens(
        image_pixel_module,
        initial_token_buffer,
        self.input_transfer_syntax,
        self.output_transfer_syntax,
        &self.encode_config,
        &mut self.image_data_functions,
      )?;

    self.decoded_image_pixel_module = Some(decoded_image_pixel_module);
    self.output_image_pixel_module = Some(output_image_pixel_module);

    tokens.push(token.clone());

    // Change the Transfer Syntax UID in the File Meta Information token
    for token in &mut tokens {
      token.change_transfer_syntax(self.output_transfer_syntax);
    }

    Ok(Some(tokens))
  }

  /// Once the Image Pixel Module has been read, this function is called to
  /// apply any required updates to its content in the transcoded tokens, e.g.
  /// a change to the Photometric Interpretation. These changes are defined by
  /// the pixel data decoder, the relevant image data function, and finally the
  /// pixel data encoder.
  ///
  fn transform_image_pixel_module_tokens(
    image_pixel_module: &ImagePixelModule,
    initial_token_buffer: Vec<P10Token>,
    input_transfer_syntax: &'static TransferSyntax,
    output_transfer_syntax: &'static TransferSyntax,
    encode_config: &PixelDataEncodeConfig,
    image_data_functions: &mut TranscodeImageDataFunctions,
  ) -> Result<
    (Vec<P10Token>, ImagePixelModule, ImagePixelModule),
    P10PixelDataTranscodeTransformError,
  > {
    // Special case for direct recompression/reconstruction of JPEG Baseline
    // 8-bit to/from JPEG XL. This is a fast path that can be taken when a full
    // encode/decode cycle isn't needed.
    if !(image_data_functions.is_encode_decode_cycle_required)(
      image_pixel_module,
    ) && (input_transfer_syntax == &transfer_syntax::JPEG_BASELINE_8BIT
      && output_transfer_syntax == &transfer_syntax::JPEG_XL_JPEG_RECOMPRESSION
      || input_transfer_syntax == &transfer_syntax::JPEG_XL_JPEG_RECOMPRESSION
        && output_transfer_syntax == &transfer_syntax::JPEG_BASELINE_8BIT)
    {
      return Ok((
        initial_token_buffer,
        image_pixel_module.clone(),
        image_pixel_module.clone(),
      ));
    }

    let mut image_pixel_module = image_pixel_module.clone();

    // Determine the photometric interpretation after decoding
    let decoded_photometric_interpretation =
      decode::decode_photometric_interpretation(
        image_pixel_module.photometric_interpretation(),
        input_transfer_syntax,
      )
      .map_err(P10PixelDataTranscodeTransformError::PixelDataDecodeError)?;

    image_pixel_module.set_photometric_interpretation(
      decoded_photometric_interpretation.clone(),
    );

    // Pass through the relevant image data function
    (image_data_functions.process_image_pixel_module)(&mut image_pixel_module)?;

    // Determine the output Image Pixel Module after encoding
    let output_image_pixel_module = encode::encode_image_pixel_module(
      image_pixel_module.clone(),
      output_transfer_syntax,
      encode_config,
    )
    .map_err(P10PixelDataTranscodeTransformError::PixelDataEncodeError)?;

    // Create filter transform for excluding the input Image Pixel Module
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

    // Pass the initial token buffer through the above two transforms
    let mut transformed_tokens = Vec::with_capacity(initial_token_buffer.len());
    for token in &initial_token_buffer {
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
    // Special case for direct recompression/reconstruction of JPEG Baseline
    // 8-bit to/from JPEG XL. This is a fast path that can be taken when a full
    // encode/decode cycle isn't needed.
    #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
    if !(self.image_data_functions.is_encode_decode_cycle_required)(
      self
        .input_image_pixel_module_transform
        .get_output()
        .unwrap(),
    ) {
      use transfer_syntax::{JPEG_BASELINE_8BIT, JPEG_XL_JPEG_RECOMPRESSION};

      if self.input_transfer_syntax == &JPEG_BASELINE_8BIT
        && self.output_transfer_syntax == &JPEG_XL_JPEG_RECOMPRESSION
      {
        let jpeg_data = input_frame.combine_chunks();
        let jpeg_xl_data =
          crate::jpeg_xl_jpeg_recompression::recompress_jpeg_to_jpeg_xl(
            jpeg_data,
          )
          .map_err(P10PixelDataTranscodeTransformError::PixelDataEncodeError)?;

        return Ok(jpeg_xl_data.into());
      }

      if self.input_transfer_syntax == &JPEG_XL_JPEG_RECOMPRESSION
        && self.output_transfer_syntax == &JPEG_BASELINE_8BIT
      {
        let jpeg_xl_data = input_frame.combine_chunks();
        let jpeg_data =
          crate::jpeg_xl_jpeg_recompression::reconstruct_jpeg_from_jpeg_xl(
            jpeg_xl_data,
          )
          .map_err(P10PixelDataTranscodeTransformError::PixelDataEncodeError)?;

        return Ok(jpeg_data.into());
      }
    }

    let image_pixel_module = self
      .input_image_pixel_module_transform
      .get_output()
      .unwrap();

    let output_frame = if image_pixel_module.is_color() {
      // Decode using the input Image Pixel Module
      let mut image = crate::decode::decode_color(
        input_frame,
        self.input_transfer_syntax,
        image_pixel_module,
        &self.decode_config,
      )
      .map_err(P10PixelDataTranscodeTransformError::PixelDataDecodeError)?;

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
      .map_err(P10PixelDataTranscodeTransformError::PixelDataEncodeError)?
    } else {
      // Decode using the input Image Pixel Module
      let mut image = crate::decode::decode_monochrome(
        input_frame,
        self.input_transfer_syntax,
        image_pixel_module,
        &self.decode_config,
      )
      .map_err(P10PixelDataTranscodeTransformError::PixelDataDecodeError)?;

      // Pass through the relevant image data function
      (self.image_data_functions.process_monochrome_image)(
        &mut image,
        self.output_image_pixel_module.as_ref().unwrap(),
      )?;

      // Encode using the output Image Pixel Module
      let frame = crate::encode::encode_monochrome(
        &image,
        self.output_image_pixel_module.as_ref().unwrap(),
        self.output_transfer_syntax,
        &self.encode_config,
      )
      .map_err(P10PixelDataTranscodeTransformError::PixelDataEncodeError)?;

      // Transcoding of multi-frame data where the frames aren't a whole number
      // of bytes isn't supported. This is an extremely rare occurrence as it
      // only occurs on non-encapsulated multi-frame data where bits allocated
      // is one and the pixel count isn't a multiple of eight.
      if !self.output_transfer_syntax.is_encapsulated
        && input_frame.index().unwrap() != 0
        && frame.len_bits() % 8 != 0
      {
        return Err(P10PixelDataTranscodeTransformError::NotSupported {
          details: "Transcoding multi-frame bitmap pixel data that isn't \
            byte-aligned is not supported"
            .to_string(),
        });
      }

      frame
    };

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

  /// If the output transfer is lossy, returns an insert transform that sets
  /// '(0028,2110) Lossy Image Compression'.
  ///
  fn lossy_image_compression_insert_transform(
    output_transfer_syntax: &'static TransferSyntax,
  ) -> Option<P10InsertTransform> {
    let lossy_output_transfer_syntaxes = [
      &transfer_syntax::JPEG_BASELINE_8BIT,
      &transfer_syntax::JPEG_EXTENDED_12BIT,
      &transfer_syntax::JPEG_LS_LOSSY_NEAR_LOSSLESS,
      &transfer_syntax::JPEG_2000,
      &transfer_syntax::JPEG_2000_MULTI_COMPONENT,
      &transfer_syntax::HIGH_THROUGHPUT_JPEG_2000,
      &transfer_syntax::JPEG_XL,
      &transfer_syntax::JPEG_XL_JPEG_RECOMPRESSION,
    ];

    if lossy_output_transfer_syntaxes.contains(&output_transfer_syntax) {
      let mut lossy_image_compression = DataSet::new();
      lossy_image_compression.insert(
        dictionary::LOSSY_IMAGE_COMPRESSION.tag,
        DataElementValue::new_code_string(&["01"]).unwrap(),
      );

      Some(P10InsertTransform::new(lossy_image_compression))
    } else {
      None
    }
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
  NotSupported { details: String },
}

impl core::fmt::Display for P10PixelDataTranscodeTransformError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::DataError(e) => e.fmt(f),
      Self::P10Error(e) => e.fmt(f),
      Self::PixelDataDecodeError(e) => e.fmt(f),
      Self::PixelDataEncodeError(e) => e.fmt(f),
      Self::NotSupported { details } => {
        write!(f, "Transcode not supported, details: {details}")
      }
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
      Self::NotSupported { details } => {
        vec![
          format!("Pixel data transcode error {}", task_description),
          "".to_string(),
          format!("  Details: {}", details),
        ]
      }
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

impl Default for TranscodeImageDataFunctions {
  fn default() -> Self {
    Self {
      is_encode_decode_cycle_required: Box::new(|_| false),
      process_image_pixel_module: Box::new(|_| Ok(())),
      process_monochrome_image: Box::new(|_, _| Ok(())),
      process_color_image: Box::new(|_, _| Ok(())),
    }
  }
}

pub type TranscodedPhotometricInterpretationFn =
  dyn Fn(&ImagePixelModule) -> Option<PhotometricInterpretation>;

impl TranscodeImageDataFunctions {
  /// Creates image data functions for use when transcoding pixel data that do
  /// a number of standard alterations to the Image Pixel Module and pixel data
  /// required for typical common pixel data transcoding cases.
  ///
  /// Without this base functionality many transcodings will fail due to
  /// the source's Image Pixel Module not being compatible with the output
  /// transfer syntax.
  ///
  /// The current behavior of these image data functions is:
  ///
  /// - Sensible alterations to the photometric interpretation based on the
  ///   output transfer syntax's capabilities and requirements.
  /// - Conversion of PALETTE_COLOR data into RGB when the output transfer
  ///   syntax doesn't support PALETTE_COLOR.
  /// - Expansion of YBR_FULL_422 to YBR_FULL when the output transfer syntax
  ///   doesn't support YBR_FULL_422.
  /// - Optional crop rectangle to apply to the pixel data.
  ///
  /// Also provided is the ability to set a desired photometric interpretation
  /// for the output pixel data, as well as the desired planar configuration.
  /// These must be valid for the output transfer syntax, otherwise the encode
  /// step is likely to fail during pixel data transcoding.
  ///
  pub fn standard_behavior(
    output_transfer_syntax: &'static TransferSyntax,
    photometric_interpretation_monochrome: Rc<
      TranscodedPhotometricInterpretationFn,
    >,
    photometric_interpretation_color: Rc<TranscodedPhotometricInterpretationFn>,
    planar_configuration: Option<PlanarConfiguration>,
    crop_rect: Option<CropRect>,
    is_output_quality_specified: bool,
  ) -> Self {
    let is_encode_decode_cycle_required = {
      let photometric_interpretation_monochrome =
        photometric_interpretation_monochrome.clone();
      let photometric_interpretation_color =
        photometric_interpretation_color.clone();

      move |image_pixel_module: &ImagePixelModule| {
        // Check whether the planar configuration is being changed
        if let Some(planar_configuration) = planar_configuration
          && image_pixel_module.planar_configuration() != planar_configuration
          && output_transfer_syntax.supports_planar_configuration()
        {
          return true;
        }

        // Check whether the photometric interpretation is being changed
        if image_pixel_module.is_monochrome()
          && let Some(photometric_interpretation) =
            photometric_interpretation_monochrome(image_pixel_module)
          && image_pixel_module.photometric_interpretation()
            != &photometric_interpretation
        {
          return true;
        }
        if image_pixel_module.is_color()
          && let Some(photometric_interpretation) =
            photometric_interpretation_color(image_pixel_module)
          && image_pixel_module.photometric_interpretation()
            != &photometric_interpretation
        {
          return true;
        }

        // Check whether there is an active crop
        if crop_rect.is_some() {
          return true;
        }

        // Check whether there is an active quality change being applied that
        // would require a decode/encode cycle
        if is_output_quality_specified
          && output_transfer_syntax.is_lossy_compression_adjustable()
        {
          return true;
        }

        false
      }
    };

    let process_image_pixel_module =
      move |image_pixel_module: &mut ImagePixelModule| {
        // For grayscale pixel data, the photometric interpretation, if set, can
        // be either MONOCHROME1 or MONOCHROME2
        if image_pixel_module.is_monochrome() {
          if let Some(photometric_interpretation) =
            photometric_interpretation_monochrome(image_pixel_module)
          {
            image_pixel_module
              .set_photometric_interpretation(photometric_interpretation);
          }
        } else if image_pixel_module.is_color() {
          // If a photometric interpretation has been explicitly specified then
          // use it for the output
          if let Some(photometric_interpretation) =
            photometric_interpretation_color(image_pixel_module)
          {
            // If the input is palette color and the specified output
            // photometric interpretation isn't palette color then the palette
            // will be applied
            if let PhotometricInterpretation::PaletteColor { palette } =
              image_pixel_module.photometric_interpretation()
              && !photometric_interpretation.is_palette_color()
            {
              image_pixel_module.set_as_palette_output(&palette.clone());
            }

            image_pixel_module
              .set_photometric_interpretation(photometric_interpretation);
          } else {
            // If the input is palette color and the output transfer syntax
            // doesn't support palette color then the palette will be applied
            if let PhotometricInterpretation::PaletteColor { palette } =
              image_pixel_module.photometric_interpretation()
              && !output_transfer_syntax.supports_palette_color()
            {
              image_pixel_module.set_as_palette_output(&palette.clone());
            }

            // If the input is YBR_FULL_422 and the output transfer syntax
            // doesn't support YBR_FULL_422 then expand to YBR_FULL by default
            if image_pixel_module
              .photometric_interpretation()
              .is_ybr_full_422()
              && !output_transfer_syntax.supports_ybr_full_422()
            {
              image_pixel_module.set_photometric_interpretation(
                PhotometricInterpretation::YbrFull,
              );
            }

            match output_transfer_syntax {
              // When transcoding to JPEG Baseline 8-bit and JPEG Extended
              // 12-bit default to YBR_FULL_422
              &transfer_syntax::JPEG_BASELINE_8BIT
              | &transfer_syntax::JPEG_EXTENDED_12BIT
              | &transfer_syntax::JPEG_XL_JPEG_RECOMPRESSION => {
                if image_pixel_module.photometric_interpretation().is_rgb()
                  || image_pixel_module
                    .photometric_interpretation()
                    .is_ybr_full()
                {
                  image_pixel_module.set_photometric_interpretation(
                    PhotometricInterpretation::YbrFull422,
                  );
                }
              }

              // When transcoding to JPEG 2000 Lossless Only default to YBR_RCT
              // unless the incoming data is PALETTE_COLOR
              &transfer_syntax::JPEG_2000_LOSSLESS_ONLY
              | &transfer_syntax::HIGH_THROUGHPUT_JPEG_2000_LOSSLESS_ONLY
                if !image_pixel_module
                  .photometric_interpretation()
                  .is_palette_color() =>
              {
                image_pixel_module.set_photometric_interpretation(
                  PhotometricInterpretation::YbrRct,
                )
              }

              // When transcoding to JPEG 2000 lossy default to YBR_ICT
              &transfer_syntax::JPEG_2000
              | &transfer_syntax::HIGH_THROUGHPUT_JPEG_2000 => {
                image_pixel_module.set_photometric_interpretation(
                  PhotometricInterpretation::YbrIct,
                )
              }

              // When transcoding to JPEG XL lossless default to RGB
              &transfer_syntax::JPEG_XL_LOSSLESS => image_pixel_module
                .set_photometric_interpretation(PhotometricInterpretation::Rgb),

              // When transcoding to JPEG XL lossy default to XYB
              &transfer_syntax::JPEG_XL => image_pixel_module
                .set_photometric_interpretation(PhotometricInterpretation::Xyb),

              _ => (),
            }
          }

          // If a planar configuration has been explicitly specified then use it
          // for the output. Not all transfer syntaxes reference the planar
          // configuration.
          if output_transfer_syntax.supports_planar_configuration()
            && let Some(planar_configuration) = planar_configuration
          {
            image_pixel_module.set_planar_configuration(planar_configuration);
          }
        }

        // Apply crop to dimensions
        if let Some(crop_rect) = crop_rect {
          let (cropped_rows, cropped_columns) = crop_rect
            .apply(image_pixel_module.rows(), image_pixel_module.columns());

          if let Err(e) =
            image_pixel_module.set_dimensions(cropped_rows, cropped_columns)
          {
            return Err(P10PixelDataTranscodeTransformError::NotSupported {
              details: e,
            });
          }
        }

        Ok(())
      };

    let process_monochrome_image =
      move |image: &mut MonochromeImage,
            image_pixel_module: &ImagePixelModule| {
        // Convert to MONOCHROME1/MONOCHROME2 based on the output photometric
        // interpretation
        match image_pixel_module.photometric_interpretation() {
          PhotometricInterpretation::Monochrome1 { .. } => {
            if !image.is_monochrome1() {
              image.change_monochrome_representation();
            }
          }

          PhotometricInterpretation::Monochrome2 { .. } => {
            if image.is_monochrome1() {
              image.change_monochrome_representation();
            }
          }

          _ => (),
        }

        // Crop image data
        if let Some(crop_rect) = crop_rect {
          image.crop(&crop_rect);
        }

        Ok(())
      };

    let process_color_image =
      move |image: &mut ColorImage, image_pixel_module: &ImagePixelModule| {
        // Convert palette color to RGB if the output image pixel module isn't
        // in palette color
        if image.is_palette_color()
          && !image_pixel_module
            .photometric_interpretation()
            .is_palette_color()
        {
          image.convert_palette_color_to_rgb();
        }

        let photometric_interpretation =
          image_pixel_module.photometric_interpretation();

        // If the output image pixel module is using RGB, or needs RGB color
        // data as its input, then convert the color image to RGB
        if photometric_interpretation.is_rgb()
          || photometric_interpretation.is_ybr_ict()
          || photometric_interpretation.is_ybr_rct()
          || photometric_interpretation.is_xyb()
        {
          image.convert_to_rgb_color_space()
        }

        // If the output image pixel module is using YBR_FULL then convert the
        // color image
        if photometric_interpretation.is_ybr_full() {
          image.convert_to_ybr_color_space();
        }

        // If the output image pixel module is using YBR_FULL_422 then convert
        // the color image
        if photometric_interpretation.is_ybr_full_422() {
          image.convert_to_ybr_422_color_space().map_err(|_| {
            P10PixelDataTranscodeTransformError::NotSupported {
              details: "Can't convert to YBR_FULL_422 because width is odd"
                .to_string(),
            }
          })?;
        }

        // Crop image data
        if let Some(crop_rect) = crop_rect {
          image.crop(&crop_rect);
        }

        Ok(())
      };

    Self {
      is_encode_decode_cycle_required: Box::new(
        is_encode_decode_cycle_required,
      ),
      process_image_pixel_module: Box::new(process_image_pixel_module),
      process_monochrome_image: Box::new(process_monochrome_image),
      process_color_image: Box::new(process_color_image),
    }
  }
}
