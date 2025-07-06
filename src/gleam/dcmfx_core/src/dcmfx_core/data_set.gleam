//// A DICOM data set, defined as a map of data element tags to data element
//// values.

import bigi.{type BigInt}
import dcmfx_core/data_element_tag.{type DataElementTag, DataElementTag}
import dcmfx_core/data_element_value.{type DataElementValue}
import dcmfx_core/data_element_value/age_string
import dcmfx_core/data_element_value/date
import dcmfx_core/data_element_value/date_time
import dcmfx_core/data_element_value/person_name
import dcmfx_core/data_element_value/time
import dcmfx_core/data_error.{type DataError}
import dcmfx_core/data_set_path.{type DataSetPath}
import dcmfx_core/data_set_print.{type DataSetPrintOptions}
import dcmfx_core/dictionary
import dcmfx_core/internal/utils
import dcmfx_core/transfer_syntax.{type TransferSyntax}
import dcmfx_core/value_multiplicity
import dcmfx_core/value_representation.{type ValueRepresentation}
import gleam/bit_array
import gleam/bool
import gleam/dict.{type Dict}
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/order
import gleam/result
import gleam/string
import ieee_float.{type IEEEFloat}

/// A DICOM data set that is a mapping of data element tags to data element
/// values.
///
pub type DataSet =
  Dict(DataElementTag, DataElementValue)

/// Returns a new empty data set.
///
pub fn new() -> DataSet {
  dict.new()
}

/// Returns the number of data elements in a data set.
///
pub fn size(data_set: DataSet) -> Int {
  data_set
  |> dict.size
}

/// Returns whether a data set is empty and contains no data elements.
///
pub fn is_empty(data_set: DataSet) -> Bool {
  data_set
  |> dict.is_empty
}

/// Returns whether a data element with the specified tag exists in a data set.
///
pub fn has(data_set: DataSet, tag: DataElementTag) -> Bool {
  data_set
  |> dict.has_key(tag)
}

/// The successful result of looking up a `DataSetPath` in a data set. Depending
/// on the path, the result will either be a specific data element value, or a
/// specific sequence item in a sequence (i.e. a nested data set).
///
@internal
pub type DataSetLookupResult {
  LookupResultDataElementValue(DataElementValue)
  LookupResultDataSet(DataSet)
}

/// Returns a new data set containing the File Meta Information data elements
/// in this data set, i.e. those where the data element tag group equals 2.
///
/// This function also sets the *'(0002,0002) Media Storage SOP Class UID'* and
/// *'(0002,0003) Media Storage SOP Instance UID'* data elements to match the
/// *'(0008,0016) SOP Class UID'* and *'(0008,0018) SOP Instance UID'* data
/// elements in this data set.
///
pub fn file_meta_information(data_set: DataSet) -> DataSet {
  let file_meta_information =
    data_set
    |> dict.filter(fn(tag, _value) { tag.group == 2 })

  let file_meta_information = case
    get_value(data_set, dictionary.sop_class_uid.tag),
    get_string(data_set, dictionary.sop_class_uid.tag)
  {
    Ok(value), Ok(_) ->
      dict.insert(
        file_meta_information,
        dictionary.media_storage_sop_class_uid.tag,
        value,
      )
    _, _ -> file_meta_information
  }

  let file_meta_information = case
    get_value(data_set, dictionary.sop_instance_uid.tag),
    get_string(data_set, dictionary.sop_instance_uid.tag)
  {
    Ok(value), Ok(_) ->
      dict.insert(
        file_meta_information,
        dictionary.media_storage_sop_instance_uid.tag,
        value,
      )
    _, _ -> file_meta_information
  }

  // Exclude sequences and encapsulated pixel data, as they aren't allowed in
  // File Meta Information
  file_meta_information
  |> dict.filter(fn(_tag, value) {
    result.is_ok(data_element_value.bytes(value))
  })
}

/// Inserts a data element tag and value into a data set. If there is already a
/// value for the tag then it is replaced with the new value.
///
pub fn insert(
  data_set: DataSet,
  tag: DataElementTag,
  value: DataElementValue,
) -> DataSet {
  dict.insert(data_set, tag, value)
}

/// Inserts a new binary value into a data set. If there is already a value for
/// the tag it is replaced with the new value.
///
pub fn insert_binary_value(
  data_set: DataSet,
  tag: DataElementTag,
  vr: ValueRepresentation,
  bytes: BitArray,
) -> Result(DataSet, DataError) {
  let value =
    data_element_value.new_binary(vr, bytes)
    |> result.map_error(fn(e) {
      data_error.with_path(e, data_set_path.new_with_data_element(tag))
    })
  use value <- result.map(value)

  insert(data_set, tag, value)
}

