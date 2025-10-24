import dcmfx_character_set/internal/jis_x_0201
import gleam/list
import gleam/string

pub fn decode_next_codepoint_test() {
  [
    #(<<0x3D>>, 0x003D),
    #(<<0xB3>>, 0xFF73),
    #(<<0xC0>>, 0xFF80),
    #(<<0xCF>>, 0xFF8F),
    #(<<0xD4>>, 0xFF94),
    #(<<0xDB>>, 0xFF9B),
    #(<<0xDE>>, 0xFF9E),
  ]
  |> list.each(fn(x) {
    let #(bytes, expected_codepoint) = x

    let assert Ok(expected_codepoint) = string.utf_codepoint(expected_codepoint)

    assert jis_x_0201.decode_next_codepoint(bytes)
      == Ok(#(expected_codepoint, <<>>))
  })

  assert jis_x_0201.decode_next_codepoint(<<>>) == Error(Nil)
}
