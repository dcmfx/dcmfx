//! Provides core DICOM concepts including data sets, data elements, value
//! representations, transfer syntaxes, and a dictionary of the data elements
//! defined in DICOM PS3.6 as well as well-known private data elements.

pub mod code_strings;
pub mod data_element_tag;
pub mod data_element_value;
pub mod data_error;
pub mod data_set;
pub mod data_set_path;
pub mod dictionary;
pub mod error;
pub mod transfer_syntax;
pub mod utils;
pub mod value_multiplicity;
pub mod value_representation;

pub use data_element_tag::DataElementTag;
pub use data_element_value::DataElementValue;
pub use data_element_value::age_string::StructuredAge;
pub use data_element_value::date::StructuredDate;
pub use data_element_value::date_time::StructuredDateTime;
pub use data_element_value::person_name::{
  PersonNameComponents, StructuredPersonName,
};
pub use data_element_value::time::StructuredTime;
pub use data_error::DataError;
pub use data_set::DataSet;
pub use data_set::print::DataSetPrintOptions;
pub use data_set_path::DataSetPath;
pub use error::DcmfxError;
pub use transfer_syntax::TransferSyntax;
pub use value_multiplicity::ValueMultiplicity;
pub use value_representation::ValueRepresentation;
