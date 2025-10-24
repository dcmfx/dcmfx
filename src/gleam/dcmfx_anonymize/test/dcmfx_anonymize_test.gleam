import dcmfx_anonymize
import dcmfx_core/data_element_tag.{DataElementTag}
import dcmfx_core/dictionary
import dcmfx_core/value_representation
import gleeunit

pub fn main() {
  gleeunit.main()
}

pub fn filter_tag_test() {
  assert dcmfx_anonymize.filter_tag(
    dictionary.specific_character_set.tag,
    value_representation.CodeString,
  )

  assert !dcmfx_anonymize.filter_tag(
    dictionary.uid.tag,
    value_representation.UniqueIdentifier,
  )

  assert dcmfx_anonymize.filter_tag(
      dictionary.station_ae_title.tag,
      value_representation.ApplicationEntity,
    )
    == False

  assert dcmfx_anonymize.filter_tag(
      DataElementTag(0x0009, 0x0002),
      value_representation.CodeString,
    )
    == False

  assert dcmfx_anonymize.filter_tag(
      DataElementTag(0x0010, 0xABCD),
      value_representation.PersonName,
    )
    == False
}
