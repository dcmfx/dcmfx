use dcmfx::core::*;
use dcmfx::deidentify::*;
use dcmfx::p10::*;

const INPUT_FILE: &str = "./example.dcm";

pub fn main() {
    let mut ds = DataSet::read_p10_file(INPUT_FILE, None).unwrap();

    ds.deidentify(&Deidentifier::new()).unwrap();
    ds.print();
}
