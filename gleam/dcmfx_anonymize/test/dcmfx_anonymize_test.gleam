import dcmfx_anonymize
import dcmfx_core/data_element_tag.{DataElementTag}
import dcmfx_core/registry
import dcmfx_core/value_representation
import gleeunit
import gleeunit/should

pub fn main() {
  gleeunit.main()
}

pub fn filter_tag_test() {
  dcmfx_anonymize.filter_tag(
    registry.specific_character_set.tag,
    value_representation.CodeString,
  )
  |> should.equal(True)

  dcmfx_anonymize.filter_tag(
    registry.uid.tag,
    value_representation.UniqueIdentifier,
  )
  |> should.equal(False)

  dcmfx_anonymize.filter_tag(
    registry.station_ae_title.tag,
    value_representation.ApplicationEntity,
  )
  |> should.equal(False)

  dcmfx_anonymize.filter_tag(
    DataElementTag(0x0009, 0x0002),
    value_representation.CodeString,
  )
  |> should.equal(False)

  dcmfx_anonymize.filter_tag(
    DataElementTag(0x0010, 0xABCD),
    value_representation.PersonName,
  )
  |> should.equal(False)
}
