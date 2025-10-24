import dcmfx_core/data_element_tag.{DataElementTag}
import dcmfx_core/data_error
import dcmfx_core/data_set_path
import dcmfx_core/value_representation
import gleam/string

pub fn to_lines_test() {
  assert string.join(
      data_error.to_lines(
        data_error.new_tag_not_present()
          |> data_error.with_path(
            data_set_path.new_with_data_element(DataElementTag(0x1000, 0x2000)),
          ),
        "testing",
      ),
      "\n",
    )
    == "DICOM data error testing

  Error: Tag not present
  Tag: (1000,2000)
  Name: Escape Triplet
  Path: 10002000"

  assert string.join(
      data_error.to_lines(data_error.new_value_not_present(), "testing"),
      "\n",
    )
    == "DICOM data error testing

  Error: Value not present"

  assert string.join(
      data_error.to_lines(data_error.new_multiplicity_mismatch(), "testing"),
      "\n",
    )
    == "DICOM data error testing

  Error: Multiplicity mismatch"

  assert string.join(
      data_error.to_lines(data_error.new_value_invalid("123"), "testing"),
      "\n",
    )
    == "DICOM data error testing

  Error: Invalid value
  Details: 123"

  assert string.join(
      data_error.to_lines(
        data_error.new_value_length_invalid(
          value_representation.AgeString,
          5,
          "Test 123",
        ),
        "testing",
      ),
      "\n",
    )
    == "DICOM data error testing

  Error: Invalid value length
  VR: AS
  Length: 5 bytes
  Details: Test 123"
}
