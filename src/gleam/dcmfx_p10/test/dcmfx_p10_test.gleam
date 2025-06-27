import dcmfx_core/data_set
import dcmfx_core/dictionary
import dcmfx_p10
import gleam/option.{None}
import gleeunit

pub fn main() {
  gleeunit.main()
}

pub fn read_file_partial_test() {
  let path = "../../../test/assets/pydicom/test_files/693_J2KI.dcm"

  let assert Ok(ds) =
    dcmfx_p10.read_file_partial(
      path,
      [dictionary.rows.tag, dictionary.columns.tag],
      None,
    )

  assert data_set.tags(ds) == [dictionary.rows.tag, dictionary.columns.tag]
}
