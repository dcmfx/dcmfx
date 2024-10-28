import dcmfx_json
import dcmfx_json/json_config.{DicomJsonConfig}
import dcmfx_p10
import gleam/io

const input_file = "../../example.dcm"

pub fn main() {
  let assert Ok(ds) = dcmfx_p10.read_file(input_file)

  let json_config = DicomJsonConfig(store_encapsulated_pixel_data: True)

  let assert Ok(ds_json) = dcmfx_json.data_set_to_json(ds, json_config)
  io.println(ds_json)
}
