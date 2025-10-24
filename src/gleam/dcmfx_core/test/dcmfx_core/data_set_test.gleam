import dcmfx_core/data_element_tag.{DataElementTag}
import dcmfx_core/data_element_value
import dcmfx_core/data_error
import dcmfx_core/data_set
import dcmfx_core/data_set_path
import dcmfx_core/value_representation

const tag_1 = DataElementTag(1, 2)

const tag_2 = DataElementTag(3, 4)

const tag_3 = DataElementTag(3, 5)

fn person_name_value() {
  data_element_value.new_binary_unchecked(value_representation.PersonName, <<
    "Jedi^Yoda",
  >>)
}

fn long_string_value() {
  data_element_value.new_binary_unchecked(value_representation.LongString, <<
    "123",
  >>)
}

fn code_string_value() {
  data_element_value.new_binary_unchecked(value_representation.CodeString, <<
    "O",
  >>)
}

pub fn has_test() {
  let ds =
    data_set.from_list([
      #(tag_1, person_name_value()),
      #(tag_2, long_string_value()),
      #(tag_3, code_string_value()),
    ])

  assert data_set.has(ds, DataElementTag(3, 4))
  assert !data_set.has(ds, DataElementTag(3, 6))
}

pub fn get_test() {
  let ds =
    data_set.from_list([
      #(tag_1, person_name_value()),
      #(tag_2, long_string_value()),
    ])

  assert data_set.get_value(ds, tag_2) == Ok(long_string_value())

  assert data_set.get_value(ds, tag_3)
    == Error(
      data_error.new_tag_not_present()
      |> data_error.with_path(data_set_path.new_with_data_element(tag_3)),
    )
}

pub fn tags_test() {
  let ds =
    data_set.from_list([
      #(tag_2, long_string_value()),
      #(tag_1, person_name_value()),
    ])

  assert data_set.tags(ds) == [tag_1, tag_2]
}

pub fn fold_test() {
  let ds =
    data_set.from_list([
      #(tag_1, long_string_value()),
      #(tag_2, long_string_value()),
    ])

  assert data_set.fold(ds, "", fn(a, tag, _value) {
      let assert Ok(s) = data_set.get_string(ds, tag)
      a <> s
    })
    == "123123"
}

pub fn partition_test() {
  let ds =
    data_set.from_list([
      #(tag_1, long_string_value()),
      #(tag_2, long_string_value()),
      #(tag_3, long_string_value()),
    ])

  assert data_set.partition(ds, fn(tag) {
      data_element_tag.to_int(tag) < data_element_tag.to_int(tag_3)
    })
    == #(
      data_set.from_list([
        #(tag_1, long_string_value()),
        #(tag_2, long_string_value()),
      ]),
      data_set.from_list([#(tag_3, long_string_value())]),
    )
}
