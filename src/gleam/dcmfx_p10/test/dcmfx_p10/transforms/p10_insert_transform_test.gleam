import dcmfx_core/data_element_tag.{type DataElementTag, DataElementTag}
import dcmfx_core/data_element_value
import dcmfx_core/data_set
import dcmfx_core/data_set_path
import dcmfx_core/value_representation
import dcmfx_p10/p10_token
import dcmfx_p10/transforms/p10_insert_transform
import gleam/bit_array
import gleam/list
import gleeunit/should

pub fn add_tokens_test() {
  let tx =
    [
      #(DataElementTag(0, 0), data_element_value.new_long_text("00")),
      #(DataElementTag(1, 0), data_element_value.new_long_text("01")),
      #(DataElementTag(3, 0), data_element_value.new_long_text("03")),
      #(DataElementTag(4, 0), data_element_value.new_long_text("04")),
      #(DataElementTag(6, 0), data_element_value.new_long_text("06")),
      #(DataElementTag(7, 0), data_element_value.new_long_text("07")),
    ]
    |> list.map(fn(x) {
      let assert #(tag, Ok(value)) = x
      #(tag, value)
    })
    |> data_set.from_list
    |> p10_insert_transform.new

  let input_tokens =
    list.flatten([
      tokens_for_tag(DataElementTag(2, 0), "12"),
      tokens_for_tag(DataElementTag(5, 0), "15"),
      tokens_for_tag(DataElementTag(6, 0), "16"),
      [p10_token.End],
    ])

  let #(final_tokens, _) =
    input_tokens
    |> list.fold(#([], tx), fn(in, input_token) {
      let #(final_tokens, tx) = in
      let assert Ok(#(new_token, tx)) =
        p10_insert_transform.add_token(tx, input_token)

      #(list.flatten([final_tokens, new_token]), tx)
    })

  final_tokens
  |> should.equal(
    list.flatten([
      tokens_for_tag(DataElementTag(0, 0), "00"),
      tokens_for_tag(DataElementTag(1, 0), "01"),
      tokens_for_tag(DataElementTag(2, 0), "12"),
      tokens_for_tag(DataElementTag(3, 0), "03"),
      tokens_for_tag(DataElementTag(4, 0), "04"),
      tokens_for_tag(DataElementTag(5, 0), "15"),
      tokens_for_tag(DataElementTag(6, 0), "06"),
      tokens_for_tag(DataElementTag(7, 0), "07"),
      [p10_token.End],
    ]),
  )
}

fn tokens_for_tag(tag: DataElementTag, value: String) {
  let value_bytes = value |> bit_array.from_string

  [
    p10_token.DataElementHeader(
      tag,
      value_representation.LongText,
      bit_array.byte_size(value_bytes),
      data_set_path.new_with_data_element(tag),
    ),
    p10_token.DataElementValueBytes(
      tag,
      value_representation.LongText,
      value_bytes,
      0,
    ),
  ]
}
