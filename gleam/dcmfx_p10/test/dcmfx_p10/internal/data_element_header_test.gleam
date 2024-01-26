import dcmfx_core/registry
import dcmfx_core/value_representation
import dcmfx_p10/internal/data_element_header.{DataElementHeader}
import gleam/option.{None, Some}
import gleeunit/should

pub fn to_string_test() {
  DataElementHeader(
    registry.patient_age.tag,
    Some(value_representation.AgeString),
    0,
  )
  |> data_element_header.to_string
  |> should.equal("(0010,1010) AS Patient's Age")

  DataElementHeader(registry.item.tag, None, 0)
  |> data_element_header.to_string
  |> should.equal("(FFFE,E000)    Item")
}
