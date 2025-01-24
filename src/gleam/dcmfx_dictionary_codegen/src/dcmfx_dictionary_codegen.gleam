import gleam/bool
import gleam/dict.{type Dict}
import gleam/dynamic/decode
import gleam/int
import gleam/io
import gleam/json
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/set
import gleam/string
import simplifile

type TargetLanguage {
  Gleam
  Rust
}

const target_language = Gleam

pub fn main() {
  let dictionary_items = read_attributes_json()
  let private_tags = read_private_tags_json()

  // Generate code and print it to stdout
  generate_constants(dictionary_items)
  generate_find_function(dictionary_items)
  generate_find_private_function(private_tags)
  generate_uid_name_function()
}

type DictionaryItem {
  DictionaryItem(
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
fn read_attributes_json() -> List(DictionaryItem) {
  let assert Ok(attributes_json) =
    simplifile.read("data/innolitics_attributes.json")

  // Decode the JSON content
  let items_decoder = {
    use tag <- decode.field("tag", decode.string)
    use name <- decode.field("name", decode.string)
    use keyword <- decode.field("keyword", decode.string)
    use value_representation <- decode.field(
      "valueRepresentation",
      decode.string,
    )
    use value_multiplicity <- decode.field("valueMultiplicity", decode.string)

    decode.success(DictionaryItem(
      tag:,
      name:,
      keyword:,
      value_representation:,
      value_multiplicity:,
    ))
  }

  let assert Ok(dictionary_items) =
    json.parse(from: attributes_json, using: decode.list(items_decoder))

  // Filter out items that have no name, keyword, or VR
  let dictionary_items =
    dictionary_items
    |> list.filter(fn(attribute) {
      attribute.name != ""
      && attribute.keyword != ""
      && attribute.value_representation != ""
    })

  // Change keyword to use snake case. Some keywords require manual adjustment.
  let dictionary_items =
    dictionary_items
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
        |> case target_language {
          Gleam -> string.lowercase
          Rust -> string.uppercase
        }

      DictionaryItem(..attribute, keyword:)
    })

  // Sort by tag
  dictionary_items
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
    decode.dict(
      decode.string,
      decode.dict(decode.string, decode.list(decode.string)),
    )

  let assert Ok(private_tags) =
    json.parse(from: json_data, using: private_tags_decoder)

  private_tags
}

/// Prints code that defines constants for each dictionary item.
///
fn generate_constants(dictionary_items: List(DictionaryItem)) -> Nil {
  dictionary_items
  |> list.each(fn(item) {
    let tag = string.replace(item.tag, "X", "0")

    let group = string.slice(tag, 1, 4)
    let element = string.slice(tag, 6, 4)

    case target_language {
      Gleam -> {
        let tag = "DataElementTag(0x" <> group <> ", 0x" <> element <> ")"
        let item_code = item_constructor(tag, item)

        io.println("pub const " <> item.keyword <> " = " <> item_code)
      }
      Rust -> {
        let tag =
          "DataElementTag { group: 0x"
          <> group
          <> ", element: 0x"
          <> element
          <> " }"
        let item_code = item_constructor(tag, item)

        io.println(
          "\npub const " <> item.keyword <> ": Item = " <> item_code <> ";",
        )
      }
    }
  })
}

fn item_constructor(tag: String, item: DictionaryItem) -> String {
  case target_language {
    Gleam -> {
      let args = [
        tag,
        "\"" <> string.replace(item.name, "\\", "\\\\") <> "\"",
        convert_value_representation(item.value_representation),
        convert_value_multiplicity(item.value_multiplicity),
      ]

      "Item(" <> string.join(args, ", ") <> ")"
    }

    Rust -> {
      let args = [
        case tag {
          "tag" -> "tag"
          _ -> "tag: " <> tag
        },
        "name: \"" <> string.replace(item.name, "\\", "\\\\") <> "\"",
        "vrs: &" <> convert_value_representation(item.value_representation),
        "multiplicity: " <> convert_value_multiplicity(item.value_multiplicity),
      ]

      "Item {\n" <> string.join(args, ",\n") <> "\n}"
    }
  }
}

