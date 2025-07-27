use dcmfx::p10::*;
use std::fs::File;

const INPUT_FILE: &str = "./example.dcm";
const OUTPUT_FILE: &str = "output.dcm";

pub fn main() -> Result<(), P10Error> {
    let mut input_stream = File::open(INPUT_FILE).unwrap();
    let mut output_stream = File::create(OUTPUT_FILE).unwrap();

    let mut read_context = P10ReadContext::new(None);
    let mut write_context = P10WriteContext::new(None);

    loop {
        let tokens = dcmfx::p10::read_tokens_from_stream(
            &mut input_stream,
            &mut read_context,
            None,
        )?;

        let ended = dcmfx::p10::write_tokens_to_stream(
            &tokens,
            &mut output_stream,
            &mut write_context,
        )?;

        if ended {
            break;
        }
    }

    Ok(())
}
