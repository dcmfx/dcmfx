use dcmfx::core::*;
use dcmfx::p10::*;
use dcmfx::pixel_data::*;

const INPUT_FILE: &str = "../../example.dcm";

pub fn main() {
    let ds = DataSet::read_p10_file(INPUT_FILE).unwrap();
    let frames = ds.get_pixel_data_frames().unwrap();

    for frame in frames {
        println!("Frame with size: {}", frame.len());
    }
}