/// Prints code for the dictionary.find() function.
///
fn generate_find_function(dictionary_items: List(DictionaryItem)) -> Nil {
  // Find all groups that don't specify a range
  let simple_groups =
    dictionary_items
    |> list.map(fn(item) { string.slice(item.tag, 1, 4) })
    |> set.from_list
    |> set.to_list
    |> list.filter(fn(group) {
      use <- bool.guard(string.contains(group, "X"), False)

      dictionary_items
      |> list.any(fn(item) {
        string.slice(item.tag, 1, 4) == group && !string.contains(item.tag, "X")
      })
    })
    |> list.sort(string.compare)

  // Generate a find function for each simple group
  simple_groups
  |> list.each(fn(group) {
    io.println("
/// Returns details for a data element in group 0x" <> group <> ".
///
" <> case target_language {
      Gleam ->
        "fn find_element_in_group_"
        <> string.lowercase(group)
        <> "(element: Int) -> Result(Item, Nil) {
  case element {"
      Rust ->
        "fn find_element_in_group_"
        <> string.lowercase(group)
        <> "(element: u16) -> Result<Item, ()> {
  match element {"
    })

    let arrow = case target_language {
      Gleam -> "->"
      Rust -> "=>"
    }

    dictionary_items
    |> list.each(fn(item) {
      use <- bool.guard(string.slice(item.tag, 1, 4) != group, Nil)
      use <- bool.guard(string.contains(item.tag, "X"), Nil)

      let element = "0x" <> string.slice(item.tag, 6, 4)

      io.println(
        "    "
        <> element
        <> " "
        <> arrow
        <> " Ok("
        <> item.keyword
        <> ")"
        <> case target_language {
          Gleam -> ""
          Rust -> ","
        },
      )
    })

    let e = case target_language {
      Gleam -> "Error(Nil)"
      Rust -> "Err(())"
    }

    io.println("    _ " <> arrow <> " " <> e <> "\n  }\n}")
  })

  io.println("
/// Returns details for a data element based on a tag. The private creator is
/// required in order to look up well-known privately defined data elements.
///
" <> case target_language {
    Gleam ->
      "pub fn find(tag: DataElementTag, private_creator: Option(String)) -> Result(Item, Nil) {
  case tag.group {"
    Rust ->
      "pub fn find(tag: DataElementTag, private_creator: Option<&str>) -> Result<Item, ()> {
  match tag.group {
  "
  })

  let arrow = case target_language {
    Gleam -> "->"
    Rust -> "=>"
  }

  let match_arm_separator = case target_language {
    Gleam -> ""
    Rust -> ","
  }

  // Handle simple groups with no ranges by passing off to their helper function
  simple_groups
  |> list.each(fn(group) {
    io.println(
      "0x"
      <> group
      <> " "
      <> arrow
      <> " find_element_in_group_"
      <> string.lowercase(group)
      <> "(tag.element)"
      <> match_arm_separator,
    )
  })

  // Now handle remaining dictionary items that specify a range of some kind
  io.println(case target_language {
    Gleam -> "\n    _ -> case tag.group, tag.element {"
    Rust -> "\n    _ => match (tag.group, tag.element) {"
  })

  // Print cases for the dictionary items with ranges simple enough to be
  // handled as a single case
  dictionary_items
  |> list.each(fn(dictionary_item) {
    use <- bool.guard(!string.contains(dictionary_item.tag, "X"), Nil)

    use <- bool.guard(string.starts_with(dictionary_item.tag, "(1000,XXX"), Nil)

    let group = "0x" <> string.slice(dictionary_item.tag, 1, 4)
    let element = "0x" <> string.slice(dictionary_item.tag, 6, 4)

    io.print(
      "\n    // Handle the '"
      <> dictionary_item.tag
      <> " "
      <> dictionary_item.name
      <> "' range of data elements"
      <> "\n    ",
    )

    case dictionary_item.tag {
      "(0020,31XX)" ->
        case target_language {
          Gleam -> "0x0020, element"
          Rust -> "(0x0020, element)"
        }
        <> " if element >= 0x3100 && element <= 0x31FF"

      "(0028,04X0)"
      | "(0028,04X1)"
      | "(0028,04X2)"
      | "(0028,04X3)"
      | "(0028,08X0)"
      | "(0028,08X2)"
      | "(0028,08X3)"
      | "(0028,08X4)"
      | "(0028,08X8)" ->
        case target_language {
          Gleam -> "0x0028, element if "
          Rust -> "(0x0028, element) if "
        }
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

      "(1010,XXXX)" ->
        case target_language {
          Gleam -> "0x1010, _"
          Rust -> "(0x1010, _)"
        }

      "(50XX," <> _ | "(60XX," <> _ | "(7FXX," <> _ ->
        case target_language {
          Gleam -> "group, " <> element
          Rust -> "(group, " <> element <> ")"
        }
        <> " if group >= "
        <> string.slice(group, 0, 4)
        <> "00 && group <= "
        <> string.slice(group, 0, 4)
        <> "FF"

      _ -> panic as { "Range not handled: " <> dictionary_item.tag }
    }
    |> io.print

    case target_language {
      Gleam -> " -> Ok(Item(.." <> dictionary_item.keyword <> ", tag: tag))"
      Rust -> " => Ok(Item{tag, .." <> dictionary_item.keyword <> "}),"
    }
    |> io.println
  })

  // Print custom handler for the (1000,XXXY) range
  io.println("
      // Handle the '(1000,XXXY)' range of data elements, where Y is in the range 0-5
      " <> case target_language {
    Gleam -> "0x1000, element -> case element % 16 {"
    Rust -> "(0x1000, element) => match element % 16 {"
  })

  list.range(0, 5)
  |> list.map(fn(i) {
    let assert Ok(item) =
      list.find(dictionary_items, fn(e) {
        e.tag == "(1000,XXX" <> int.to_string(i) <> ")"
      })

    "      "
    <> int.to_string(i)
    <> case target_language {
      Gleam -> " -> Ok(Item(.."
      Rust -> " => Ok(Item{tag, .."
    }
    <> item.keyword
    <> case target_language {
      Gleam -> ", tag: tag))"
      Rust -> "}),"
    }
  })
  |> string.join("\n")
  |> io.print

  { "
      _ " <> case target_language {
      Gleam -> "-> Error(Nil)"
      Rust -> "=> Err(())"
    } <> "
    }
      " }
  |> io.print

  // Print custom handler for the (gggg,00XX) range
  case target_language {
    Gleam -> {
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

      DictionaryItem(
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
      |> io.println
    }
    Rust ->
      "
      // Handle private range tags
      _ => {
        // Check this is a private range tag
        if !tag.is_private() {
          return Err(());
        }

        // Handle the '(gggg,00XX) Private Creator' data elements.
        // Ref: PS3.5 7.8.1.
        if tag.is_private_creator() {
          return Ok(Item {
            tag,
            name: \"Private Creator\",
            vrs: &[ValueRepresentation::LongString],
            multiplicity: VM_1,
          });
        }

        // Handle other private range tags
        match private_creator {
          Some(private_creator) => find_private(tag, private_creator),
          None => Err(())
        }
      }
    }
  }
}"
      |> io.println
  }
}

