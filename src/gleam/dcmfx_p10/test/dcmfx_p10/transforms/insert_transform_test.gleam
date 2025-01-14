import dcmfx_core/data_element_tag.{type DataElementTag, DataElementTag}
import dcmfx_core/data_element_value
import dcmfx_core/data_set
import dcmfx_core/value_representation
import dcmfx_p10/p10_part
import dcmfx_p10/transforms/p10_insert_transform
import gleam/bit_array
import gleam/int
import gleam/list
import gleeunit/should

pub fn add_parts_test() {
  let tx =
    [
      #(DataElementTag(0, 0), data_element_value.new_long_text("0")),
      #(DataElementTag(1, 0), data_element_value.new_long_text("1")),
      #(DataElementTag(3, 0), data_element_value.new_long_text("3")),
      #(DataElementTag(4, 0), data_element_value.new_long_text("4")),
      #(DataElementTag(6, 0), data_element_value.new_long_text("6")),
    ]
    |> list.map(fn(x) {
      let assert #(tag, Ok(value)) = x
      #(tag, value)
    })
    |> data_set.from_list
    |> p10_insert_transform.new

  let input_parts =
    list.flatten([
      parts_for_tag(DataElementTag(2, 0)),
      parts_for_tag(DataElementTag(5, 0)),
      [p10_part.End],
    ])

  let #(final_parts, _) =
    input_parts
    |> list.fold(#([], tx), fn(in, input_part) {
      let #(final_parts, tx) = in
      let #(new_parts, tx) = p10_insert_transform.add_part(tx, input_part)

      #(list.flatten([final_parts, new_parts]), tx)
    })

  final_parts
  |> should.equal(
    list.flatten([
      parts_for_tag(DataElementTag(0, 0)),
      parts_for_tag(DataElementTag(1, 0)),
      parts_for_tag(DataElementTag(2, 0)),
      parts_for_tag(DataElementTag(3, 0)),
      parts_for_tag(DataElementTag(4, 0)),
      parts_for_tag(DataElementTag(5, 0)),
      parts_for_tag(DataElementTag(6, 0)),
      [p10_part.End],
    ]),
  )
}

fn parts_for_tag(tag: DataElementTag) {
  let value_bytes = { int.to_string(tag.group) <> " " } |> bit_array.from_string

  [
    p10_part.DataElementHeader(
      tag,
      value_representation.LongText,
      bit_array.byte_size(value_bytes),
    ),
    p10_part.DataElementValueBytes(
      value_representation.LongText,
      value_bytes,
      0,
    ),
  ]
}
