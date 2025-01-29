import dcmfx_core/data_element_tag.{type DataElementTag, DataElementTag}
import dcmfx_core/data_element_value
import dcmfx_core/data_set
import dcmfx_core/value_representation
import dcmfx_p10/p10_token
import dcmfx_p10/transforms/p10_insert_transform
import gleam/bit_array
import gleam/int
import gleam/list
import gleeunit/should

pub fn add_tokens_test() {
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

  let input_tokens =
    list.flatten([
      tokens_for_tag(DataElementTag(2, 0)),
      tokens_for_tag(DataElementTag(5, 0)),
      [p10_token.End],
    ])

  let #(final_tokens, _) =
    input_tokens
    |> list.fold(#([], tx), fn(in, input_token) {
      let #(final_tokens, tx) = in
      let #(new_token, tx) = p10_insert_transform.add_token(tx, input_token)

      #(list.flatten([final_tokens, new_token]), tx)
    })

  final_tokens
  |> should.equal(
    list.flatten([
      tokens_for_tag(DataElementTag(0, 0)),
      tokens_for_tag(DataElementTag(1, 0)),
      tokens_for_tag(DataElementTag(2, 0)),
      tokens_for_tag(DataElementTag(3, 0)),
      tokens_for_tag(DataElementTag(4, 0)),
      tokens_for_tag(DataElementTag(5, 0)),
      tokens_for_tag(DataElementTag(6, 0)),
      [p10_token.End],
    ]),
  )
}

fn tokens_for_tag(tag: DataElementTag) {
  let value_bytes = { int.to_string(tag.group) <> " " } |> bit_array.from_string

  [
    p10_token.DataElementHeader(
      tag,
      value_representation.LongText,
      bit_array.byte_size(value_bytes),
    ),
    p10_token.DataElementValueBytes(
      value_representation.LongText,
      value_bytes,
      0,
    ),
  ]
}
