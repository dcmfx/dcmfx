import gleam/bool
import gleam/dict.{type Dict}
import gleam/dynamic.{field, string}
import gleam/int
import gleam/io
import gleam/json
import gleam/list
import gleam/set
import gleam/string
import simplifile

pub fn main() {
  let registry_entries = read_attributes_json()
  let private_tags = read_private_tags_json()

  // Generate code and print it to stdout
  generate_constants(registry_entries)
  generate_find_function(registry_entries)
  generate_find_private_function(private_tags)
  generate_uid_name_function()
}

type RegistryItem {
  RegistryItem(
    tag: String,
    name: String,
    keyword: String,
    value_representation: String,
    value_multiplicity: String,
  )
}

/// Reads the raw DICOM attributes file sourced from Innolitics and prepares it
/// for code generation.
///
/// The latest file can be downloaded here:
/// https://raw.githubusercontent.com/innolitics/dicom-standard/master/standard/attributes.json
///
fn read_attributes_json() -> List(RegistryItem) {
  let assert Ok(attributes_json) =
    simplifile.read("data/innolitics_attributes.json")

  // Decode the JSON content
  let registry_entries_decoder =
    dynamic.list(of: dynamic.decode5(
      RegistryItem,
      field("tag", of: string),
      field("name", of: string),
      field("keyword", of: string),
      field("valueRepresentation", of: string),
      field("valueMultiplicity", of: string),
    ))

  let assert Ok(registry_entries) =
    json.decode(from: attributes_json, using: registry_entries_decoder)

  // Filter out entries that have no name, keyword, or VR
  let registry_entries =
    registry_entries
    |> list.filter(fn(attribute) {
      attribute.name != ""
      && attribute.keyword != ""
      && attribute.value_representation != ""
    })

  // Change keyword to use snake case. Some keywords require manual adjustment.
  let registry_entries =
    registry_entries
    |> list.map(fn(attribute) {
      let keyword =
        case attribute.keyword {
          "MRFOV" <> k -> "MR_FOV" <> k
          k -> k
        }
        |> string.replace("IDs", "Ids")
        |> string.replace("ExposureInmAa", "InMilliampSeconds")
        |> string.replace("ExposureInuAs", "ExposureInMicroampSeconds")
        |> string.replace("ExposureTimeInuS", "ExposureTimeInMicroseconds")
        |> string.replace("XRayTubeCurrentInmA", "XRayTubeCurrentInMilliamps")
        |> string.replace("XRayTubeCurrentInuA", "XRayTubeCurrentInMicroamps")
        |> string.replace("Latitudeyyy", "LatitudeYyy")
        |> string.replace("Latitudezzz", "LatitudeZzz")
        |> string.replace("CTDIvol", "CTDIVol")
        |> string.replace("CTXRay", "CT_XRay")
        |> string.replace("DVHROI", "DVH_ROI")
        |> insert_underscores_in_keyword("")
        |> string.lowercase

      RegistryItem(..attribute, keyword: keyword)
    })

  // Sort by tag
  registry_entries
  |> list.sort(fn(a, b) { string.compare(a.tag, b.tag) })
}

/// Inserts underscores into a keyword name between words, e.g. "PatientName"
/// becomes "Patient_Name".
///
fn insert_underscores_in_keyword(keyword: String, acc: String) -> String {
  let length = string.length(keyword)

  use <- bool.guard(length < 2, acc <> keyword)

  let a = string.slice(keyword, 0, 1)
  let b = string.slice(keyword, 1, 1)

  case is_uppercase(a), is_uppercase(b), string.length(acc) {
    True, False, l if l > 0 ->
      insert_underscores_in_keyword(
        string.slice(keyword, 1, length - 1),
        acc <> "_" <> a,
      )

    False, True, _ ->
      insert_underscores_in_keyword(
        string.slice(keyword, 2, length - 2),
        acc <> a <> "_" <> b,
      )

    _, _, _ ->
      insert_underscores_in_keyword(
        string.slice(keyword, 1, length - 1),
        acc <> a,
      )
  }
}

