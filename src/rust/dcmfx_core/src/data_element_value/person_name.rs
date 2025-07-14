//! Work with the DICOM `PersonName` value representation.

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec::Vec,
};

use crate::DataError;

/// The components of a single person name.
///
#[derive(Clone, Debug, PartialEq)]
pub struct PersonNameComponents {
  pub last_name: String,
  pub first_name: String,
  pub middle_name: String,
  pub prefix: String,
  pub suffix: String,
}

/// A structured person name that can be converted to/from a `PersonName` value.
/// Person name values have three variants: alphabetic, ideographic, and
/// phonetic. All variants are optional, however it is common for only the
/// alphabetic variant to be used.
///
#[derive(Clone, Debug, PartialEq)]
pub struct StructuredPersonName {
  pub alphabetic: Option<PersonNameComponents>,
  pub ideographic: Option<PersonNameComponents>,
  pub phonetic: Option<PersonNameComponents>,
}

/// Converts a `PersonName` value to a list of structured person names.
///
pub fn from_bytes(
  bytes: &[u8],
) -> Result<Vec<StructuredPersonName>, DataError> {
  let person_name_string = core::str::from_utf8(bytes).map_err(|_| {
    DataError::new_value_invalid("PersonName is invalid UTF-8".to_string())
  })?;

  let person_names = person_name_string
    .split('\\')
    .map(parse_person_name_string)
    .collect::<Result<Vec<StructuredPersonName>, _>>()?;

  Ok(person_names)
}

/// Parses a `PersonName` value by splitting it on the '=' character to find the
/// list of component groups, then splitting each component group on
/// the '^' character to find the individual components of each name variant.
///
fn parse_person_name_string(
  person_name_string: &str,
) -> Result<StructuredPersonName, DataError> {
  let component_groups: Vec<&str> = person_name_string.split('=').collect();

  let component_group_count = component_groups.len();

  if component_group_count > 3 {
    return Err(DataError::new_value_invalid(format!(
      "PersonName has too many component groups: {component_group_count}"
    )));
  }

  let mut person_names = component_groups
    .iter()
    .map(|s| parse_person_name_component_group(s))
    .collect::<Result<Vec<Option<PersonNameComponents>>, _>>()?;

  person_names.resize(3, None);

  Ok(StructuredPersonName {
    alphabetic: person_names[0].clone(),
    ideographic: person_names[1].clone(),
    phonetic: person_names[2].clone(),
  })
}

fn parse_person_name_component_group(
  component_group: &str,
) -> Result<Option<PersonNameComponents>, DataError> {
  let mut components: Vec<&str> = component_group
    .split('^')
    .map(|s| s.trim_end_matches(' '))
    .collect();

  if components.len() > 5 {
    return Err(DataError::new_value_invalid(format!(
      "PersonName has too many components: {}",
      components.len()
    )));
  }

  // If all components of the name are empty then don't return anything
  if components.iter().all(|c| c.is_empty()) {
    return Ok(None);
  }

  // Resize to reach a length of 5
  components.resize(5, "");
  Ok(Some(PersonNameComponents {
    last_name: components[0].to_string(),
    first_name: components[1].to_string(),
    middle_name: components[2].to_string(),
    prefix: components[3].to_string(),
    suffix: components[4].to_string(),
  }))
}

/// Converts a list of structured person names to a `PersonName` value.
///
pub fn to_bytes(values: &[StructuredPersonName]) -> Result<Vec<u8>, DataError> {
  let names: Result<Vec<String>, _> = values
    .iter()
    .map(|value| {
      let a: Result<Vec<String>, _> =
        [&value.alphabetic, &value.ideographic, &value.phonetic]
          .iter()
          .map(|component_group| match component_group {
            Some(components) => components_to_string(components),
            None => Ok("".to_string()),
          })
          .collect();

      Ok(a?.join("=").trim_end_matches('=').to_string())
    })
    .collect();

  let mut bytes = names?.join("\\").into_bytes();

  if bytes.len() % 2 == 1 {
    bytes.push(0x20);
  }

  Ok(bytes)
}

