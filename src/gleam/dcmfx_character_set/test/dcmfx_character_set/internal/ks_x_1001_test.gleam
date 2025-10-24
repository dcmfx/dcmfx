import dcmfx_character_set/internal/ks_x_1001
import gleam/list
import gleam/string

pub fn decode_next_codepoint_test() {
  [
    #(<<0xA1, 0xA1>>, 0x3000),
    #(<<0xA1, 0xA2>>, 0x3001),
    #(<<0xA1, 0xF7>>, 0x2287),
    #(<<0xB0, 0xA1>>, 0xAC00),
    #(<<0xC8, 0xFE>>, 0xD79D),
    #(<<0xFD, 0xFE>>, 0x8A70),
    #(<<0xA1, 0xFF>>, 0xFFFD),
  ]
  |> list.each(fn(x) {
    let #(bytes, expected_codepoint) = x

    let assert Ok(expected_codepoint) = string.utf_codepoint(expected_codepoint)

    let assert Ok(r) =
      bytes
      |> ks_x_1001.decode_next_codepoint

    assert r.0 == expected_codepoint
  })

  assert ks_x_1001.decode_next_codepoint(<<>>) == Error(Nil)
}
