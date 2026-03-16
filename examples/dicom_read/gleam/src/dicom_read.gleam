import dcmfx_core/data_set
import dcmfx_core/dictionary
import dcmfx_p10
import gleam/io
import gleam/option.{None}
import gleam/string

const input_file = "../../example.dcm"

pub fn main() {
  let assert Ok(ds) = dcmfx_p10.read_file(input_file, None)
  data_set.print(ds)

  let assert Ok(patient_id) = data_set.get_string(ds, dictionary.patient_id.tag)
  io.println("Patient ID: " <> patient_id)

  let assert Ok(study_date) = data_set.get_date(ds, dictionary.study_date.tag)
  io.println("Study Date: " <> string.inspect(study_date))
}
