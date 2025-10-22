// This file contains the C entry points called from Rust to perform JPEG-LS
// decoding and encoding with CharLS.

#include <cstdint>
#include <stdexcept>

#include <charls/charls_jpegls_decoder.h>
#include <charls/charls_jpegls_encoder.h>

using namespace charls;

extern "C" size_t charls_decode(const void *input_data, size_t input_data_size,
                                size_t width, size_t height,
                                size_t samples_per_pixel, size_t bits_allocated,
                                void *output_buffer, size_t output_buffer_size,
                                char *error_buffer, size_t error_buffer_size) {
  charls_jpegls_decoder *decoder = nullptr;

  try {
    // Create decoder
    decoder = charls_jpegls_decoder_create();
    if (!decoder) {
      throw std::runtime_error("charls_jpegls_decoder_create() failed");
    }

    // Set decoder source
    if (charls_jpegls_decoder_set_source_buffer(
            decoder, input_data, input_data_size) != jpegls_errc::success) {
      throw std::runtime_error(
          "charls_jpegls_decoder_set_source_buffer() failed");
    }

    // Read header
    if (charls_jpegls_decoder_read_header(decoder) != jpegls_errc::success) {
      throw std::runtime_error("charls_jpegls_decoder_read_header() failed");
    }

    // Get frame info
    charls_frame_info frame_info = {};
    if (charls_jpegls_decoder_get_frame_info(decoder, &frame_info) !=
        jpegls_errc::success) {
      throw std::runtime_error("charls_jpegls_decoder_get_frame_info() failed");
    }

    // Check frame into matches the expected format
    if (frame_info.width != width || frame_info.height != height ||
        (uint32_t)frame_info.component_count != samples_per_pixel ||
        (((uint32_t)frame_info.bits_per_sample + 7) / 8) * 8 !=
            bits_allocated) {
      throw std::runtime_error(
          "Image does not have the expected width, height, samples per pixel, "
          "or bits allocated");
    }

    // Get required destination size
    size_t destination_size_bytes = 0;
    if (charls_jpegls_decoder_get_destination_size(
            decoder, 0, &destination_size_bytes) != jpegls_errc::success) {
      throw std::runtime_error(
          "charls_jpegls_decoder_get_destination_size() failed");
    }

    // Check the required destination size matches the output buffer's size
    if (destination_size_bytes != output_buffer_size) {
      throw std::runtime_error("Output buffer has incorrect size");
    }

    // Perform decode
    if (charls_jpegls_decoder_decode_to_buffer(decoder, output_buffer,
                                               destination_size_bytes,
                                               0) != jpegls_errc::success) {
      throw std::runtime_error(
          "charls_jpegls_decoder_decode_to_buffer() failed");
    }

    charls_jpegls_decoder_destroy(decoder);

    return 0;
  } catch (const std::runtime_error &e) {
    snprintf(error_buffer, error_buffer_size, "%s", e.what());

    charls_jpegls_decoder_destroy(decoder);

    return 1;
  }
}

extern "C" size_t charls_encode(
    const void *input_data, size_t width, size_t height,
    size_t samples_per_pixel, size_t bits_allocated, size_t near_lossless,
    void *(*output_buffer_allocate)(size_t len, void *ctx),
    void *output_buffer_context, char *error_buffer, size_t error_buffer_size) {
  charls_jpegls_encoder *encoder = nullptr;

  try {
    // Create encoder
    encoder = charls_jpegls_encoder_create();
    if (!encoder) {
      throw std::runtime_error("charls_jpegls_encoder_create() failed");
    }

    // Set encoding quality
    if (charls_jpegls_encoder_set_near_lossless(encoder, near_lossless) !=
        jpegls_errc::success) {
      throw std::runtime_error(
          "charls_jpegls_encoder_set_near_lossless() failed");
    }

    charls_frame_info frame_info = {};
    frame_info.width = static_cast<uint32_t>(width);
    frame_info.height = static_cast<uint32_t>(height);
    frame_info.bits_per_sample = static_cast<int32_t>(bits_allocated);
    frame_info.component_count = static_cast<int32_t>(samples_per_pixel);

    // Set frame into
    if (charls_jpegls_encoder_set_frame_info(encoder, &frame_info) !=
        jpegls_errc::success) {
      throw std::runtime_error("charls_jpegls_encoder_set_frame_info() failed");
    }

    // Estimate output size
    size_t encoded_length = 0;
    if (charls_jpegls_encoder_get_estimated_destination_size(
            encoder, &encoded_length) != jpegls_errc::success) {
      throw std::runtime_error(
          "charls_jpegls_encoder_get_estimated_destination_size() failed");
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

    if (charls_jpegls_encoder_set_destination_buffer(
            encoder, encoded_buffer, encoded_length) != jpegls_errc::success) {
      throw std::runtime_error(
          "charls_jpegls_encoder_set_destination_buffer() failed");
    }

    // Encode the image
    if (charls_jpegls_encoder_encode_from_buffer(
            encoder, input_data,
            width * height * samples_per_pixel * (bits_allocated / 8),
            0) != jpegls_errc::success) {
      throw std::runtime_error(
          "charls_jpegls_encoder_encode_from_buffer() failed");
    }

    // Get the actual size of the encoded data
    size_t bytes_written = 0;
    if (charls_jpegls_encoder_get_bytes_written(encoder, &bytes_written) !=
        jpegls_errc::success) {
      throw std::runtime_error(
          "charls_jpegls_encoder_get_bytes_written() failed");
    }

    charls_jpegls_encoder_destroy(encoder);

    return bytes_written;
  } catch (const std::runtime_error &e) {
    snprintf(error_buffer, error_buffer_size, "%s", e.what());

    charls_jpegls_encoder_destroy(encoder);

    return 0;
  }
}