/// Returns whether the passed string only contains uppercase characters.
///
fn is_uppercase(s: String) -> Bool {
  s == string.uppercase(s)
}

type PrivateTags =
  Dict(String, Dict(String, List(String)))

/// Reads the raw private tags file sourced from GDCM.
///
fn read_private_tags_json() -> PrivateTags {
  let assert Ok(json_data) = simplifile.read("data/gdcm_private_tags.json")

  // Decode the JSON content
  let private_tags_decoder =
    dynamic.dict(
      of: dynamic.string,
      to: dynamic.dict(of: string, to: dynamic.list(of: dynamic.string)),
    )

  let assert Ok(private_tags) =
    json.decode(from: json_data, using: private_tags_decoder)

  private_tags
}

/// Prints code that defines constants for each registry entry.
///
fn generate_constants(registry_entries: List(RegistryItem)) -> Nil {
  registry_entries
  |> list.each(fn(registry_entry) {
    let tag = string.replace(registry_entry.tag, "X", "0")

    let group = string.slice(tag, 1, 4)
    let element = string.slice(tag, 6, 4)

    let item =
      item_constructor(
        "DataElementTag(0x" <> group <> ", 0x" <> element <> ")",
        registry_entry,
      )

    io.println("pub const " <> registry_entry.keyword <> " = " <> item)
  })
}

fn item_constructor(tag: String, registry_entry: RegistryItem) -> String {
  let args = [
    tag,
    "\"" <> string.replace(registry_entry.name, "\\", "\\\\") <> "\"",
    convert_value_representation(registry_entry.value_representation),
    convert_value_multiplicity(registry_entry.value_multiplicity),
  ]

  "Item(" <> string.join(args, ", ") <> ")"
}

