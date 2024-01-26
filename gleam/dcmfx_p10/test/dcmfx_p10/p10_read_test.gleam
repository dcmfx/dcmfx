import dcmfx_core/data_element_value
import dcmfx_core/data_set
import dcmfx_core/registry
import dcmfx_core/value_representation
import dcmfx_p10/p10_part
import dcmfx_p10/p10_read
import gleam/bit_array
import gleam/list
import gleeunit/should

pub fn read_file_meta_information_test() {
  let preamble_bytes = list.repeat(<<0x03>>, 128) |> bit_array.concat

  let file_meta_information_bytes =
    bit_array.concat([
      <<2:16-little, 1:16-little, "OB", 0:16-little, 2:32-little, 0, 1>>,
      <<2:16-little, 2:16-little, "UI", 4:16-little, "1.23">>,
    ])

  let file_meta_information_length =
    bit_array.byte_size(file_meta_information_bytes)

  let file_meta_information_length_bytes = <<
    2:16-little,
    0:16,
    "UL",
    4:16-little,
    file_meta_information_length:32-little,
  >>

  let assert Ok(context) =
    p10_read.new_read_context()
    |> p10_read.write_bytes(
      bit_array.concat([
        preamble_bytes,
        <<"DICM">>,
        file_meta_information_length_bytes,
        file_meta_information_bytes,
      ]),
      True,
    )

  let assert Ok(#(parts, context)) = p10_read.read_parts(context)

  parts
  |> should.equal([p10_part.FilePreambleAndDICMPrefix(preamble_bytes)])

  let assert Ok(#(parts, context)) = p10_read.read_parts(context)

  parts
  |> should.equal([
    p10_part.FileMetaInformation(
      data_set.new()
      |> data_set.insert(
        registry.file_meta_information_version.tag,
        data_element_value.new_binary_unchecked(
          value_representation.OtherByteString,
          <<0, 1>>,
        ),
      )
      |> data_set.insert(
        registry.media_storage_sop_class_uid.tag,
        data_element_value.new_binary_unchecked(
          value_representation.UniqueIdentifier,
          <<"1.23">>,
        ),
      ),
    ),
  ])

  let assert Ok(#(parts, _)) = p10_read.read_parts(context)

  parts
  |> should.equal([p10_part.End])
}