/// Inserts a data element with an age string value into a data set. The data
/// element being inserted must be referenced through its dictionary entry.
///
pub fn insert_age_string(
  data_set: DataSet,
  item: dictionary.Item,
  value: age_string.StructuredAge,
) -> Result(DataSet, DataError) {
  use <- bool.lazy_guard(
    !value_multiplicity.contains(item.multiplicity, 1),
    fn() { invalid_insert_error(item) },
  )

  let value =
    case item.vrs {
      [value_representation.AgeString] ->
        data_element_value.new_age_string(value)
      _ -> invalid_insert_error(item)
    }
    |> result.map_error(fn(e) {
      data_error.with_path(e, data_set_path.new_with_data_element(item.tag))
    })
  use value <- result.map(value)

  insert(data_set, item.tag, value)
}

/// Inserts a data element with an attribute tag value into a data set. The data
/// element being inserted must be referenced through its dictionary entry.
///
pub fn insert_attribute_tag_value(
  data_set: DataSet,
  item: dictionary.Item,
  value: List(DataElementTag),
) -> Result(DataSet, DataError) {
  use <- bool.lazy_guard(
    !value_multiplicity.contains(item.multiplicity, list.length(value)),
    fn() { invalid_insert_error(item) },
  )

  let value =
    case item.vrs {
      [value_representation.AttributeTag] ->
        data_element_value.new_attribute_tag(value)
      _ -> invalid_insert_error(item)
    }
    |> result.map_error(fn(e) {
      data_error.with_path(e, data_set_path.new_with_data_element(item.tag))
    })
  use value <- result.map(value)

  insert(data_set, item.tag, value)
}

/// Inserts a data element with a date value into a data set. The data element
/// being inserted must be referenced through its dictionary entry.
///
pub fn insert_date_value(
  data_set: DataSet,
  item: dictionary.Item,
  value: date.StructuredDate,
) -> Result(DataSet, DataError) {
  use <- bool.lazy_guard(
    !value_multiplicity.contains(item.multiplicity, 1),
    fn() { invalid_insert_error(item) },
  )

  let value =
    case item.vrs {
      [value_representation.Date] -> data_element_value.new_date(value)
      _ -> invalid_insert_error(item)
    }
    |> result.map_error(fn(e) {
      data_error.with_path(e, data_set_path.new_with_data_element(item.tag))
    })
  use value <- result.map(value)

  insert(data_set, item.tag, value)
}

/// Inserts a data element with a date time value into a data set. The data
/// element being inserted must be referenced through its dictionary entry.
///
pub fn insert_date_time_value(
  data_set: DataSet,
  item: dictionary.Item,
  value: date_time.StructuredDateTime,
) -> Result(DataSet, DataError) {
  use <- bool.lazy_guard(
    !value_multiplicity.contains(item.multiplicity, 1),
    fn() { invalid_insert_error(item) },
  )

  let value =
    case item.vrs {
      [value_representation.Date] -> data_element_value.new_date_time(value)
      _ -> invalid_insert_error(item)
    }
    |> result.map_error(fn(e) {
      data_error.with_path(e, data_set_path.new_with_data_element(item.tag))
    })
  use value <- result.map(value)

  insert(data_set, item.tag, value)
}

/// Inserts a data element with float values into a data set. The data element
/// being inserted must be referenced through its dictionary entry. This method
/// automatically determines the correct VR to use for the new data element.
///
pub fn insert_float_value(
  data_set: DataSet,
  item: dictionary.Item,
  value: List(IEEEFloat),
) -> Result(DataSet, DataError) {
  use <- bool.lazy_guard(
    !value_multiplicity.contains(item.multiplicity, list.length(value)),
    fn() { invalid_insert_error(item) },
  )

  let value =
    case item.vrs {
      [value_representation.DecimalString] ->
        value
        |> list.map(ieee_float.to_finite)
        |> result.all
        |> result.replace_error(data_error.new_value_invalid(
          "DecimalString float value was not finite",
        ))
        |> result.try(data_element_value.new_decimal_string)

      [value_representation.FloatingPointDouble] ->
        data_element_value.new_floating_point_double(value)
      [value_representation.FloatingPointSingle] ->
        data_element_value.new_floating_point_single(value)
      [value_representation.OtherDoubleString] ->
        data_element_value.new_other_double_string(value)
      [value_representation.OtherFloatString] ->
        data_element_value.new_other_float_string(value)

      _ -> invalid_insert_error(item)
    }
    |> result.map_error(fn(e) {
      data_error.with_path(e, data_set_path.new_with_data_element(item.tag))
    })
  use value <- result.map(value)

  insert(data_set, item.tag, value)
}

