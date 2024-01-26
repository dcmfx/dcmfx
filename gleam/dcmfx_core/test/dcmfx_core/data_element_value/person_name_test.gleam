import dcmfx_core/data_element_value/person_name.{
  PersonNameComponents, StructuredPersonName,
}
import dcmfx_core/data_error
import gleam/option.{None, Some}
import gleam/string
import gleeunit/should

pub fn from_bytes_test() {
  <<>>
  |> person_name.from_bytes
  |> should.equal(Ok([StructuredPersonName(None, None, None)]))

  <<"A^B^^^">>
  |> person_name.from_bytes
  |> should.equal(
    Ok([
      StructuredPersonName(
        Some(PersonNameComponents("A", "B", "", "", "")),
        None,
        None,
      ),
    ]),
  )

  <<"A^B^C^D^E">>
  |> person_name.from_bytes
  |> should.equal(
    Ok([
      StructuredPersonName(
        Some(PersonNameComponents("A", "B", "C", "D", "E")),
        None,
        None,
      ),
    ]),
  )

  <<"A^B^C^D^E=1^2^3^4^5=v^w^x^y^z">>
  |> person_name.from_bytes
  |> should.equal(
    Ok([
      StructuredPersonName(
        Some(PersonNameComponents("A", "B", "C", "D", "E")),
        Some(PersonNameComponents("1", "2", "3", "4", "5")),
        Some(PersonNameComponents("v", "w", "x", "y", "z")),
      ),
    ]),
  )

  <<0xD0>>
  |> person_name.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("PersonName is invalid UTF-8")),
  )

  <<"A=B=C=D">>
  |> person_name.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid(
      "PersonName has too many component groups: 4",
    )),
  )

  <<"A^B^C^D^E^F">>
  |> person_name.from_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("PersonName has too many components: 6")),
  )
}

pub fn to_bytes_test() {
  [
    StructuredPersonName(
      Some(PersonNameComponents("A", "B", "C", "D", "E")),
      Some(PersonNameComponents("1", "2", "3", "4", "5")),
      Some(PersonNameComponents("v", "w", "x", "y", "z")),
    ),
  ]
  |> person_name.to_bytes
  |> should.equal(Ok(<<"A^B^C^D^E=1^2^3^4^5=v^w^x^y^z ">>))

  [
    StructuredPersonName(
      None,
      Some(PersonNameComponents("A", "B", "C", "", "E")),
      None,
    ),
  ]
  |> person_name.to_bytes
  |> should.equal(Ok(<<"=A^B^C^^E ">>))

  [
    StructuredPersonName(
      Some(PersonNameComponents("^", "", "", "", "")),
      None,
      None,
    ),
  ]
  |> person_name.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid(
      "PersonName component has disallowed characters",
    )),
  )

  [
    StructuredPersonName(
      Some(PersonNameComponents(string.repeat("A", 65), "", "", "", "")),
      None,
      None,
    ),
  ]
  |> person_name.to_bytes
  |> should.equal(
    Error(data_error.new_value_invalid("PersonName component is too long")),
  )
}
