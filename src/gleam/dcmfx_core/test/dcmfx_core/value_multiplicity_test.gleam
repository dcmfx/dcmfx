import dcmfx_core/value_multiplicity.{ValueMultiplicity}
import gleam/option.{None, Some}

pub fn to_string_test() {
  assert value_multiplicity.to_string(ValueMultiplicity(1, Some(1))) == "1"

  assert value_multiplicity.to_string(ValueMultiplicity(1, Some(3))) == "1-3"

  assert value_multiplicity.to_string(ValueMultiplicity(1, None)) == "1-n"
}