/// Inserts a data element with integer values into a data set. The data
/// element being inserted must be referenced through its dictionary entry. This
/// method automatically determines the correct VR to use for the new data
/// element.
///
pub fn insert_int_value(
  data_set: DataSet,
  item: dictionary.Item,
  value: List(Int),
) -> Result(DataSet, DataError) {
  use <- bool.lazy_guard(
    !value_multiplicity.contains(item.multiplicity, list.length(value)),
    fn() { invalid_insert_error(item) },
  )

  let value =
    case item.vrs {
      [value_representation.IntegerString] ->
        data_element_value.new_integer_string(value)
      [value_representation.SignedLong] ->
        data_element_value.new_signed_long(value)
      [value_representation.SignedShort] ->
        data_element_value.new_signed_short(value)
      [value_representation.UnsignedLong] ->
        data_element_value.new_unsigned_long(value)
      [value_representation.UnsignedShort] ->
        data_element_value.new_unsigned_short(value)

      _ -> invalid_insert_error(item)
    }
    |> result.map_error(fn(e) {
      data_error.with_path(e, data_set_path.new_with_data_element(item.tag))
    })
  use value <- result.map(value)

  insert(data_set, item.tag, value)
}

/// Inserts a data element with big integer values into a data set. The data
/// element being inserted must be referenced through its dictionary entry. This
/// method automatically determines the correct VR to use for the new data
/// element.
///
pub fn insert_big_int_value(
  data_set: DataSet,
  item: dictionary.Item,
  value: List(BigInt),
) -> Result(DataSet, DataError) {
  use <- bool.lazy_guard(
    !value_multiplicity.contains(item.multiplicity, list.length(value)),
    fn() { invalid_insert_error(item) },
  )

  let value =
    case item.vrs {
      [value_representation.SignedVeryLong] ->
        data_element_value.new_signed_very_long(value)
      [value_representation.UnsignedVeryLong] ->
        data_element_value.new_unsigned_very_long(value)

      _ -> invalid_insert_error(item)
    }
    |> result.map_error(fn(e) {
      data_error.with_path(e, data_set_path.new_with_data_element(item.tag))
    })
  use value <- result.map(value)

  insert(data_set, item.tag, value)
}

/// Inserts a data element with a person name value into a data set. The data
/// element being inserted must be referenced through its dictionary entry.
///
pub fn insert_person_name_value(
  data_set: DataSet,
  item: dictionary.Item,
  value: List(person_name.StructuredPersonName),
) -> Result(DataSet, DataError) {
  use <- bool.lazy_guard(
    !value_multiplicity.contains(item.multiplicity, list.length(value)),
    fn() { invalid_insert_error(item) },
  )

  let value =
    case item.vrs {
      [value_representation.PersonName] ->
        data_element_value.new_person_name(value)
      _ -> invalid_insert_error(item)
    }
    |> result.map_error(fn(e) {
      data_error.with_path(e, data_set_path.new_with_data_element(item.tag))
    })
  use value <- result.map(value)

  insert(data_set, item.tag, value)
}

/// Inserts a data element with a sequence value into a data set. The data
/// element being inserted must be referenced through its dictionary entry.
///
pub fn insert_sequence(
  data_set: DataSet,
  item: dictionary.Item,
  value: List(DataSet),
) -> Result(DataSet, DataError) {
  let value =
    case item.vrs {
      [value_representation.Sequence] ->
        Ok(data_element_value.new_sequence(value))
      _ -> invalid_insert_error(item)
    }
    |> result.map_error(fn(e) {
      data_error.with_path(e, data_set_path.new_with_data_element(item.tag))
    })
  use value <- result.map(value)

  insert(data_set, item.tag, value)
}

