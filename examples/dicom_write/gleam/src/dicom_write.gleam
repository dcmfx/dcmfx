import dcmfx_p10
import gleam/option.{None}

const input_file = "../../example.dcm"

const output_file = "output.dcm"

pub fn main() {
  let assert Ok(ds) = dcmfx_p10.read_file(input_file, None)
  let assert Ok(Nil) = dcmfx_p10.write_file(output_file, ds, None)
}
