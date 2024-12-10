//// Converts between DICOM data sets and DICOM JSON.
////
//// Ref: PS3.18 F.

import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/data_set_path
import dcmfx_json/internal/json_to_data_set
import dcmfx_json/json_config.{type DicomJsonConfig}
import dcmfx_json/json_error.{type JsonDeserializeError, type JsonSerializeError}
import dcmfx_json/transforms/p10_json_transform
import dcmfx_p10/p10_write
import gleam/dynamic
import gleam/json
import gleam/pair
import gleam/result

/// Converts a data set to DICOM JSON.
///
pub fn data_set_to_json(
  data_set: DataSet,
  config: DicomJsonConfig,
) -> Result(String, JsonSerializeError) {
  let transform = p10_json_transform.new(config)

  let context = #(transform, "")

  p10_write.data_set_to_parts(data_set, context, fn(context, part) {
    let #(transform, json) = context
    use #(transform, new_json) <- result.map(p10_json_transform.add_part(
      transform,
      part,
    ))

    #(transform, json <> new_json)
  })
  |> result.map(pair.second)
}

/// Converts DICOM JSON data into a data set.
///
pub fn json_to_data_set(
  data_set_json: String,
) -> Result(DataSet, JsonDeserializeError) {
  let data_set_json =
    json.decode(data_set_json, dynamic.dynamic)
    |> result.replace_error(json_error.JsonInvalid(
      "Input is not valid JSON",
      path: data_set_path.new(),
    ))
  use data_set_json <- result.try(data_set_json)

  json_to_data_set.convert_json_to_data_set(data_set_json, data_set_path.new())
}