fn components_to_string(
  components: &PersonNameComponents,
) -> Result<String, DataError> {
  let components: [&str; 5] = [
    components.last_name.trim_end_matches(' '),
    components.first_name.trim_end_matches(' '),
    components.middle_name.trim_end_matches(' '),
    components.prefix.trim_end_matches(' '),
    components.suffix.trim_end_matches(' '),
  ];

  for component in components {
    // Check the maximum number of characters isn't exceeded
    if component.len() > 64 {
      return Err(DataError::new_value_invalid(
        "PersonName component is too long".to_string(),
      ));
    }

    // Check there are no disallowed characters used
    if component.contains(['^', '=', '\\']) {
      return Err(DataError::new_value_invalid(
        "PersonName component has disallowed characters".to_string(),
      ));
    }
  }

  Ok(components.join("^").trim_end_matches(['^']).to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(not(feature = "std"))]
  use alloc::vec;

  #[test]
  fn from_bytes_test() {
    assert_eq!(
      from_bytes(&[]),
      Ok(vec![StructuredPersonName {
        alphabetic: None,
        ideographic: None,
        phonetic: None
      }])
    );

    assert_eq!(
      from_bytes(b"A^B^^^"),
      Ok(vec![StructuredPersonName {
        alphabetic: Some(PersonNameComponents {
          last_name: "A".to_string(),
          first_name: "B".to_string(),
          middle_name: "".to_string(),
          prefix: "".to_string(),
          suffix: "".to_string()
        }),
        ideographic: None,
        phonetic: None
      }])
    );

    assert_eq!(
      from_bytes(b"A^B^C^D^E"),
      Ok(vec![StructuredPersonName {
        alphabetic: Some(PersonNameComponents {
          last_name: "A".to_string(),
          first_name: "B".to_string(),
          middle_name: "C".to_string(),
          prefix: "D".to_string(),
          suffix: "E".to_string()
        }),
        ideographic: None,
        phonetic: None
      }])
    );

    assert_eq!(
      from_bytes(b"A^B^C^D^E=1^2^3^4^5=v^w^x^y^z"),
      Ok(vec![StructuredPersonName {
        alphabetic: Some(PersonNameComponents {
          last_name: "A".to_string(),
          first_name: "B".to_string(),
          middle_name: "C".to_string(),
          prefix: "D".to_string(),
          suffix: "E".to_string()
        }),
        ideographic: Some(PersonNameComponents {
          last_name: "1".to_string(),
          first_name: "2".to_string(),
          middle_name: "3".to_string(),
          prefix: "4".to_string(),
          suffix: "5".to_string()
        }),
        phonetic: Some(PersonNameComponents {
          last_name: "v".to_string(),
          first_name: "w".to_string(),
          middle_name: "x".to_string(),
          prefix: "y".to_string(),
          suffix: "z".to_string()
        })
      }])
    );

    assert_eq!(
      from_bytes(&[0xD0]),
      Err(DataError::new_value_invalid(
        "PersonName is invalid UTF-8".to_string()
      ))
    );

    assert_eq!(
      from_bytes(b"A=B=C=D"),
      Err(DataError::new_value_invalid(
        "PersonName has too many component groups: 4".to_string()
      ))
    );

    assert_eq!(
      from_bytes(b"A^B^C^D^E^F"),
      Err(DataError::new_value_invalid(
        "PersonName has too many components: 6".to_string()
      ))
    );
  }

  #[test]
  fn to_bytes_test() {
    assert_eq!(
      to_bytes(&[StructuredPersonName {
        alphabetic: Some(PersonNameComponents {
          last_name: "A".to_string(),
          first_name: "B".to_string(),
          middle_name: "C".to_string(),
          prefix: "D".to_string(),
          suffix: "E".to_string()
        }),
        ideographic: Some(PersonNameComponents {
          last_name: "1".to_string(),
          first_name: "2".to_string(),
          middle_name: "3".to_string(),
          prefix: "4".to_string(),
          suffix: "5".to_string()
        }),
        phonetic: Some(PersonNameComponents {
          last_name: "v".to_string(),
          first_name: "w".to_string(),
          middle_name: "x".to_string(),
          prefix: "y".to_string(),
          suffix: "z".to_string()
        }),
      },]),
      Ok(b"A^B^C^D^E=1^2^3^4^5=v^w^x^y^z ".to_vec())
    );

    assert_eq!(
      to_bytes(&[StructuredPersonName {
        alphabetic: None,
        ideographic: Some(PersonNameComponents {
          last_name: "A".to_string(),
          first_name: "B".to_string(),
          middle_name: "C".to_string(),
          prefix: "".to_string(),
          suffix: "E".to_string()
        }),
        phonetic: None,
      },]),
      Ok(b"=A^B^C^^E ".to_vec())
    );

    assert_eq!(
      to_bytes(&[StructuredPersonName {
        alphabetic: Some(PersonNameComponents {
          last_name: "^".to_string(),
          first_name: "B".to_string(),
          middle_name: "C".to_string(),
          prefix: "".to_string(),
          suffix: "E".to_string()
        }),
        ideographic: None,
        phonetic: None,
      },]),
      Err(DataError::new_value_invalid(
        "PersonName component has disallowed characters".to_string()
      ))
    );

    assert_eq!(
      to_bytes(&[StructuredPersonName {
        alphabetic: Some(PersonNameComponents {
          last_name: "A".repeat(65),
          first_name: "".to_string(),
          middle_name: "".to_string(),
          prefix: "".to_string(),
          suffix: "E".to_string()
        }),
        ideographic: None,
        phonetic: None,
      },]),
      Err(DataError::new_value_invalid(
        "PersonName component is too long".to_string()
      ))
    );
  }
}
