# Libraries

::: tip UNRELEASED
DCMfx's libraries are not yet published as packages or crates.
:::

## `dcmfx_core`

Provides core DICOM concepts such as data sets, data elements, value
representations, value multiplicity, and transfer syntaxes. Defines a registry
of the data elements defined in Part 6 of the DICOM specification. as well as
well-known private data elements.

More details [here](./libraries/dcmfx-core).

## `dcmfx_p10`

Reads, writes, and modifies the DICOM Part 10 (P10) file format. Uses a
streaming design suited for highly concurrent and memory-constrained
environments.

Provides transforms for reading and altering the data in a stream of DICOM P10
data in a variety of ways.

More details [here](./libraries/dcmfx-p10).

## `dcmfx_json`

Converts between DICOM data and the DICOM JSON Model. Supports stream conversion
of DICOM Part 10 data to DICOM JSON with very low memory usage.

Features an optional extension to the DICOM JSON Model that allows for the
storage of encapsulated pixel data.

More details [here](./libraries/dcmfx-json).

## `dcmfx_pixel_data`

Extracts frames of pixel data from a DICOM data set.

Note that decoding or decompression of the pixel data is not yet supported.

More details [here](./libraries/dcmfx-pixel-data).

## `dcmfx_anonymize`

Anonymizes the data elements in a DICOM data set or stream of DICOM Part 10
data.

More details [here](./libraries/dcmfx-anonymize).

## `dcmfx_character_set`

Internal library that decodes DICOM string data. This library does not need to
be used directly when using DCMfx, it is used by `dcmfx_p10` when needed in
order to read incoming DICOM P10 data.

More details [here](./libraries/dcmfx-character-set).
