use dcmfx::anonymize::*;
use dcmfx::core::*;
use dcmfx::p10::*;

const INPUT_FILE: &str = "./example.dcm";

pub fn main() {
    let mut ds = DataSet::read_p10_file(INPUT_FILE).unwrap();

    ds.anonymize();
    ds.print();
}
