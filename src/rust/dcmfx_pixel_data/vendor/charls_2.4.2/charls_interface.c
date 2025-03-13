// This file contains the C entry point called from Rust to perform decoding of
// JPEG-LS data.

#ifndef __wasm__

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <charls/charls_jpegls_decoder.h>

int charls_decode(uint8_t *input_data, uint64_t input_data_size, uint32_t width,
                  uint32_t height, uint32_t samples_per_pixel,
                  uint32_t bits_allocated, uint8_t *output_buffer,
                  uint64_t output_buffer_size, char *error_buffer,
                  uint32_t error_buffer_size) {
  // Create decoder
  struct charls_jpegls_decoder *decoder = charls_jpegls_decoder_create();
  if (!decoder) {
    strncpy(error_buffer, "charls_jpegls_decoder_create() failed",
            error_buffer_size - 1);
    return -1;
  }

  // Set decoder source
  if (charls_jpegls_decoder_set_source_buffer(decoder, input_data,
                                              input_data_size) != 0) {
    strncpy(error_buffer, "charls_jpegls_decoder_set_source_buffer() failed",
            error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return -1;
  }

  // Read header
  if (charls_jpegls_decoder_read_header(decoder) != 0) {
    strncpy(error_buffer, "charls_jpegls_decoder_read_header() failed",
            error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return -1;
  }

  // Get frame info
  charls_frame_info frame_info = {};
  if (charls_jpegls_decoder_get_frame_info(decoder, &frame_info) != 0) {
    strncpy(error_buffer, "charls_jpegls_decoder_read_spiff_header() failed",
            error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return -1;
  }

  // Check frame into matches the expected format
  if (frame_info.width != width || frame_info.height != height ||
      (uint32_t)frame_info.component_count != samples_per_pixel ||
      (((uint32_t)frame_info.bits_per_sample + 7) / 8) * 8 != bits_allocated) {
    strncpy(
        error_buffer,
        "Image does not have the expected width, height, samples per pixel, "
        "or bits allocated",
        error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return -1;
  }

  // Get required destination size
  size_t stride = 0;
  size_t destination_size_bytes = 0;
  if (charls_jpegls_decoder_get_destination_size(
          decoder, stride, &destination_size_bytes) != 0) {
    strncpy(error_buffer, "charls_jpegls_decoder_get_destination_size() failed",
            error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return -1;
  }

  // Check the required destination size matches the output buffer's size
  if (destination_size_bytes != output_buffer_size) {
    strncpy(error_buffer, "Output buffer has incorrect size",
            error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return -1;
  }

  // Perform decode
  if (charls_jpegls_decoder_decode_to_buffer(
          decoder, output_buffer, destination_size_bytes, stride) != 0) {
    strncpy(error_buffer, "charls_jpegls_decoder_decode_to_buffer() failed",
            error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return -1;
  }

  charls_jpegls_decoder_destroy(decoder);

  return 0;
}

#endif