/// Prints code for the dictionary.find_private() function.
///
fn generate_find_private_function(private_tags: PrivateTags) -> Nil {
  io.println("
/// Returns details for a well-known privately defined data element.
///\n" <> case target_language {
    Gleam ->
      "fn find_private(tag: DataElementTag, private_creator: String) -> Result(Item, Nil) {
  // Get the high and low bytes of the group and element to match against
  let g0 = int.bitwise_shift_right(tag.group, 8)
  let g1 = int.bitwise_and(tag.group, 0xFF)
  let e0 = int.bitwise_shift_right(tag.element, 8)
  let e1 = int.bitwise_and(tag.element, 0xFF)

  case private_creator {"
    Rust ->
      "fn find_private(tag: DataElementTag, private_creator: &str) -> Result<Item, ()> {
  // Get the high and low bytes of the group and element to match against
  let g0 = tag.group >> 8;
  let g1 = tag.group & 0xFF;
  let e0 = tag.element >> 8;
  let e1 = tag.element & 0xFF;

  match private_creator {"
  })

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

    case target_language {
      Gleam -> {
        io.println("    \"" <> name <> "\" ->")
        io.println("      case g0, g1, e0, e1 {")
      }
      Rust -> {
        io.println("    \"" <> name <> "\" =>")
        io.println("      match (g0, g1, e0, e1) {")
      }
    }

    private_creator_tags
    |> dict.keys()
    |> list.each(fn(tag) {
      // Match on this tag's bytes, where 'xx' is a wildcard
      io.print("        ")
      case target_language {
        Gleam -> io.print("")
        Rust -> io.print("(")
      }

      let separator = case target_language {
        Gleam -> ", "
        Rust -> ""
      }

      [0, 2, 4, 6]
      |> list.each(fn(i) {
        case string.slice(tag, i, 2) {
          "xx" -> "_" <> separator
          b -> "0x" <> b <> "" <> separator
        }
        |> io.print

        case target_language {
          Gleam -> Nil
          Rust ->
            case i {
              6 -> ""
              _ -> ", "
            }
            |> io.print
        }
      })

      // Print entry
      let assert Ok([vrs, multiplicity, name, _private]) =
        dict.get(private_creator_tags, tag)
      case target_language {
        Gleam -> " -> Ok("
        Rust -> ") => Ok("
      }
      |> io.print()

      let item =
        DictionaryItem(
          tag: "",
          name: name,
          keyword: "",
          value_representation: vrs,
          value_multiplicity: multiplicity,
        )

      item_constructor("tag", item)
      |> io.print

      case target_language {
        Gleam -> io.println(")")
        Rust -> io.println("),")
      }
    })

    case target_language {
      Gleam -> {
        io.println("        _, _, _, _ -> Error(Nil)")
        io.println("      }")
      }
      Rust -> {
        io.println("        _ => Err(()),")
        io.println("      },")
      }
    }
  })

  case target_language {
    Gleam -> io.println("    _ -> Error(Nil)")
    Rust -> io.println("    _ => Err(())")
  }
  io.println("  }")
  io.println("}")
}

