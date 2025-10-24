import dcmfx_character_set/internal/utf8
import gleam/list
import gleam/string

pub fn decode_next_codepoint_test() {
  [
    #(<<0x20>>, 0x0020),
    #(<<0xC2, 0xA3>>, 0x00A3),
    #(<<0xD0, 0x98>>, 0x0418),
    #(<<0xE0, 0xA4, 0xB9>>, 0x0939),
    #(<<0xE2, 0x82, 0xAC>>, 0x20AC),
    #(<<0xED, 0x95, 0x9C>>, 0xD55C),
    #(<<0xF0, 0x90, 0x8D, 0x88>>, 0x10348),
    #(<<0xF0>>, 0xFFFD),
  ]
  |> list.each(fn(x) {
    let #(bytes, expected_codepoint) = x

    let assert Ok(expected_codepoint) = string.utf_codepoint(expected_codepoint)

    assert utf8.decode_next_codepoint(bytes) == Ok(#(expected_codepoint, <<>>))
  })

  assert utf8.decode_next_codepoint(<<>>) == Error(Nil)
}
