import dcmfx_core/data_element_tag.{DataElementTag}
import dcmfx_core/registry
import dcmfx_core/value_multiplicity.{ValueMultiplicity}
import dcmfx_core/value_representation
import gleam/option.{None, Some}
import gleeunit/should

pub fn tag_name_test() {
  registry.tag_name(DataElementTag(0x0010, 0x0010), None)
  |> should.equal("Patient's Name")

  registry.tag_name(DataElementTag(0x1234, 0x5678), None)
  |> should.equal("unknown_tag")

  registry.tag_name(DataElementTag(0x1231, 0), None)
  |> should.equal("unknown_private_tag")
}

pub fn tag_with_name_test() {
  registry.tag_with_name(DataElementTag(0x0010, 0x0010), None)
  |> should.equal("(0010,0010) Patient's Name")

  registry.tag_with_name(DataElementTag(0x1234, 0x5678), None)
  |> should.equal("(1234,5678) unknown_tag")

  registry.tag_with_name(DataElementTag(0x1231, 0), None)
  |> should.equal("(1231,0000) unknown_private_tag")
}

pub fn find_test() {
  registry.find(DataElementTag(0x0010, 0x0010), None)
  |> should.equal(
    Ok(registry.Item(
      DataElementTag(0x0010, 0x0010),
      "Patient's Name",
      [value_representation.PersonName],
      ValueMultiplicity(1, Some(1)),
    )),
  )

  let tag = DataElementTag(0x0029, 0x0160)

  registry.find(tag, Some("SIEMENS MEDCOM HEADER2"))
  |> should.equal(
    Ok(registry.Item(
      tag,
      name: "Series Workflow Status",
      vrs: [value_representation.LongString],
      multiplicity: ValueMultiplicity(1, Some(1)),
    )),
  )

  registry.find(DataElementTag(0x0000, 0xFFFF), None)
  |> should.equal(Error(Nil))
}