/// Returns the code for a value representation.
///
fn convert_value_representation(vr: String) -> String {
  let prefix = case target_language {
    Gleam -> ""
    Rust -> "ValueRepresentation::"
  }

  case vr {
    "AE" -> "[" <> prefix <> "ApplicationEntity]"
    "AS" -> "[" <> prefix <> "AgeString]"
    "AT" -> "[" <> prefix <> "AttributeTag]"
    "CS" -> "[" <> prefix <> "CodeString]"
    "DA" -> "[" <> prefix <> "Date]"
    "DS" -> "[" <> prefix <> "DecimalString]"
    "DT" -> "[" <> prefix <> "DateTime]"
    "FD" -> "[" <> prefix <> "FloatingPointDouble]"
    "FL" -> "[" <> prefix <> "FloatingPointSingle]"
    "IS" -> "[" <> prefix <> "IntegerString]"
    "LO" -> "[" <> prefix <> "LongString]"
    "LT" -> "[" <> prefix <> "LongText]"
    "OB" -> "[" <> prefix <> "OtherByteString]"
    "OD" -> "[" <> prefix <> "OtherDoubleString]"
    "OF" -> "[" <> prefix <> "OtherFloatString]"
    "OL" -> "[" <> prefix <> "OtherLongString]"
    "OV" -> "[" <> prefix <> "OtherVeryLongString]"
    "OW" -> "[" <> prefix <> "OtherWordString]"
    "PN" -> "[" <> prefix <> "PersonName]"
    "SH" -> "[" <> prefix <> "ShortString]"
    "SL" -> "[" <> prefix <> "SignedLong]"
    "SQ" -> "[" <> prefix <> "Sequence]"
    "SS" -> "[" <> prefix <> "SignedShort]"
    "ST" -> "[" <> prefix <> "ShortText]"
    "SV" -> "[" <> prefix <> "SignedVeryLong]"
    "TM" -> "[" <> prefix <> "Time]"
    "UC" -> "[" <> prefix <> "UnlimitedCharacters]"
    "UI" -> "[" <> prefix <> "UniqueIdentifier]"
    "UL" -> "[" <> prefix <> "UnsignedLong]"
    "UN" -> "[" <> prefix <> "Unknown]"
    "UR" -> "[" <> prefix <> "UniversalResourceIdentifier]"
    "US" -> "[" <> prefix <> "UnsignedShort]"
    "UT" -> "[" <> prefix <> "UnlimitedText]"
    "UV" -> "[" <> prefix <> "UnsignedVeryLong]"

    "OB_OW" | "OB or OW" ->
      "[" <> prefix <> "OtherByteString, " <> prefix <> "OtherWordString]"
    "US or SS" -> "[" <> prefix <> "UnsignedShort, " <> prefix <> "SignedShort]"
    "US or OW" ->
      "[" <> prefix <> "UnsignedShort, " <> prefix <> "OtherWordString]"
    "US or SS or OW" ->
      "["
      <> prefix
      <> "UnsignedShort, "
      <> prefix
      <> "SignedShort, "
      <> prefix
      <> "OtherWordString]"

    "See Note 2" -> "[]"

    _ -> panic as { "Unknown value representation: " <> vr }
  }
}

