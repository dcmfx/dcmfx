import dcmfx_core/data_element_value
import dcmfx_core/data_element_value/person_name.{
  PersonNameComponents, StructuredPersonName,
}
import dcmfx_core/data_set
import dcmfx_core/dictionary
import dcmfx_core/transfer_syntax
import dcmfx_core/value_representation
import dcmfx_json
import dcmfx_json/json_config.{DicomJsonConfig}
import gleam/list
import gleam/option.{None, Some}
import gleeunit
import ieee_float

pub fn main() {
  gleeunit.main()
}

/// Returns pairs of data sets and their corresponding DICOM JSON string. These
/// are used to test conversion both to and from the DICOM JSON.
fn test_data_sets() {
  [
    #(
      [
        #(
          dictionary.manufacturer.tag,
          data_element_value.new_long_string(["123"]),
        ),
        #(
          dictionary.patient_name.tag,
          data_element_value.new_person_name([
            StructuredPersonName(
              Some(PersonNameComponents("Jedi", "Yoda", "", "", "")),
              None,
              None,
            ),
          ]),
        ),
        #(dictionary.patient_sex.tag, data_element_value.new_code_string(["O"])),
      ],
      "{"
        <> "\"00080070\":{\"vr\":\"LO\",\"Value\":[\"123\"]},"
        <> "\"00100010\":{\"vr\":\"PN\",\"Value\":[{\"Alphabetic\":\"Jedi^Yoda\"}]},"
        <> "\"00100040\":{\"vr\":\"CS\",\"Value\":[\"O\"]}"
        <> "}",
    ),
    #(
      [#(dictionary.manufacturer.tag, data_element_value.new_long_string([""]))],
      "{\"00080070\":{\"vr\":\"LO\"}}",
    ),
    #(
      [
        #(
          dictionary.manufacturer.tag,
          data_element_value.new_long_string(["", ""]),
        ),
      ],
      "{\"00080070\":{\"vr\":\"LO\",\"Value\":[null,null]}}",
    ),
    #(
      [
        #(
          dictionary.stage_number.tag,
          data_element_value.new_integer_string([1]),
        ),
      ],
      "{\"00082122\":{\"vr\":\"IS\",\"Value\":[1]}}",
    ),
    #(
      [
        #(
          dictionary.patient_size.tag,
          data_element_value.new_decimal_string([1.2]),
        ),
      ],
      "{\"00101020\":{\"vr\":\"DS\",\"Value\":[1.2]}}",
    ),
    #(
      [
        #(
          dictionary.pixel_data.tag,
          data_element_value.new_other_byte_string(<<1, 2>>),
        ),
      ],
      "{\"7FE00010\":{\"vr\":\"OB\",\"InlineBinary\":\"AQI=\"}}",
    ),
    #(
      [
        #(
          dictionary.pixel_data.tag,
          data_element_value.new_other_word_string(<<0x03, 0x04>>),
        ),
      ],
      "{\"7FE00010\":{\"vr\":\"OW\",\"InlineBinary\":\"AwQ=\"}}",
    ),
    #(
      [
        #(
          dictionary.transfer_syntax_uid.tag,
          data_element_value.new_unique_identifier([
            transfer_syntax.encapsulated_uncompressed_explicit_vr_little_endian.uid,
          ]),
        ),
        #(
          dictionary.pixel_data.tag,
          data_element_value.new_encapsulated_pixel_data(
            value_representation.OtherByteString,
            [<<>>, <<1, 2>>],
          ),
        ),
      ],
      "{"
        <> "\"00020010\":{\"vr\":\"UI\",\"Value\":[\"1.2.840.10008.1.2.1.98\"]},"
        <> "\"7FE00010\":{\"vr\":\"OB\",\"InlineBinary\":\"/v8A4AAAAAD+/wDgAgAAAAEC\"}"
        <> "}",
    ),
    #(
      [
        #(
          dictionary.energy_weighting_factor.tag,
          data_element_value.new_floating_point_single([
            ieee_float.positive_infinity(),
          ]),
        ),
        #(
          dictionary.distance_source_to_isocenter.tag,
          data_element_value.new_floating_point_single([
            ieee_float.negative_infinity(),
          ]),
        ),
        #(
          dictionary.distance_object_to_table_top.tag,
          data_element_value.new_floating_point_single([ieee_float.nan()]),
        ),
      ],
      "{"
        <> "\"00189353\":{\"vr\":\"FL\",\"Value\":[\"Infinity\"]},"
        <> "\"00189402\":{\"vr\":\"FL\",\"Value\":[\"-Infinity\"]},"
        <> "\"00189403\":{\"vr\":\"FL\",\"Value\":[\"NaN\"]}"
        <> "}",
    ),
  ]
  |> list.map(fn(x) {
    let data_elements = x.0
    let json = x.1

    let data_elements =
      list.map(data_elements, fn(data_element) {
        let assert Ok(value) = data_element.1
        #(data_element.0, value)
      })

    #(data_elements, json)
  })
}

pub fn data_set_to_json_test() {
  test_data_sets()
  |> list.each(fn(x) {
    let #(tags, expected_json) = x

    let ds = data_set.from_list(tags)

    let config =
      DicomJsonConfig(store_encapsulated_pixel_data: True, pretty_print: False)

    assert dcmfx_json.data_set_to_json(ds, config) == Ok(expected_json)
  })
}

pub fn json_to_data_set_test() {
  test_data_sets()
  |> list.each(fn(x) {
    let #(tags, expected_json) = x

    let ds = data_set.from_list(tags)

    assert dcmfx_json.json_to_data_set(expected_json) == Ok(ds)
  })
}
