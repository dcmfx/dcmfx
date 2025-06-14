// This file contains the C entry points called from Rust to perform JPEG-LS
// decoding and encoding with CharLS.

#ifndef __wasm__

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <charls/charls_jpegls_decoder.h>
#include <charls/charls_jpegls_encoder.h>

size_t charls_decode(const void *input_data, size_t input_data_size,
                     size_t width, size_t height, size_t samples_per_pixel,
                     size_t bits_allocated, void *output_buffer,
                     size_t output_buffer_size, char *error_buffer,
                     size_t error_buffer_size) {
  // Create decoder
  struct charls_jpegls_decoder *decoder = charls_jpegls_decoder_create();
  if (!decoder) {
    strncpy(error_buffer, "charls_jpegls_decoder_create() failed",
            error_buffer_size - 1);
    return 1;
  }

  // Set decoder source
  if (charls_jpegls_decoder_set_source_buffer(decoder, input_data,
                                              input_data_size) != 0) {
    strncpy(error_buffer, "charls_jpegls_decoder_set_source_buffer() failed",
            error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return 1;
  }

  // Read header
  if (charls_jpegls_decoder_read_header(decoder) != 0) {
    strncpy(error_buffer, "charls_jpegls_decoder_read_header() failed",
            error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return 1;
  }

  // Get frame info
  charls_frame_info frame_info = {};
  if (charls_jpegls_decoder_get_frame_info(decoder, &frame_info) != 0) {
    strncpy(error_buffer, "charls_jpegls_decoder_read_spiff_header() failed",
            error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return 1;
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
    return 1;
  }

  // Get required destination size
  size_t destination_size_bytes = 0;
  if (charls_jpegls_decoder_get_destination_size(
          decoder, 0, &destination_size_bytes) != 0) {
    strncpy(error_buffer, "charls_jpegls_decoder_get_destination_size() failed",
            error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return 1;
  }

  // Check the required destination size matches the output buffer's size
  if (destination_size_bytes != output_buffer_size) {
    strncpy(error_buffer, "Output buffer has incorrect size",
            error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return 1;
  }

  // Perform decode
  if (charls_jpegls_decoder_decode_to_buffer(decoder, output_buffer,
                                             destination_size_bytes, 0) != 0) {
    strncpy(error_buffer, "charls_jpegls_decoder_decode_to_buffer() failed",
            error_buffer_size - 1);
    charls_jpegls_decoder_destroy(decoder);
    return 1;
  }

  charls_jpegls_decoder_destroy(decoder);

  return 0;
}

size_t charls_encode(const void *input_data, size_t width, size_t height,
                     size_t samples_per_pixel, size_t bits_allocated,
                     size_t is_near_lossless,
                     void *(*output_buffer_allocate)(size_t len, void *ctx),
                     void *output_buffer_context, char *error_buffer,
                     size_t error_buffer_size) {
  // Create encoder
  struct charls_jpegls_encoder *encoder = charls_jpegls_encoder_create();
  if (!encoder) {
    strncpy(error_buffer, "charls_jpegls_encoder_create() failed",
            error_buffer_size - 1);
    return 0;
  }

  // Enable near-lossless encoding if requested
  if (charls_jpegls_encoder_set_near_lossless(encoder, is_near_lossless)) {
    strncpy(error_buffer, "charls_jpegls_encoder_set_near_lossless() failed",
            error_buffer_size - 1);
    charls_jpegls_encoder_destroy(encoder);
    return 0;
  }

  struct charls_frame_info frame_info = {.width = width,
                                         .height = height,
                                         .bits_per_sample = bits_allocated,
                                         .component_count = samples_per_pixel};

  // Set frame into
  if (charls_jpegls_encoder_set_frame_info(encoder, &frame_info)) {
    strncpy(error_buffer, "charls_jpegls_encoder_set_frame_info() failed",
            error_buffer_size - 1);
    charls_jpegls_encoder_destroy(encoder);
    return 0;
  }

  // Estimate output size
  size_t encoded_length = 0;
  if (charls_jpegls_encoder_get_estimated_destination_size(encoder,
                                                           &encoded_length)) {
    strncpy(error_buffer,
            "charls_jpegls_encoder_get_estimated_destination_size() failed",
            error_buffer_size - 1);
    charls_jpegls_encoder_destroy(encoder);
    return 0;
  }

  // The above size is meant to be the worst case size, however for purely
  // random input data it isn't actually large enough, so add 10% extra.
  encoded_length += encoded_length / 10;

  // Allocate destination buffer
  void *encoded_buffer =
      output_buffer_allocate(encoded_length, output_buffer_context);
  if (encoded_buffer == NULL) {
    return 0;
  }

  if (charls_jpegls_encoder_set_destination_buffer(encoder, encoded_buffer,
                                                   encoded_length)) {
    strncpy(error_buffer,
            "charls_jpegls_encoder_set_destination_buffer() failed",
            error_buffer_size - 1);
    charls_jpegls_encoder_destroy(encoder);
    return 0;
  }

  // Encode the image
  if (charls_jpegls_encoder_encode_from_buffer(
          encoder, input_data,
          width * height * samples_per_pixel * (bits_allocated / 8), 0)) {
    strncpy(error_buffer, "charls_jpegls_encoder_encode_from_buffer() failed",
            error_buffer_size - 1);
    charls_jpegls_encoder_destroy(encoder);
    return 0;
  }

  // Get the actual size of the encoded data
  size_t bytes_written = 0;
  if (charls_jpegls_encoder_get_bytes_written(encoder, &bytes_written)) {
    strncpy(error_buffer, "charls_jpegls_encoder_get_bytes_written() failed",
            error_buffer_size - 1);
    charls_jpegls_encoder_destroy(encoder);
    return 0;
  }

  charls_jpegls_encoder_destroy(encoder);

  return bytes_written;
}

#endif
