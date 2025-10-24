import dcmfx_core/data_element_tag.{DataElementTag}
import dcmfx_core/dictionary
import dcmfx_core/value_multiplicity.{ValueMultiplicity}
import dcmfx_core/value_representation
import gleam/option.{None, Some}

pub fn tag_name_test() {
  assert dictionary.tag_name(DataElementTag(0x0010, 0x0010), None)
    == "Patient's Name"

  assert dictionary.tag_name(DataElementTag(0x1234, 0x5678), None)
    == "unknown_tag"

  assert dictionary.tag_name(DataElementTag(0x1231, 0), None)
    == "unknown_private_tag"
}

pub fn tag_with_name_test() {
  assert dictionary.tag_with_name(DataElementTag(0x0010, 0x0010), None)
    == "(0010,0010) Patient's Name"

  assert dictionary.tag_with_name(DataElementTag(0x1234, 0x5678), None)
    == "(1234,5678) unknown_tag"

  assert dictionary.tag_with_name(DataElementTag(0x1231, 0), None)
    == "(1231,0000) unknown_private_tag"
}

pub fn find_test() {
  assert dictionary.find(DataElementTag(0x0010, 0x0010), None)
    == Ok(dictionary.Item(
      DataElementTag(0x0010, 0x0010),
      "Patient's Name",
      [value_representation.PersonName],
      ValueMultiplicity(1, Some(1)),
    ))

  let tag = DataElementTag(0x0029, 0x0160)

  assert dictionary.find(tag, Some("SIEMENS MEDCOM HEADER2"))
    == Ok(dictionary.Item(
      tag,
      name: "Series Workflow Status",
      vrs: [value_representation.LongString],
      multiplicity: ValueMultiplicity(1, Some(1)),
    ))

  assert dictionary.find(DataElementTag(0x0000, 0xFFFF), None) == Error(Nil)
}