/// Prints code for the registry.find() function.
///
fn generate_find_function(registry_entries: List(RegistryItem)) -> Nil {
  // Find all groups that don't specify a range
  let simple_groups =
    registry_entries
    |> list.map(fn(registry_entry) { string.slice(registry_entry.tag, 1, 4) })
    |> set.from_list
    |> set.to_list
    |> list.filter(fn(tag) { !string.contains(tag, "X") })
    |> list.sort(string.compare)

  // Generate a find function for each simple group
  simple_groups
  |> list.each(fn(group) {
    { "
/// Returns details for a data element in group 0x" <> group <> ".
///
fn find_element_in_group_" <> string.lowercase(group) <> "(element: Int) -> Result(Item, Nil) {
  case element {" }
    |> io.println

    registry_entries
    |> list.each(fn(registry_entry) {
      use <- bool.guard(string.slice(registry_entry.tag, 1, 4) != group, Nil)
      use <- bool.guard(string.contains(registry_entry.tag, "X"), Nil)

      let element = "0x" <> string.slice(registry_entry.tag, 6, 4)

      { "    " <> element <> " -> Ok(" <> registry_entry.keyword <> ")" }
      |> io.println
    })

    "
    _ -> Error(Nil)
  }
}"
    |> io.println
  })

  "
/// Returns details for a data element based on a tag. The private creator is
/// required in order to look up well-known privately defined data elements.
///
pub fn find(tag: DataElementTag, private_creator: Option(String)) -> Result(Item, Nil) {
  case tag.group {
  "
  |> io.println

  // Handle simple groups with no ranges by passing off to their helper function
  simple_groups
  |> list.each(fn(group) {
    {
      "0x"
      <> group
      <> " -> find_element_in_group_"
      <> string.lowercase(group)
      <> "(tag.element)"
    }
    |> io.println
  })

  // Now handle remaining registry entries that specify a range of some kind
  "
    _ ->  case tag.group, tag.element {"
  |> io.println

  // Print cases for the registry entries with ranges simple enough to be
  // handled as a single case
  registry_entries
  |> list.each(fn(registry_entry) {
    use <- bool.guard(!string.contains(registry_entry.tag, "X"), Nil)

    use <- bool.guard(string.starts_with(registry_entry.tag, "(1000,XXX"), Nil)

    let group = "0x" <> string.slice(registry_entry.tag, 1, 4)
    let element = "0x" <> string.slice(registry_entry.tag, 6, 4)

    {
      "\n    // Handle the '"
      <> registry_entry.tag
      <> " "
      <> registry_entry.name
      <> "' range of data elements"
      <> "\n    "
    }
    |> io.print

    case registry_entry.tag {
      "(0020,31XX)" ->
        "0x0020, element if element >= 0x3100 && element <= 0x31FF"

      "(0028,04X0)"
      | "(0028,04X1)"
      | "(0028,04X2)"
      | "(0028,04X3)"
      | "(0028,08X0)"
      | "(0028,08X2)"
      | "(0028,08X3)"
      | "(0028,08X4)"
      | "(0028,08X8)" -> {
        "0x0028, element if "
        <> {
          list.range(0, 15)
          |> list.map(fn(i) {
            "element == "
            <> string.slice(element, 0, 4)
            <> int.to_base16(i)
            <> string.slice(element, 5, 1)
          })
          |> string.join(" || ")
        }
      }

      "(1010,XXXX)" -> "0x1010, _"

      "(50XX," <> _ | "(60XX," <> _ | "(7FXX," <> _ -> {
        "group, "
        <> element
        <> " if group >= "
        <> string.slice(group, 0, 4)
        <> "00 && group <= "
        <> string.slice(group, 0, 4)
        <> "FF"
      }

      _ -> panic as { "Range not handled: " <> registry_entry.tag }
    }
    |> io.print

    io.println(" -> Ok(Item(.." <> registry_entry.keyword <> ", tag: tag))")
  })

  // Print custom handler for the (1000,XXXY) range
  "
    // Handle the '(1000,XXXY)' range of data elements, where Y is in the range 0-5
    0x1000, element -> case element % 16 {
"
  |> io.print

  list.range(0, 5)
  |> list.map(fn(i) {
    let assert Ok(item) =
      list.find(registry_entries, fn(e) {
        e.tag == "(1000,XXX" <> int.to_string(i) <> ")"
      })

    "      "
    <> int.to_string(i)
    <> " -> Ok(Item(.."
    <> item.keyword
    <> ", tag: tag))"
  })
  |> string.join("\n")
  |> io.print

  "
      _ -> Error(Nil)
    }
      "
  |> io.print

  // Print custom handler for the (gggg,00XX) range
  "
      // Handle private range tags
      _, _ -> {
        // Check this is a private range tag
        use <- bool.guard(!data_element_tag.is_private(tag), Error(Nil))

        // Handle the '(gggg,00XX) Private Creator' data elements.
        // Ref: PS3.5 7.8.1.
        use <- bool.guard(data_element_tag.is_private_creator(tag), Ok(
          "
  |> io.print

  RegistryItem(
    tag: "",
    name: "Private Creator",
    keyword: "",
    value_representation: "LO",
    value_multiplicity: "1",
  )
  |> item_constructor("tag", _)
  |> io.print

  "
        ))

        // Handle other private range tags
        case private_creator {
          Some(private_creator) -> find_private(tag, private_creator)
          None -> Error(Nil)
        }
      }
    }
  }
}"
  |> io.print
}

/// Prints code for the registry.find_private() function.
///
fn generate_find_private_function(private_tags: PrivateTags) -> Nil {
  io.println(
    "
/// Returns details for a well-known privately defined data element.
///
fn find_private(tag: DataElementTag, private_creator: String) -> Result(Item, Nil) {
  // Get the high and low bytes of the group and element to match against
  let g0 = int.bitwise_shift_right(tag.group, 8)
  let g1 = int.bitwise_and(tag.group, 0xFF)
  let e0 = int.bitwise_shift_right(tag.element, 8)
  let e1 = int.bitwise_and(tag.element, 0xFF)

  case private_creator {",
  )

  private_tags
  |> dict.keys
  |> list.each(fn(name) {
    let assert Ok(private_creator_tags) = dict.get(private_tags, name)

    // Ignore tags where the high nibble of the element is zero, as this isn't
    // valid. Ref: PS3.5 7.8.1.
    let private_creator_tags =
      private_creator_tags
      |> dict.filter(fn(tag, _) { string.slice(tag, 4, 1) != "0" })
    use <- bool.guard(dict.is_empty(private_creator_tags), Nil)

    io.println("    \"" <> name <> "\" ->")
    io.println("      case g0, g1, e0, e1 {")

    private_creator_tags
    |> dict.keys()
    |> list.each(fn(tag) {
      // Match on this tag's bytes, where 'xx' is a wildcard
      io.print("        ")
      [0, 2, 4, 6]
      |> list.each(fn(i) {
        case string.slice(tag, i, 2) {
          "xx" -> "_, "
          b -> "0x" <> b <> ", "
        }
        |> io.print
      })

      // Print entry
      let assert Ok([vrs, multiplicity, name, _private]) =
        dict.get(private_creator_tags, tag)
      io.print(" -> Ok(")
      RegistryItem(
        tag: "",
        name: name,
        keyword: "",
        value_representation: vrs,
        value_multiplicity: multiplicity,
      )
      |> item_constructor("tag", _)
      |> io.print

      io.println(")")
    })

    io.println("        _, _, _, _ -> Error(Nil)")
    io.println("      }")
  })

  io.println("    _ -> Error(Nil)")
  io.println("  }")
  io.println("}")
}

/// Returns the code for a value representation.
///
fn convert_value_representation(vr: String) -> String {
  case vr {
    "AE" -> "[ApplicationEntity]"
    "AS" -> "[AgeString]"
    "AT" -> "[AttributeTag]"
    "CS" -> "[CodeString]"
    "DA" -> "[Date]"
    "DS" -> "[DecimalString]"
    "DT" -> "[DateTime]"
    "FD" -> "[FloatingPointDouble]"
    "FL" -> "[FloatingPointSingle]"
    "IS" -> "[IntegerString]"
    "LO" -> "[LongString]"
    "LT" -> "[LongText]"
    "OB" -> "[OtherByteString]"
    "OD" -> "[OtherDoubleString]"
    "OF" -> "[OtherFloatString]"
    "OL" -> "[OtherLongString]"
    "OV" -> "[OtherVeryLongString]"
    "OW" -> "[OtherWordString]"
    "PN" -> "[PersonName]"
    "SH" -> "[ShortString]"
    "SL" -> "[SignedLong]"
    "SQ" -> "[Sequence]"
    "SS" -> "[SignedShort]"
    "ST" -> "[ShortText]"
    "SV" -> "[SignedVeryLong]"
    "TM" -> "[Time]"
    "UC" -> "[UnlimitedCharacters]"
    "UI" -> "[UniqueIdentifier]"
    "UL" -> "[UnsignedLong]"
    "UN" -> "[Unknown]"
    "UR" -> "[UniversalResourceIdentifier]"
    "US" -> "[UnsignedShort]"
    "UT" -> "[UnlimitedText]"
    "UV" -> "[UnsignedVeryLong]"

    "OB_OW" | "OB or OW" -> "[OtherByteString, OtherWordString]"
    "US or SS" -> "[UnsignedShort, SignedShort]"
    "US or OW" ->
      "[value_representation.UnsignedShort, "
      <> "value_representation.OtherWordString]"
    "US or SS or OW" ->
      "[value_representation.UnsignedShort, "
      <> "value_representation.SignedShort, "
      <> "value_representation.OtherWordString]"

    "See Note 2" -> "[]"

    _ -> panic as { "Unknown value representation: " <> vr }
  }
}

/// Returns the code for a value multiplicity.
///
fn convert_value_multiplicity(value_multiplicity: String) -> String {
  case value_multiplicity {
    "1" -> "vm_1"
    "2" -> "vm_2"
    "3" -> "vm_3"
    "4" -> "vm_4"
    "5" -> "vm_5"
    "6" -> "vm_6"
    "8" -> "ValueMultiplicity(8, Some(8))"
    "9" -> "ValueMultiplicity(9, Some(9))"
    "10" -> "ValueMultiplicity(10, Some(10))"
    "12" -> "ValueMultiplicity(12, Some(12))"
    "16" -> "ValueMultiplicity(16, Some(16))"
    "18" -> "ValueMultiplicity(18, Some(18))"
    "24" -> "ValueMultiplicity(24, Some(24))"
    "28" -> "ValueMultiplicity(28, Some(28))"
    "35" -> "ValueMultiplicity(35, Some(35))"
    "256" -> "ValueMultiplicity(256, Some(256))"
    "1-2" -> "vm_1_to_2"
    "1-3" -> "ValueMultiplicity(1, Some(3))"
    "1-4" -> "ValueMultiplicity(1, Some(4))"
    "1-8" -> "ValueMultiplicity(1, Some(8))"
    "1-99" -> "ValueMultiplicity(1, Some(99))"
    "1-32" -> "ValueMultiplicity(1, Some(32))"
    "1-5" -> "ValueMultiplicity(1, Some(5))"
    "2-4" -> "ValueMultiplicity(2, Some(4))"
    "3-4" -> "ValueMultiplicity(3, Some(4))"
    "4-5" -> "ValueMultiplicity(4, Some(5))"
    "1-n" | "1-n or 1" -> "vm_1_to_n"
    "2-n" | "2-2n" -> "vm_2_to_n"
    "3-n" | "3-3n" -> "vm_3_to_n"
    "4-4n" -> "ValueMultiplicity(4, None)"
    "6-n" | "6-6n" -> "ValueMultiplicity(6, None)"
    "7-n" | "7-7n" -> "ValueMultiplicity(7, None)"
    "30-30n" -> "ValueMultiplicity(30, None)"
    "47-47n" -> "ValueMultiplicity(47, None)"

    _ -> panic as { "Unknown value multiplicity: " <> value_multiplicity }
  }
}

type UidDefinition {
  UidDefinition(
    uid: String,
    name: String,
    uid_type: String,
    info: String,
    retired: String,
    keyword: String,
  )
}

/// Reads the raw UIDs file sourced from pydicom.
///
fn read_uid_definitions_json() -> List(UidDefinition) {
  let assert Ok(uids_json) = simplifile.read("data/uids.json")

  // Decode the JSON content
  let decoder =
    dynamic.list(of: dynamic.decode6(
      UidDefinition,
      field("UID", of: string),
      field("Name", of: string),
      field("Type", of: string),
      field("Info", of: string),
      field("Retired", of: string),
      field("Keyword", of: string),
    ))

  let assert Ok(uid_definitions) = json.decode(from: uids_json, using: decoder)

  uid_definitions
  |> list.sort(fn(a, b) { string.compare(a.uid, b.uid) })
}

fn generate_uid_name_function() {
  let uid_definitions = read_uid_definitions_json()

  io.println(
    "
/// Returns the display name for a UID defined in the DICOM standard.
///
pub fn uid_name(uid: String) -> Result(String, Nil) {
  case uid {",
  )

  uid_definitions
  |> list.each(fn(n) {
    io.println("    \"" <> n.uid <> "\" -> Ok(\"" <> n.name <> "\")")
  })

  io.println("    _ -> Error(Nil)")
  io.println("  }")
  io.println("}")
}
