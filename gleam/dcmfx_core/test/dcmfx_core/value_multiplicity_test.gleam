import dcmfx_core/value_multiplicity.{ValueMultiplicity}
import gleam/option.{None, Some}
import gleeunit/should

pub fn to_string_test() {
  ValueMultiplicity(1, Some(1))
  |> value_multiplicity.to_string
  |> should.equal("1")

  ValueMultiplicity(1, Some(3))
  |> value_multiplicity.to_string
  |> should.equal("1-3")

  ValueMultiplicity(1, None)
  |> value_multiplicity.to_string
  |> should.equal("1-n")
}