/// Inserts a data element with a string value into a data set. The data
/// element being inserted must be referenced through its dictionary entry. This
/// method automatically determines the correct VR to use for the new data
/// element.
///
pub fn insert_string_value(
  data_set: DataSet,
  item: dictionary.Item,
  value: List(String),
) -> Result(DataSet, DataError) {
  use <- bool.lazy_guard(
    !value_multiplicity.contains(item.multiplicity, list.length(value)),
    fn() { invalid_insert_error(item) },
  )

  let value =
    case item.vrs, value {
      [value_representation.ApplicationEntity], [value] ->
        data_element_value.new_application_entity(value)
      [value_representation.CodeString], _ ->
        data_element_value.new_code_string(value)
      [value_representation.LongString], _ ->
        data_element_value.new_long_string(value)
      [value_representation.LongText], [value] ->
        data_element_value.new_long_text(value)
      [value_representation.ShortString], _ ->
        data_element_value.new_short_string(value)
      [value_representation.ShortText], [value] ->
        data_element_value.new_short_text(value)
      [value_representation.UniqueIdentifier], _ ->
        data_element_value.new_unique_identifier(value)
      [value_representation.UniversalResourceIdentifier], [value] ->
        data_element_value.new_universal_resource_identifier(value)
      [value_representation.UnlimitedCharacters], _ ->
        data_element_value.new_unlimited_characters(value)
      [value_representation.UnlimitedText], [value] ->
        data_element_value.new_unlimited_text(value)

      _, _ -> invalid_insert_error(item)
    }
    |> result.map_error(fn(e) {
      data_error.with_path(e, data_set_path.new_with_data_element(item.tag))
    })
  use value <- result.map(value)

  insert(data_set, item.tag, value)
}

/// Inserts a data element with a time value into a data set. The data element
/// being inserted must be referenced through its dictionary entry.
///
pub fn insert_time_value(
  data_set: DataSet,
  item: dictionary.Item,
  value: time.StructuredTime,
) -> Result(DataSet, DataError) {
  use <- bool.lazy_guard(
    !value_multiplicity.contains(item.multiplicity, 1),
    fn() { invalid_insert_error(item) },
  )

  let value =
    case item.vrs {
      [value_representation.Time] -> data_element_value.new_time(value)
      _ -> invalid_insert_error(item)
    }
    |> result.map_error(fn(e) {
      data_error.with_path(e, data_set_path.new_with_data_element(item.tag))
    })
  use value <- result.map(value)

  insert(data_set, item.tag, value)
}

/// Merges two data sets together to form a new data set. Data elements from the
/// second data set take precedence.
///
pub fn merge(a: DataSet, b: DataSet) -> DataSet {
  dict.merge(a, b)
}

/// Converts a data set to a list of data element tags and values.
///
pub fn to_list(data_set: DataSet) -> List(#(DataElementTag, DataElementValue)) {
  data_set
  |> tags
  |> list.map(fn(tag) {
    let assert Ok(value) = dict.get(data_set, tag)
    #(tag, value)
  })
}

/// Creates a data set from a list of data element tags and values.
///
pub fn from_list(
  data_elements: List(#(DataElementTag, DataElementValue)),
) -> DataSet {
  data_elements
  |> list.fold(new(), fn(data_set, element) {
    let #(tag, value) = element

    insert(data_set, tag, value)
  })
}

/// Creates a new data set with all the data elements in the given data set
/// except for the specified tag. Also returns the deleted data element value,
/// if any.
///
pub fn delete(
  data_set: DataSet,
  tag: DataElementTag,
) -> #(Option(DataElementValue), DataSet) {
  let deleted_value =
    dict.get(data_set, tag)
    |> option.from_result

  let data_set = dict.delete(data_set, tag)

  #(deleted_value, data_set)
}

/// Returns the tags in a data set, sorted by group and element.
///
pub fn tags(data_set: DataSet) -> List(DataElementTag) {
  data_set
  |> dict.keys
  |> list.sort(fn(a, b) {
    case int.compare(a.group, b.group) {
      order.Eq -> int.compare(a.element, b.element)
      o -> o
    }
  })
}

/// Maps the tags and values in a data set to a new list, in order of increasing
/// tag value.
///
pub fn map(
  data_set: DataSet,
  callback: fn(DataElementTag, DataElementValue) -> a,
) -> List(a) {
  data_set
  |> tags
  |> list.map(fn(tag) {
    let assert Ok(value) = dict.get(data_set, tag)

    callback(tag, value)
  })
}

/// Maps the values in a data set to a new data set.
///
pub fn map_values(
  data_set: DataSet,
  callback: fn(DataElementTag, DataElementValue) -> DataElementValue,
) -> DataSet {
  dict.map_values(data_set, callback)
}

/// Creates a new data set containing only those data elements for which the
/// given function returns `True`.
///
pub fn filter(
  data_set: DataSet,
  predicate: fn(DataElementTag, DataElementValue) -> Bool,
) -> DataSet {
  data_set
  |> dict.filter(predicate)
}

/// Folds the tags and values in a data set, in order of increasing tag value,
/// into a single value.
///
pub fn fold(
  data_set: DataSet,
  initial: a,
  callback: fn(a, DataElementTag, DataElementValue) -> a,
) -> a {
  data_set
  |> tags
  |> list.fold(initial, fn(current, tag) {
    let assert Ok(value) = dict.get(data_set, tag)

    callback(current, tag, value)
  })
}