/// Returns the code for a value multiplicity.
///
fn convert_value_multiplicity(value_multiplicity: String) -> String {
  let multiplicity_constant = fn(constant: String) {
    case target_language {
      Gleam -> constant
      Rust -> constant |> string.uppercase
    }
  }

  let multiplicity = fn(min: Int, max: Option(Int)) {
    let min = int.to_string(min)

    let max = case max {
      None -> "None"
      Some(i) -> "Some(" <> int.to_string(i) <> ")"
    }

    case target_language {
      Gleam -> "ValueMultiplicity(" <> min <> ", " <> max <> ")"
      Rust -> "ValueMultiplicity{min:" <> min <> ", max:" <> max <> "}"
    }
  }

  case value_multiplicity {
    "1" -> multiplicity_constant("vm_1")
    "2" -> multiplicity_constant("vm_2")
    "3" -> multiplicity_constant("vm_3")
    "4" -> multiplicity_constant("vm_4")
    "5" -> multiplicity_constant("vm_5")
    "6" -> multiplicity_constant("vm_6")
    "8" -> multiplicity(8, Some(8))
    "9" -> multiplicity(9, Some(9))
    "10" -> multiplicity(10, Some(10))
    "12" -> multiplicity(12, Some(12))
    "16" -> multiplicity(16, Some(16))
    "18" -> multiplicity(18, Some(18))
    "24" -> multiplicity(24, Some(24))
    "28" -> multiplicity(28, Some(28))
    "35" -> multiplicity(35, Some(35))
    "256" -> multiplicity(256, Some(256))
    "1-2" -> multiplicity_constant("vm_1_to_2")
    "1-3" -> multiplicity(1, Some(3))
    "1-4" -> multiplicity(1, Some(4))
    "1-8" -> multiplicity(1, Some(8))
    "1-99" -> multiplicity(1, Some(99))
    "1-32" -> multiplicity(1, Some(32))
    "1-5" -> multiplicity(1, Some(5))
    "2-4" -> multiplicity(2, Some(4))
    "3-4" -> multiplicity(3, Some(4))
    "4-5" -> multiplicity(4, Some(5))
    "1-n" | "1-n or 1" -> multiplicity_constant("vm_1_to_n")
    "2-n" | "2-2n" -> multiplicity_constant("vm_2_to_n")
    "3-n" | "3-3n" -> multiplicity_constant("vm_3_to_n")
    "4-4n" -> multiplicity(4, None)
    "6-n" | "6-6n" -> multiplicity(6, None)
    "7-n" | "7-7n" -> multiplicity(7, None)
    "30-30n" -> multiplicity(30, None)
    "47-47n" -> multiplicity(47, None)

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
  let uid_definition_decoder = {
    use uid <- decode.field("UID", decode.string)
    use name <- decode.field("Name", decode.string)
    use uid_type <- decode.field("Type", decode.string)
    use info <- decode.field("Info", decode.string)
    use retired <- decode.field("Retired", decode.string)
    use keyword <- decode.field("Keyword", decode.string)

    decode.success(UidDefinition(
      uid:,
      name:,
      uid_type:,
      info:,
      retired:,
      keyword:,
    ))
  }

  let assert Ok(uid_definitions) =
    json.parse(from: uids_json, using: decode.list(uid_definition_decoder))

  uid_definitions
  |> list.sort(fn(a, b) { string.compare(a.uid, b.uid) })
}

fn generate_uid_name_function() {
  let uid_definitions = read_uid_definitions_json()

  io.println("
/// Returns the display name for a UID defined in the DICOM standard.
///" <> case target_language {
    Gleam ->
      "
pub fn uid_name(uid: String) -> Result(String, Nil) {
  case uid {"
    Rust ->
      "
#[allow(clippy::result_unit_err)]
pub fn uid_name(uid: &str) -> Result<&'static str, ()> {
  match uid {"
  })

  uid_definitions
  |> list.each(fn(n) {
    case target_language {
      Gleam -> "    \"" <> n.uid <> "\" -> Ok(\"" <> n.name <> "\")"
      Rust -> "    \"" <> n.uid <> "\" => Ok(\"" <> n.name <> "\"),"
    }
    |> io.println
  })

  case target_language {
    Gleam -> "    _ -> Error(Nil)"
    Rust -> "    _ => Err(())"
  }
  |> io.println

  io.println("  }")
  io.println("}")
}
