#!/usr/bin/env python3
#
# This script uses pydicom to write a JSON file for the DICOM file specified as
# an argument.
#
# To install required Python packages: `pip3 install -r requirements.txt`.
#
# To regenerate all DICOM JSON files:
#
#   find . -type f -name "*.dcm" -exec ./dicom_to_json.py {} \;

import base64
import pathlib
import pydicom
import simplejson
import sys


def dicom_to_json(file):
    # Read file with pydicom
    data_set = pydicom.dcmread(file, force=True)
    dicom_json_dict = data_set.to_json_dict()

    # Add the transfer syntax UID to the output JSON, pydicom doesn't include it
    # by default
    is_big_endian = False
    if "TransferSyntaxUID" in data_set.file_meta:
        transfer_syntax_uid = data_set.file_meta.TransferSyntaxUID
        dicom_json_dict["00020010"] = {
            "vr": "UI",
            "Value": [transfer_syntax_uid]
        }

        is_big_endian = transfer_syntax_uid == "1.2.840.10008.1.2.2"

    standardize_json_dict(dicom_json_dict, is_big_endian)

    # Write the result to a JSON file
    pydicom_result = simplejson.dumps(dicom_json_dict, indent=2, sort_keys=True)
    pathlib.Path(file + ".json").write_text(pydicom_result)


# Takes a DICOM JSON conversion generated by pydicom and makes some updates to
# it to exactly match DCMfx's output.
def standardize_json_dict(dicom_json_dict, is_big_endian):
    for tag, value in dicom_json_dict.copy().items():
        vr = value["vr"]

        # Remove retired group length tags, the SpecificCharacterSet tag, and
        # the data set trailing padding tag
        if tag.endswith("0000") or tag == "00080005" or tag == "FFFCFFFC":
            del dicom_json_dict[tag]

        # Replace invalid "OB or OW" VRs with "UN" to conform to the DICOM JSON
        # spec, the former is not permitted
        elif vr == "OB or OW":
            value["vr"] = "UN"

        # Convert simple values
        elif vr != "SQ" and "Value" in value:
            # Strip strings
            value["Value"] = [
                (v.rstrip() if isinstance(v, str) else v)
                for v in value["Value"]
            ]

            # Turn empty strings into 'None'
            value["Value"] = [(None if v == "" else v) for v in value["Value"]]

            # Remove empty Value arrays
            if value["Value"] == []:
                del value["Value"]

        # Byte swap big endian to little endian in inline binaries. This is what
        # dcm2json outputs, and DCMfx does the same thing, i.e. InlineBinary in
        # DICOM JSON data is always little endian.
        elif (
            is_big_endian
            and "InlineBinary" in value
            and vr in ["OW", "OD", "OF", "OL", "OV"]
        ):
            item_size = {"OW": 2, "OD": 8, "OF": 4, "OL": 4, "OV": 8}[vr]

            bytes = base64.b64decode(value["InlineBinary"])

            # Iterate over the bytearray in chunks of `item_size`
            swapped = bytearray()
            for i in range(0, len(bytes), 2):
                item = bytes[i : i + item_size]
                swapped.extend(item[::-1])

            value["InlineBinary"] = base64.b64encode(swapped)

        # Recursively sanitize sequences
        elif vr == "SQ" and "Value" in value:
            for item in value["Value"]:
                standardize_json_dict(item, is_big_endian)


if __name__ == "__main__":
    dicom_to_json(sys.argv[1])