/// Folds the tags and values in a data set, in order of increasing tag value,
/// into a single value. If the folding function returns `Ok(..)` then folding
/// continues, and if it returns `Error(..)` then folding stops and the error is
/// returned.
///
pub fn try_fold(
  data_set: DataSet,
  initial: a,
  callback: fn(a, DataElementTag, DataElementValue) -> Result(a, b),
) -> Result(a, b) {
  data_set
  |> tags
  |> list.try_fold(initial, fn(current, tag) {
    let assert Ok(value) = dict.get(data_set, tag)

    callback(current, tag, value)
  })
}

/// Folds the tags and values in a data set, in order of increasing tag value,
/// into a single value.
///
/// This variant of `fold()` allows for folding to be ended early by returning
/// `Stop` from the callback.
///
pub fn fold_until(
  data_set: DataSet,
  initial: a,
  callback: fn(a, DataElementTag, DataElementValue) -> list.ContinueOrStop(a),
) -> a {
  data_set
  |> tags
  |> list.fold_until(initial, fn(current, tag) {
    let assert Ok(value) = dict.get(data_set, tag)

    callback(current, tag, value)
  })
}

/// Partitions a data set into a pair of data sets by a given categorization
/// function.
///
pub fn partition(
  data_set: DataSet,
  predicate: fn(DataElementTag) -> Bool,
) -> #(DataSet, DataSet) {
  data_set
  |> dict.fold(#(new(), new()), fn(current, tag, value) {
    case predicate(tag) {
      True -> #(insert(current.0, tag, value), current.1)
      False -> #(current.0, insert(current.1, tag, value))
    }
  })
}

/// Prints a data set to stdout formatted for readability.
///
pub fn print(data_set: DataSet) -> Nil {
  print_with_options(data_set, data_set_print.new_print_options())
}

/// Prints a data set to stdout formatted for readability using the given print
/// options.
///
pub fn print_with_options(
  data_set: DataSet,
  print_options: DataSetPrintOptions,
) -> Nil {
  to_lines(data_set, print_options, Nil, fn(_, s) { io.println(s) })
}

/// Converts a data set to a list of printable lines using the specified print
/// options. The lines are returned via a callback.
///
pub fn to_lines(
  data_set: DataSet,
  print_options: DataSetPrintOptions,
  context: a,
  callback: fn(a, String) -> a,
) -> a {
  do_to_lines(data_set, print_options, context, callback, 0)
}

fn do_to_lines(
  data_set: DataSet,
  print_options: DataSetPrintOptions,
  context: a,
  callback: fn(a, String) -> a,
  indent: Int,
) -> a {
  data_set
  |> tags
  |> list.fold(context, fn(context, tag) {
    let assert Ok(value) = get_value(data_set, tag)

    let #(header, header_width) =
      data_set_print.format_data_element_prefix(
        tag,
        tag_name(data_set, tag),
        Some(data_element_value.value_representation(value)),
        data_element_value.bytes(value)
          |> result.map(bit_array.byte_size)
          |> option.from_result,
        indent,
        print_options,
      )

    // For sequences, recursively print their items
    case
      data_element_value.sequence_items(value),
      data_element_value.encapsulated_pixel_data(value)
    {
      Ok(items), _ -> {
        let context = callback(context, header)

        let context =
          items
          |> list.fold(context, fn(context, item) {
            let context =
              callback(
                context,
                data_set_print.format_data_element_prefix(
                  dictionary.item.tag,
                  dictionary.item.name,
                  None,
                  None,
                  indent + 1,
                  print_options,
                ).0,
              )

            let context =
              do_to_lines(item, print_options, context, callback, indent + 2)

            callback(
              context,
              data_set_print.format_data_element_prefix(
                dictionary.item_delimitation_item.tag,
                dictionary.item_delimitation_item.name,
                None,
                None,
                indent + 1,
                print_options,
              ).0,
            )
          })

        callback(
          context,
          data_set_print.format_data_element_prefix(
            dictionary.sequence_delimitation_item.tag,
            dictionary.sequence_delimitation_item.name,
            None,
            None,
            indent,
            print_options,
          ).0,
        )
      }

      _, Ok(items) -> {
        let context = callback(context, header)

        let context =
          items
          |> list.fold(context, fn(context, item) {
            let #(item_header, item_header_width) =
              data_set_print.format_data_element_prefix(
                dictionary.item.tag,
                dictionary.item.name,
                None,
                Some(bit_array.byte_size(item)),
                indent + 1,
                print_options,
              )

            let value_max_width =
              int.max(print_options.max_width - item_header_width, 10)

            callback(
              context,
              item_header
                <> utils.inspect_bit_array(item, value_max_width / 3 - 1),
            )
          })

        callback(
          context,
          data_set_print.format_data_element_prefix(
            dictionary.sequence_delimitation_item.tag,
            dictionary.sequence_delimitation_item.name,
            None,
            None,
            indent,
            print_options,
          ).0,
        )
      }

      _, _ -> {
        let value_max_width =
          int.max(print_options.max_width - header_width, 10)

        callback(
          context,
          header <> data_element_value.to_string(value, tag, value_max_width),
        )
      }
    }
  })
}

