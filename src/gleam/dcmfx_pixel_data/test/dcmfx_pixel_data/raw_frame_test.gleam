import dcmfx_pixel_data/raw_frame
import gleeunit/should

pub fn single_fragment_test() {
  let frame =
    raw_frame.new()
    |> raw_frame.push_fragment(<<0, 1, 2>>)

  frame
  |> raw_frame.length
  |> should.equal(3)

  frame
  |> raw_frame.fragments
  |> should.equal([<<0, 1, 2>>])

  frame
  |> raw_frame.to_bytes
  |> should.equal(<<0, 1, 2>>)
}

pub fn multiple_fragments_test() {
  let frame =
    raw_frame.new()
    |> raw_frame.push_fragment(<<0, 1>>)
    |> raw_frame.push_fragment(<<5, 6>>)
    |> raw_frame.push_fragment(<<10, 11>>)

  frame
  |> raw_frame.length
  |> should.equal(6)

  frame
  |> raw_frame.fragments
  |> should.equal([<<0, 1>>, <<5, 6>>, <<10, 11>>])

  frame
  |> raw_frame.to_bytes
  |> should.equal(<<0, 1, 5, 6, 10, 11>>)
}

pub fn drop_end_bytes_test() {
  let frame =
    raw_frame.new()
    |> raw_frame.push_fragment(<<0, 1, 2, 3, 4>>)

  frame
  |> raw_frame.drop_end_bytes(2)
  |> raw_frame.to_bytes()
  |> should.equal(<<0, 1, 2>>)

  let frame =
    raw_frame.new()
    |> raw_frame.push_fragment(<<0, 1>>)
    |> raw_frame.push_fragment(<<2, 3>>)

  frame
  |> raw_frame.drop_end_bytes(1)
  |> raw_frame.to_bytes()
  |> should.equal(<<0, 1, 2>>)

  let frame =
    raw_frame.new()
    |> raw_frame.push_fragment(<<0, 1>>)
    |> raw_frame.push_fragment(<<2, 3>>)
    |> raw_frame.push_fragment(<<4, 5>>)

  frame
  |> raw_frame.drop_end_bytes(2)
  |> raw_frame.to_bytes()
  |> should.equal(<<0, 1, 2, 3>>)
}
