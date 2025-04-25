import dcmfx_p10
import dcmfx_pixel_data
import dcmfx_pixel_data/pixel_data_frame
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{Some}

const input_file = "../../example.dcm"

pub fn main() {
  let assert Ok(ds) = dcmfx_p10.read_file(input_file)
  let assert Ok(frames) = dcmfx_pixel_data.get_pixel_data_frames(ds)

  frames
  |> list.each(fn(frame) {
    let assert Some(frame_index) = pixel_data_frame.index(frame)

    io.println(
      "Frame "
      <> int.to_string(frame_index)
      <> " has size "
      <> int.to_string(pixel_data_frame.length(frame))
      <> " bytes",
    )
  })
}
