use dcmfx::core::*;
use dcmfx::json::*;

const INPUT_FILE: &str = "./example.dcm.json";

pub fn main() {
    let json_data = std::fs::read_to_string(INPUT_FILE).unwrap();
    let ds = DataSet::from_json(&json_data).unwrap();
    ds.print();
}
