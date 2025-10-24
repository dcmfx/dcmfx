import dcmfx_core/data_element_value/person_name.{
  PersonNameComponents, StructuredPersonName,
}
import dcmfx_core/data_error
import gleam/option.{None, Some}
import gleam/string

pub fn from_bytes_test() {
  assert person_name.from_bytes(<<>>)
    == Ok([StructuredPersonName(None, None, None)])

  assert person_name.from_bytes(<<"A^B^^^">>)
    == Ok([
      StructuredPersonName(
        Some(PersonNameComponents("A", "B", "", "", "")),
        None,
        None,
      ),
    ])

  assert person_name.from_bytes(<<"A^B^C^D^E">>)
    == Ok([
      StructuredPersonName(
        Some(PersonNameComponents("A", "B", "C", "D", "E")),
        None,
        None,
      ),
    ])

  assert person_name.from_bytes(<<"A^B^C^D^E=1^2^3^4^5=v^w^x^y^z">>)
    == Ok([
      StructuredPersonName(
        Some(PersonNameComponents("A", "B", "C", "D", "E")),
        Some(PersonNameComponents("1", "2", "3", "4", "5")),
        Some(PersonNameComponents("v", "w", "x", "y", "z")),
      ),
    ])

  assert person_name.from_bytes(<<0xD0>>)
    == Error(data_error.new_value_invalid("PersonName is invalid UTF-8"))

  assert person_name.from_bytes(<<"A=B=C=D">>)
    == Error(data_error.new_value_invalid(
      "PersonName has too many component groups: 4",
    ))

  assert person_name.from_bytes(<<"A^B^C^D^E^F">>)
    == Error(data_error.new_value_invalid(
      "PersonName has too many components: 6",
    ))
}

pub fn to_bytes_test() {
  assert person_name.to_bytes([
      StructuredPersonName(
        Some(PersonNameComponents("A", "B", "C", "D", "E")),
        Some(PersonNameComponents("1", "2", "3", "4", "5")),
        Some(PersonNameComponents("v", "w", "x", "y", "z")),
      ),
    ])
    == Ok(<<"A^B^C^D^E=1^2^3^4^5=v^w^x^y^z ">>)

  assert person_name.to_bytes([
      StructuredPersonName(
        None,
        Some(PersonNameComponents("A", "B", "C", "", "E")),
        None,
      ),
    ])
    == Ok(<<"=A^B^C^^E ">>)

  assert person_name.to_bytes([
      StructuredPersonName(
        Some(PersonNameComponents("^", "", "", "", "")),
        None,
        None,
      ),
    ])
    == Error(data_error.new_value_invalid(
      "PersonName component has disallowed characters",
    ))

  assert person_name.to_bytes([
      StructuredPersonName(
        Some(PersonNameComponents(string.repeat("A", 65), "", "", "", "")),
        None,
        None,
      ),
    ])
    == Error(data_error.new_value_invalid("PersonName component is too long"))
}
