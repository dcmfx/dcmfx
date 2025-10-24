import dcmfx_core/dictionary
import dcmfx_core/value_representation
import dcmfx_p10/internal/data_element_header.{DataElementHeader}
import dcmfx_p10/internal/value_length
import gleam/option.{None, Some}

pub fn to_string_test() {
  assert data_element_header.to_string(DataElementHeader(
      dictionary.patient_age.tag,
      Some(value_representation.AgeString),
      value_length.zero,
    ))
    == "(0010,1010) AS Patient's Age"

  assert data_element_header.to_string(DataElementHeader(
      dictionary.item.tag,
      None,
      value_length.zero,
    ))
    == "(FFFE,E000)    Item"
}
