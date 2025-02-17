import dcmfx_pixel_data/pixel_data_frame
import gleeunit/should

pub fn single_fragment_test() {
  let frame =
    pixel_data_frame.new(0)
    |> pixel_data_frame.push_fragment(<<0, 1, 2>>)

  frame
  |> pixel_data_frame.length
  |> should.equal(3)

  frame
  |> pixel_data_frame.fragments
  |> should.equal([<<0, 1, 2>>])

  frame
  |> pixel_data_frame.to_bytes
  |> should.equal(<<0, 1, 2>>)
}

pub fn multiple_fragments_test() {
  let frame =
    pixel_data_frame.new(0)
    |> pixel_data_frame.push_fragment(<<0, 1>>)
    |> pixel_data_frame.push_fragment(<<5, 6>>)
    |> pixel_data_frame.push_fragment(<<10, 11>>)

  frame
  |> pixel_data_frame.length
  |> should.equal(6)

  frame
  |> pixel_data_frame.fragments
  |> should.equal([<<0, 1>>, <<5, 6>>, <<10, 11>>])

  frame
  |> pixel_data_frame.to_bytes
  |> should.equal(<<0, 1, 5, 6, 10, 11>>)
}

pub fn drop_end_bytes_test() {
  let frame =
    pixel_data_frame.new(0)
    |> pixel_data_frame.push_fragment(<<0, 1, 2, 3, 4>>)

  frame
  |> pixel_data_frame.drop_end_bytes(2)
  |> pixel_data_frame.to_bytes()
  |> should.equal(<<0, 1, 2>>)

  let frame =
    pixel_data_frame.new(0)
    |> pixel_data_frame.push_fragment(<<0, 1>>)
    |> pixel_data_frame.push_fragment(<<2, 3>>)

  frame
  |> pixel_data_frame.drop_end_bytes(1)
  |> pixel_data_frame.to_bytes()
  |> should.equal(<<0, 1, 2>>)

  let frame =
    pixel_data_frame.new(0)
    |> pixel_data_frame.push_fragment(<<0, 1>>)
    |> pixel_data_frame.push_fragment(<<2, 3>>)
    |> pixel_data_frame.push_fragment(<<4, 5>>)

  frame
  |> pixel_data_frame.drop_end_bytes(2)
  |> pixel_data_frame.to_bytes()
  |> should.equal(<<0, 1, 2, 3>>)
}
