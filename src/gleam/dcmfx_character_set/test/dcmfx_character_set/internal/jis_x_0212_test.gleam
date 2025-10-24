import dcmfx_character_set/internal/jis_x_0212
import gleam/list
import gleam/string

pub fn decode_next_codepoint_test() {
  [
    #(<<0x20>>, 0x0020),
    #(<<0x22, 0x2F>>, 0x02D8),
    #(<<0x33, 0x58>>, 0x529C),
    #(<<0x51, 0x4A>>, 0x7A60),
    #(<<0x57, 0x5A>>, 0x82F7),
    #(<<0x61, 0x4F>>, 0x9018),
    #(<<0x6D, 0x63>>, 0x9FA5),
    #(<<0x6D, 0x64>>, 0xFFFD),
  ]
  |> list.each(fn(x) {
    let #(bytes, expected_codepoint) = x

    let assert Ok(expected_codepoint) = string.utf_codepoint(expected_codepoint)

    assert jis_x_0212.decode_next_codepoint(bytes)
      == Ok(#(expected_codepoint, <<>>))
  })

  assert jis_x_0212.decode_next_codepoint(<<>>) == Error(Nil)
}
