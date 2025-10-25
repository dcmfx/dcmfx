# Roadmap

DCMfx has no formal roadmap, but future work is likely to include the following:

- Pixel data handling

  - Decode High-Throughput JPEG 2000 pixel data with OpenJPH as an alternative
    to the existing decoding with OpenJPEG
  - Change to `libjpeg-turbo` for JPEG Baseline 8-bit, JPEG Extended 12-bit, and
    JPEG Lossless.
  - Transcode multi-frame pixel data into H.264/H.265 transfer syntaxes
  - Resize/rotate/flip pixel data while transcoding
  - Allow fast cropping of JPEG pixel data when the crop is aligned to
    compression blocks
  - Crop pixel data overlays when cropping pixel data
  - Update `SequenceOfUltrasoundRegions` when cropping pixel data.

- CLI:

  - Native use of S3/Azure/GCP/WebDAV URLs via `object_store` crate
  - `get-document` command:
    - New command to get an encapsulated document such as a PDF
  - `list` command:
    - Make all File Meta Information data elements selectable, not just the
      transfer syntax
    - Filter listed DICOMs by data element value
  - `modify` command:
    - `--merge` and `--merge-json` arguments to merge DICOM data sets together
    - `--select-frames` argument to transcode only specific frames
    - `--replace-pixel-data` to replace a DICOM's pixel data
    - `--create-basic-offset-table` to add a basic offset table and fragments to
      the pixel data if absent

- Decode JPEG-LS pixel data on WASM

- DIMSE networking

- Decoding and encoding of waveform data

- Extraction of DICOM structured report data

- Creation of DICOMDIR indexes
