import dcmfx_anonymize
import dcmfx_core/data_set
import dcmfx_p10

const input_file = "../../example.dcm"

pub fn main() {
  let assert Ok(ds) = dcmfx_p10.read_file(input_file)

  ds
  |> dcmfx_anonymize.anonymize_data_set
  |> data_set.print
}
