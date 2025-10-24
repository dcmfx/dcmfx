import dcmfx_character_set/internal/jis_x_0208
import gleam/list
import gleam/string

pub fn decode_next_codepoint_test() {
  [
    #(<<0x20>>, 0x0020),
    #(<<0x21>>, 0xFFFD),
    #(<<0x21, 0x21>>, 0x3000),
    #(<<0x21, 0x6F>>, 0xFFE5),
    #(<<0x24, 0x5D>>, 0x307D),
    #(<<0x3B, 0x33>>, 0x5C71),
    #(<<0x45, 0x44>>, 0x7530),
    #(<<0x74, 0x26>>, 0x7199),
  ]
  |> list.each(fn(x) {
    let #(bytes, expected_codepoint) = x

    let assert Ok(expected_codepoint) = string.utf_codepoint(expected_codepoint)

    assert jis_x_0208.decode_next_codepoint(bytes)
      == Ok(#(expected_codepoint, <<>>))
  })

  assert jis_x_0208.decode_next_codepoint(<<>>) == Error(Nil)
}
