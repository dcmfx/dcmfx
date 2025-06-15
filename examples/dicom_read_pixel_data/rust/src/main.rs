use dcmfx::core::*;
use dcmfx::p10::*;
use dcmfx::pixel_data::*;

const INPUT_FILE: &str = "./example.dcm";

pub fn main() {
    let ds = DataSet::read_p10_file(INPUT_FILE).unwrap();
    let frames = ds.get_pixel_data_frames().unwrap();

    let pixel_data_renderer =
        PixelDataRenderer::from_data_set(&ds).unwrap();

    for mut frame in frames {
        println!(
            "Frame {} has size {} bytes",
            frame.index().unwrap(),
            frame.len()
        );

        // Render raw frame data into an image::RgbImage
        let frame_image =
            pixel_data_renderer.render_frame(&mut frame, None).unwrap();

        // Open output PNG file
        let output_filename =
            format!("frame.{}.png", frame.index().unwrap());
        let mut output_file =
            std::fs::File::create(&output_filename).unwrap();

        // Write frame as PNG
        frame_image
            .write_to(&mut output_file, image::ImageFormat::Png)
            .unwrap();

        println!("Wrote \"{}\"", output_filename);
    }
}