/// Looks up a data set path in a data set and returns the data element or
/// data set that it specifies. If the path is invalid for the data set then
/// an error is returned.
///
fn lookup(
  data_set: DataSet,
  path: DataSetPath,
) -> Result(DataSetLookupResult, DataError) {
  let create_error = fn() {
    Error(data_error.new_tag_not_present() |> data_error.with_path(path))
  }

  path
  |> data_set_path.entries
  |> list.reverse
  |> list.try_fold(LookupResultDataSet(data_set), fn(lookup_result, entry) {
    case lookup_result {
      LookupResultDataElementValue(value) ->
        case entry {
          data_set_path.SequenceItem(index) ->
            case data_element_value.sequence_items(value) {
              Ok(items) ->
                case utils.list_at(items, index) {
                  Ok(data_set) -> Ok(LookupResultDataSet(data_set))
                  Error(Nil) -> create_error()
                }
              Error(_) -> create_error()
            }
          _ -> create_error()
        }

      LookupResultDataSet(data_set) ->
        case entry {
          data_set_path.DataElement(tag) ->
            case dict.get(data_set, tag) {
              Ok(value) -> Ok(LookupResultDataElementValue(value))
              Error(Nil) -> create_error()
            }
          _ -> create_error()
        }
    }
  })
}

