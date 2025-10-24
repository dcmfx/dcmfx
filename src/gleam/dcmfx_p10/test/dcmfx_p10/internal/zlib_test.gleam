import dcmfx_p10/internal/zlib
import dcmfx_p10/internal/zlib/flush_command
import dcmfx_p10/internal/zlib/inflate_result
import gleam/bit_array
import gleam/list

const window_bits = -15

fn zeros_16_kib() {
  list.repeat(<<0>>, 16 * 1024)
  |> bit_array.concat
}

fn zeros_256_kib() {
  list.repeat(zeros_16_kib(), 16)
  |> bit_array.concat
}

const zeros_256_kib_deflated = <<
  237, 193, 49, 1, 0, 0, 0, 194, 160, 245, 79, 237, 109, 7, 160, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 222, 0,
>>

pub fn deflate_test() {
  let stream = zlib.open()
  zlib.deflate_init(stream, 9, zlib.Deflated, window_bits, 8, zlib.Default)
  assert zlib.deflate(stream, zeros_256_kib(), flush_command.Finish)
    == [zeros_256_kib_deflated]
}

pub fn inflate_test() {
  let stream = zlib.open()
  zlib.inflate_init(stream, window_bits)
  assert zlib.safe_inflate(stream, zeros_256_kib_deflated)
    == Ok(inflate_result.Continue(zeros_16_kib()))

  list.range(0, 14)
  |> list.each(fn(_) {
    assert zlib.safe_inflate(stream, <<>>)
      == Ok(inflate_result.Continue(zeros_16_kib()))
  })

  assert zlib.safe_inflate(stream, <<>>) == Ok(inflate_result.Finished(<<>>))
}
