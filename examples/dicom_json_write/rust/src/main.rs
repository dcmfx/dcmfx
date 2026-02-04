use dcmfx::core::*;
use dcmfx::json::*;
use dcmfx::p10::*;

const INPUT_FILE: &str = "./example.dcm";

pub fn main() {
    let ds = DataSet::read_p10_file(INPUT_FILE).unwrap();

    let json_config = DicomJsonConfig {
        store_encapsulated_pixel_data: true,
        pretty_print: true,
        emit_binary_data_values: None,
    };

    let ds_json = ds.to_json(json_config).unwrap();
    println!("{ds_json}");
}
