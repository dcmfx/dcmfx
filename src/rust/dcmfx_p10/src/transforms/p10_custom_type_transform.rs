#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use dcmfx_core::{DataElementTag, DataError, DataSet, DcmfxError, IodModule};

use crate::{DataSetBuilder, P10Error, P10FilterTransform, P10Token};

/// Transforms a stream of DICOM P10 tokens into a custom type. This is done by:
///
/// 1. Specifying the tags of the data elements needed to create the custom
///    type.
/// 2. Extracting the specified data elements from the incoming DICOM P10 token
///    stream into a data set.
/// 3. Passing the data set to a function that creates the custom type.
///
/// The result is then accessed using [`P10CustomTypeTransform::get_output()`]
/// which returns `None` if the target is not yet available or was unable to be
/// created.
///
pub struct P10CustomTypeTransform<T> {
  filter: Option<(P10FilterTransform, DataSetBuilder)>,
  highest_tag: DataElementTag,
  target_from_data_set: TargetFromDataSetFn<T>,
  target: Option<T>,
}

type TargetFromDataSetFn<T> = fn(&DataSet) -> Result<T, DataError>;

/// An error that occurred in the process of converting a stream DICOM P10
/// tokens to a custom type.
///
#[derive(Clone, Debug, PartialEq)]
pub enum P10CustomTypeTransformError {
  /// An error that occurred when adding a P10 token to the data set builder.
  /// This can happen when the stream of DICOM P10 tokens is invalid.
  P10Error(P10Error),

  /// An error that occurred when creating the custom type from the gathered
  /// data set.
  DataError(DataError),
}

impl core::fmt::Display for P10CustomTypeTransformError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::P10Error(e) => e.fmt(f),
      Self::DataError(e) => e.fmt(f),
    }
  }
}

impl DcmfxError for P10CustomTypeTransformError {
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    match self {
      Self::P10Error(e) => e.to_lines(task_description),
      Self::DataError(e) => e.to_lines(task_description),
    }
  }
}

impl<T> P10CustomTypeTransform<T> {
  /// Creates a new transform for converting a stream of DICOM P10 tokens to
  /// a custom type. The data elements needed by the custom type must be
  /// specified.
  ///
  pub fn new(
    tags: &'static [DataElementTag],
    target_from_data_set: TargetFromDataSetFn<T>,
  ) -> Self {
    let filter =
      P10FilterTransform::new(Box::new(move |tag, _vr, _length, _location| {
        tags.contains(&tag)
      }));

    let highest_tag = *tags.iter().max().unwrap_or(&DataElementTag::ZERO);

    Self {
      filter: Some((filter, DataSetBuilder::new())),
      highest_tag,
      target_from_data_set,
      target: None,
    }
  }

  /// Creates a new transform for converting a stream of DICOM P10 tokens into
  /// a specific [`IodModule`].
  ///
  pub fn new_for_iod_module() -> Self
  where
    T: IodModule,
  {
    let filter =
      P10FilterTransform::new(Box::new(move |tag, vr, length, _location| {
        T::is_iod_module_data_element(tag, vr, length, _location)
      }));

    Self {
      filter: Some((filter, DataSetBuilder::new())),
      highest_tag: T::iod_module_highest_tag(),
      target_from_data_set: T::from_data_set,
      target: None,
    }
  }

  /// Adds the next token in the DICOM P10 token stream.
  ///
  pub fn add_token(
    &mut self,
    token: &P10Token,
  ) -> Result<(), P10CustomTypeTransformError> {
    if let Some((filter, data_set_builder)) = self.filter.as_mut() {
      let is_at_root = filter.is_at_root();

      if filter
        .add_token(token)
        .map_err(P10CustomTypeTransformError::P10Error)?
      {
        data_set_builder
          .add_token(token)
          .map_err(P10CustomTypeTransformError::P10Error)?;
      }

      // Check whether all the relevant tags have now been read. If they have
      // then the final type can be constructed.
      let is_complete = is_at_root
        && match token {
          P10Token::DataElementHeader { tag, .. }
          | P10Token::SequenceStart { tag, .. } => *tag > self.highest_tag,

          P10Token::DataElementValueBytes {
            tag,
            bytes_remaining: 0,
            ..
          }
          | P10Token::SequenceDelimiter { tag } => *tag == self.highest_tag,

          P10Token::End => true,

          _ => false,
        };

      if is_complete {
        data_set_builder.force_end();
        let data_set = data_set_builder.final_data_set().unwrap();

        let target = (self.target_from_data_set)(&data_set)
          .map_err(P10CustomTypeTransformError::DataError)?;

        self.target = Some(target);
        self.filter = None;
      }
    }

    Ok(())
  }

  /// Returns the custom type created by this transform. This is set once all
  /// the required data elements have been gathered from the stream of DICOM P10
  /// tokens and successfully constructed into the custom type.
  ///
  pub fn get_output(&self) -> Option<&T> {
    self.target.as_ref()
  }

  /// Returns the custom type created by this transform. This is set once all
  /// the required data elements have been gathered from the stream of DICOM P10
  /// tokens and successfully constructed into the custom type.
  ///
  pub fn get_output_mut(&mut self) -> &mut Option<T> {
    &mut self.target
  }
}
