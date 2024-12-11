use dcmfx::p10::*;
use std::fs::File;

const INPUT_FILE: &str = "../../example.dcm";
const OUTPUT_FILE: &str = "output.dcm";

pub fn main() -> Result<(), P10Error> {
    let mut input_stream = File::open(INPUT_FILE).unwrap();
    let mut output_stream = File::create(OUTPUT_FILE).unwrap();

    let mut read_context = P10ReadContext::new();
    let mut write_context = P10WriteContext::new();

    loop {
        let parts = dcmfx::p10::read_parts_from_stream(
            &mut input_stream,
            &mut read_context,
        )?;

        let ended = dcmfx::p10::write_parts_to_stream(
            &parts,
            &mut output_stream,
            &mut write_context,
        )?;

        if ended {
            break;
        }
    }

    Ok(())
}