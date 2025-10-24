import dcmfx_pixel_data/pixel_data_frame

pub fn single_fragment_test() {
  let frame =
    pixel_data_frame.new()
    |> pixel_data_frame.push_chunk(<<0, 1, 2>>)

  assert pixel_data_frame.length(frame) == 3

  assert pixel_data_frame.chunks(frame) == [<<0, 1, 2>>]

  assert pixel_data_frame.to_bytes(frame) == <<0, 1, 2>>
}

pub fn multiple_fragments_test() {
  let frame =
    pixel_data_frame.new()
    |> pixel_data_frame.push_chunk(<<0, 1>>)
    |> pixel_data_frame.push_chunk(<<5, 6>>)
    |> pixel_data_frame.push_chunk(<<10, 11>>)

  assert pixel_data_frame.length(frame) == 6

  assert pixel_data_frame.chunks(frame) == [<<0, 1>>, <<5, 6>>, <<10, 11>>]

  assert pixel_data_frame.to_bytes(frame) == <<0, 1, 5, 6, 10, 11>>
}

pub fn drop_end_bytes_test() {
  let frame =
    pixel_data_frame.new()
    |> pixel_data_frame.push_chunk(<<0, 1, 2, 3, 4>>)

  assert frame
    |> pixel_data_frame.drop_end_bytes(2)
    |> pixel_data_frame.to_bytes()
    == <<0, 1, 2>>

  let frame =
    pixel_data_frame.new()
    |> pixel_data_frame.push_chunk(<<0, 1>>)
    |> pixel_data_frame.push_chunk(<<2, 3>>)

  assert frame
    |> pixel_data_frame.drop_end_bytes(1)
    |> pixel_data_frame.to_bytes()
    == <<0, 1, 2>>

  let frame =
    pixel_data_frame.new()
    |> pixel_data_frame.push_chunk(<<0, 1>>)
    |> pixel_data_frame.push_chunk(<<2, 3>>)
    |> pixel_data_frame.push_chunk(<<4, 5>>)

  assert frame
    |> pixel_data_frame.drop_end_bytes(2)
    |> pixel_data_frame.to_bytes()
    == <<0, 1, 2, 3>>
}