/// Returns the data element value for the specified tag in a data set.
///
pub fn get_value(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(DataElementValue, DataError) {
  dict.get(data_set, tag)
  |> result.map_error(fn(_) {
    data_error.new_tag_not_present()
    |> data_error.with_path(data_set_path.new_with_data_element(tag))
  })
}

/// Returns the data element value at the specified path in a data set. The
/// path must end with a data element tag.
///
pub fn get_value_at_path(
  data_set: DataSet,
  path: DataSetPath,
) -> Result(DataElementValue, DataError) {
  case lookup(data_set, path) {
    Ok(LookupResultDataElementValue(value)) -> Ok(value)
    _ ->
      data_error.new_tag_not_present()
      |> data_error.with_path(path)
      |> Error
  }
}

/// Returns the data set at the specified path in a data set. The path must
/// be empty or end with a sequence item index.
///
pub fn get_data_set_at_path(
  data_set: DataSet,
  path: DataSetPath,
) -> Result(DataSet, DataError) {
  case lookup(data_set, path) {
    Ok(LookupResultDataSet(data_set)) -> Ok(data_set)
    _ ->
      data_error.new_tag_not_present()
      |> data_error.with_path(path)
      |> Error
  }
}

/// Returns the raw value bytes for the specified tag in a data set.
///
/// See `data_element_value.bytes()`.
///
pub fn get_value_bytes(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(BitArray, DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.bytes)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the raw value bytes for the specified tag in a data set and also
/// checks that its value representation is one of the specified allowed VRs.
///
/// See `data_element_value.vr_bytes()`.
///
pub fn get_value_vr_bytes(
  data_set: DataSet,
  tag: DataElementTag,
  allowed_vrs: List(ValueRepresentation),
) -> Result(BitArray, DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.vr_bytes(_, allowed_vrs))
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the singular string value for a data element in a data set. If the
/// data element with the specified tag does not hold exactly one string value
/// then an error is returned.
///
pub fn get_string(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(String, DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_string)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns all of the string values for a data element in a data set. If the
/// data element with the specified tag is not of a type that supports multiple
/// string values then an error is returned.
///
pub fn get_strings(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(List(String), DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_strings)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the singular integer value for a data element in a data set. If the
/// data element with the specified tag does not hold exactly one integer value
/// then an error is returned.
///
pub fn get_int(data_set: DataSet, tag: DataElementTag) -> Result(Int, DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_int)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the singular integer value for a data element in a data set. If the
/// data element with the specified tag does not hold exactly one integer value
/// then an error is returned.
///
/// If the data element is not in the data set then the specified default value
/// is returned.
///
pub fn get_int_with_default(
  data_set: DataSet,
  tag: DataElementTag,
  default: Int,
) -> Result(Int, DataError) {
  case has(data_set, tag) {
    True -> get_int(data_set, tag)
    False -> Ok(default)
  }
}

/// Returns all of the integer values for a data element in a data set. If the
/// data element with the specified tag is not of a type that supports multiple
/// integer values then an error is returned.
///
pub fn get_ints(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(List(Int), DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_ints)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the lookup table descriptor value for a data element in a data set.
/// If the data element with the specified tab does not hold a lookup table
/// descriptor then an error is returned.
///
pub fn get_lookup_table_descriptor(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(#(Int, Int, Int), DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_lookup_table_descriptor)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the singular big integer value for a data element in a data set. If
/// the data element with the specified tag does not hold exactly one big
/// integer value then an error is returned.
///
pub fn get_big_int(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(BigInt, DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_big_int)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns all of the big integer values for a data element in a data set. If
/// the data element with the specified tag is not of a type that supports
/// multiple big integer values then an error is returned.
///
pub fn get_big_ints(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(List(BigInt), DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_big_ints)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the singular floating point value for a data element in a data set.
/// If the data element with the specified tag does not hold exactly one
/// floating point value then an error is returned.
///
pub fn get_float(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(IEEEFloat, DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_float)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the singular floating point value for a data element in a data set.
/// If the data element with the specified tag does not hold exactly one
/// floating point value then an error is returned.
///
/// If the data element is not in the data set then the specified default value
/// is returned.
///
pub fn get_float_with_default(
  data_set: DataSet,
  tag: DataElementTag,
  default: Float,
) -> Result(IEEEFloat, DataError) {
  case has(data_set, tag) {
    True -> get_float(data_set, tag)
    False -> Ok(ieee_float.finite(default))
  }
}

/// Returns all of the floating point values for a data element in a data set.
/// If the data element with the specified tag is not of a type that supports
/// multiple floating point values then an error is returned.
///
pub fn get_floats(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(List(IEEEFloat), DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_floats)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the age value for a data element in a data set. If the data element
/// does not hold an `AgeString` value then an error is returned.
///
pub fn get_age(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(age_string.StructuredAge, DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_age)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the attribute tags value for a data element in a data set. If the
/// data element does not hold an `AttributeTag` value then an error is
/// returned.
///
pub fn get_attribute_tags(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(List(DataElementTag), DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_attribute_tags)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the date value for a data element in a data set. If the data element
/// does not hold a `Date` value then an error is returned.
///
pub fn get_date(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(date.StructuredDate, DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_date)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the structured date/time value for a data element in a data set. If
/// the data element does not hold a `DateTime` value then an error is returned.
///
pub fn get_date_time(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(date_time.StructuredDateTime, DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_date_time)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the time value for a data element in a data set. If the data element
/// does not hold a `Time` value then an error is returned.
///
pub fn get_time(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(time.StructuredTime, DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_time)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the singular person name value for a data element in a data set.
/// If the data element with the specified tag does not hold exactly one
/// person name value then an error is returned.
///
pub fn get_person_name(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(person_name.StructuredPersonName, DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_person_name)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns all of the person name values for a data element in a data set. If
/// the data element with the specified tag is not of a type that supports
/// multiple person name values then an error is returned.
///
pub fn get_person_names(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(List(person_name.StructuredPersonName), DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.get_person_names)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Returns the sequence items for a data element in a data set. If the data
/// element with the specified tag is not a sequence then an error is
/// returned.
///
pub fn get_sequence_items(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(List(DataSet), DataError) {
  data_set
  |> get_value(tag)
  |> result.try(data_element_value.sequence_items)
  |> result.map_error(data_error.with_path(
    _,
    data_set_path.new_with_data_element(tag),
  ))
}

/// Looks up the *'(0002,0010) Transfer Syntax UID'* data element in a data set,
/// and if present, attempts to convert it to a known transfer syntax
/// definition.
///
pub fn get_transfer_syntax(
  data_set: DataSet,
) -> Result(TransferSyntax, DataError) {
  let transfer_syntax_uid =
    get_string(data_set, dictionary.transfer_syntax_uid.tag)
  use transfer_syntax_uid <- result.try(transfer_syntax_uid)

  transfer_syntax.from_uid(transfer_syntax_uid)
  |> result.map_error(fn(_) {
    data_error.new_value_invalid(
      "Unrecognized transfer syntax UID: '" <> transfer_syntax_uid <> "'",
    )
  })
}

/// Returns the size in bytes of all data elements in a data set.
///
/// See `data_element_value.total_byte_size()`.
///
pub fn total_byte_size(data_set: DataSet) -> Int {
  data_set
  |> dict.fold(0, fn(total, _tag, value) {
    total + data_element_value.total_byte_size(value)
  })
}

/// Returns the human-readable name for a data element tag in a data set,
/// using its data elements to determine the private creator if the tag is
/// private.
///
pub fn tag_name(data_set: DataSet, tag: DataElementTag) -> String {
  let private_creator =
    data_set
    |> private_creator_for_tag(tag)
    |> option.from_result

  dictionary.tag_name(tag, private_creator)
}

/// Formats a data element tag in a data set as `"(GROUP,ELEMENT) TAG_NAME"`,
/// e.g. "(0008,0020) StudyDate"`. The other data elements in the data set
/// are used to determine the private creator if the tag is private.
///
pub fn tag_with_name(data_set: DataSet, tag: DataElementTag) -> String {
  let private_creator =
    data_set
    |> private_creator_for_tag(tag)
    |> option.from_result

  dictionary.tag_with_name(tag, private_creator)
}

/// Returns the value of the `(gggg,00xx) Private Creator` data element in this
/// data set for the specified private tag.
///
pub fn private_creator_for_tag(
  data_set: DataSet,
  tag: DataElementTag,
) -> Result(String, Nil) {
  use <- bool.guard(!data_element_tag.is_private(tag), Error(Nil))

  let private_creator_tag =
    DataElementTag(tag.group, int.bitwise_shift_right(tag.element, 8))

  use <- bool.guard(
    private_creator_tag.element < 0x10 || private_creator_tag.element > 0xFF,
    Error(Nil),
  )

  case get_string(data_set, private_creator_tag) {
    Ok(s) -> Ok(s)
    Error(_) -> Error(Nil)
  }
}

/// Removes all private range tags from a data set, including recursively
/// into any sequences that are present.
///
pub fn delete_private_elements(data_set: DataSet) -> DataSet {
  data_set
  |> fold(new(), fn(current, tag, value) {
    use <- bool.guard(data_element_tag.is_private(tag), current)

    let value = case data_element_value.sequence_items(value) {
      Ok(items) ->
        items
        |> list.map(delete_private_elements)
        |> data_element_value.new_sequence
      _ -> value
    }

    insert(current, tag, value)
  })
}

/// Returns a new data set containing just the private tags for the given group
/// and private creator name in a data set. The group number must always be odd
/// for private data elements, and the private creator name must match exactly.
///
/// If the group number is even or there is no `(gggg,00XX) Private Creator`
/// data element with the specified name then an error is returned.
///
pub fn private_block(
  data_set: DataSet,
  group: Int,
  private_creator: String,
) -> Result(DataSet, String) {
  use <- bool.guard(int.is_even(group), Error("Private group number is even"))

  let private_creator_value =
    data_element_value.new_long_string([private_creator])
    |> result.replace_error("Private creator name is invalid")
  use private_creator_value <- result.try(private_creator_value)

  // Search for a matching `(gggg,00XX) Private Creator' data element.
  // Ref: PS3.5 7.8.1.
  let private_creator_element =
    list.range(0x10, 0xFF)
    |> list.find(fn(element) {
      dict.get(data_set, DataElementTag(group, element))
      == Ok(private_creator_value)
    })
    |> result.replace_error(
      "Private creator '" <> private_creator <> "' not found",
    )
  use private_creator_element <- result.map(private_creator_element)

  // Calculate the range of element values to include in the returned data set
  let element_start = int.bitwise_shift_left(private_creator_element, 8)
  let element_end = element_start + 0xFF

  // Filter this data set to only include the relevant private data elements
  data_set
  |> fold(new(), fn(current, tag, value) {
    case
      tag.group == group
      && tag.element >= element_start
      && tag.element <= element_end
    {
      True -> insert(current, tag, value)
      False -> current
    }
  })
}

/// Helper function that returns an error message when one of the
/// `insert_*_element` functions is called with invalid arguments.
///
fn invalid_insert_error(item: dictionary.Item) -> Result(a, DataError) {
  case item.vrs {
    [vr] ->
      Error(data_error.new_value_invalid(
        "Data element '"
        <> item.name
        <> "' does not support the provided data, its VR is "
        <> value_representation.to_string(vr)
        <> " with multiplicity "
        <> value_multiplicity.to_string(item.multiplicity),
      ))

    vrs ->
      Error(data_error.new_value_invalid(
        "Data element '"
        <> item.name
        <> "' supports multiple VRs: "
        <> vrs |> list.map(value_representation.to_string) |> string.join(", "),
      ))
  }
}
