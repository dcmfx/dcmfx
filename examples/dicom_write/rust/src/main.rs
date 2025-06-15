use dcmfx::core::*;
use dcmfx::p10::*;

const INPUT_FILE: &str = "./example.dcm";
const OUTPUT_FILE: &str = "output.dcm";

pub fn main() {
    let ds = DataSet::read_p10_file(INPUT_FILE).unwrap();
    ds.write_p10_file(OUTPUT_FILE, None).unwrap();
}
