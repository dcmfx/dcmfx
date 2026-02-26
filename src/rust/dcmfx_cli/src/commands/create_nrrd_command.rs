use std::sync::Arc;

use clap::Args;
use tokio::sync::Mutex;

use dcmfx::{
  core::*,
  p10::*,
  pixel_data::iods::{ImagePixelModule, ImagePlaneModule},
};

use crate::utils::{self, InputSource};

pub const ABOUT: &str = "Converts the pixel data in a DICOM series into a \
  single NRRD file";

#[derive(Args)]
pub struct CreateNRRDArgs {
  #[arg(
    long,
    help = "The number of concurrent tasks to use. Defaults to the number of CPU
      cores.",
    default_value_t = {num_cpus::get()}
  )]
  concurrency: usize,

  #[command(flatten)]
  input: crate::args::input_args::P10InputArgs,

  #[arg(
    long,
    help = "The Series Instance UID to convert to an NRRD file. Only input \
      files with this Series Instance UID will be part of the NRRD conversion. \
      Other DICOMs will be silently ignored."
  )]
  series_instance_uid: String,
}

#[derive(Debug)]
enum CreateNRRDError {
  P10Error(P10Error),
  DataError(DataError),
}

pub async fn run(args: CreateNRRDArgs) -> Result<(), ()> {
  



  let mut sop_instances = series_sop_instances.lock().await;

  // Sort by the slice location
  sop_instances.sort_by(|a, b| {
    a.slice_location().partial_cmp(&b.slice_location()).unwrap()
  });

  // Write the NRRD data

  Ok(())
}

#[derive(Debug)]
struct SeriesSOPInstance {
  input_source: InputSource,
  image_pixel_module: ImagePixelModule,
  image_plane_module: ImagePlaneModule,
}

impl SeriesSOPInstance {
  fn slice_location(&self) -> f32 {
    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
      [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
      ]
    }

    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
      a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    let orientation = self.image_plane_module.image_orientation_patient;
    let position = self.image_plane_module.image_position_patient;

    let row = [orientation[0], orientation[1], orientation[2]];
    let col = [orientation[3], orientation[4], orientation[5]];
    let normal = cross(row, col);

    dot(position, normal)
  }
}

async fn read_series_sop_instances(args: &CreateNRRDArgs) -> Result<Vec<SeriesSOPInstance>, ()>{

  let input_sources = args.input.base.input_sources().await;

  let series_sop_instances: Arc<Mutex<Vec<SeriesSOPInstance>>> =
    Arc::new(Mutex::new(vec![]));

  // Get
  let get_details_result =
    utils::run_tasks(args.concurrency, input_sources, async |input_source| {
      match read_input_source_details(&input_source, &args.series_instance_uid)
        .await
      {
        Ok(Some(details)) => series_sop_instances.lock().await.push(details),

        Ok(None) => (),

        Err(CreateNRRDError::P10Error(P10Error::DicmPrefixNotPresent))
          if args.input.ignore_invalid =>
        {
          ()
        }

        Err(e) => return Err((e, input_source)),
      };

      Ok(())
    })
    .await;

  match get_details_result {
    Ok(()) => (),

    Err((e, input_source)) => {
      let task_description = format!("converting \"{input_source}\"");

      match e {
        CreateNRRDError::P10Error(e) => e.print(&task_description),
        CreateNRRDError::DataError(e) => e.print(&task_description),
      };

      return Err(());
    }
  };

  series_sop_instances.lock().await
}

async fn read_input_source_details(
  input_source: &InputSource,
  required_series_instance_uid: &str,
) -> Result<Option<SeriesSOPInstance>, CreateNRRDError> {
  let mut stream = input_source
    .open_read_stream()
    .await
    .map_err(CreateNRRDError::P10Error)?;

  let mut tags = ImagePixelModule::TAGS.to_vec();
  tags.extend_from_slice(&ImagePlaneModule::TAGS);
  tags.push(dictionary::SERIES_INSTANCE_UID.tag);

  let dataset = dcmfx::p10::read_stream_partial_async(&mut stream, &tags, None)
    .await
    .map_err(CreateNRRDError::P10Error)?;

  match dataset.get_string(dictionary::SERIES_INSTANCE_UID.tag) {
    Ok(uid) => {
      if uid != required_series_instance_uid {
        return Ok(None);
      }
    }

    Err(_) => return Ok(None),
  }

  let image_pixel_module = ImagePixelModule::from_data_set(&dataset)
    .map_err(CreateNRRDError::DataError)?;
  let image_plane_module = ImagePlaneModule::from_data_set(&dataset)
    .map_err(CreateNRRDError::DataError)?;

  Ok(Some(SeriesSOPInstance {
    input_source: input_source.clone(),
    image_pixel_module,
    image_plane_module,
  }))
}
