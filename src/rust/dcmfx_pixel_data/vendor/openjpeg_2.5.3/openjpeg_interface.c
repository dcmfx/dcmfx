// This file contains the C entry point called from Rust to perform decoding of
// JPEG 2000 data.

#ifndef __wasm__
#include <stdio.h>
#endif
#include <stdlib.h>
#include <string.h>

#include <openjpeg.h>

static const uint8_t JP2_RFC3745_MAGIC[] = {0x00, 0x00, 0x00, 0x0c, 0x6a, 0x50,
                                            0x20, 0x20, 0x0d, 0x0a, 0x87, 0x0a};
static const uint8_t JP2_MAGIC[] = {0x0d, 0x0a, 0x87, 0x0a};
static const uint8_t J2K_CODESTREAM_MAGIC[] = {0xff, 0x4f, 0xff, 0x51};

#define ERROR_DETAILS_SIZE 128

static void error_handler(char const *msg, void *client_data) {
  char *error_details = (char *)client_data;
  strncpy(error_details, msg, ERROR_DETAILS_SIZE - 1);
}

typedef struct {
  uint8_t *data;
  uint64_t data_length;
  uint64_t offset;
} openjpeg_data_source;

size_t stream_read(void *p_buffer, size_t n_bytes, void *p_user_data) {
  openjpeg_data_source *data_source = (openjpeg_data_source *)p_user_data;

  if (n_bytes == 0 || data_source->offset >= data_source->data_length) {
    return -1;
  }

  size_t remaining_data = data_source->data_length - data_source->offset;
  size_t read_length = n_bytes <= remaining_data ? n_bytes : remaining_data;
  memcpy(p_buffer, data_source->data + data_source->offset, read_length);

  data_source->offset += read_length;

  return read_length;
}

OPJ_OFF_T stream_skip(OPJ_OFF_T n_bytes, void *p_user_data) {
  openjpeg_data_source *data_source = (openjpeg_data_source *)p_user_data;

  int64_t original_offset = (int64_t)data_source->offset;
  int64_t new_offset = original_offset + n_bytes;

  if (new_offset < 0) {
    new_offset = 0;
  } else if ((uint64_t)new_offset > data_source->data_length) {
    new_offset = data_source->data_length;
  }

  data_source->offset = new_offset;

  return new_offset - original_offset;
}

OPJ_BOOL stream_seek(OPJ_OFF_T n_bytes, void *p_user_data) {
  openjpeg_data_source *data_source = (openjpeg_data_source *)p_user_data;

  if (n_bytes < 0) {
    return OPJ_FALSE;
  } else if (n_bytes > (int64_t)data_source->data_length) {
    n_bytes = (int64_t)data_source->data_length;
  }

  data_source->offset = n_bytes;

  return OPJ_TRUE;
}

static void cleanup(opj_stream_t stream, opj_codec_t *codec, opj_image_t *image,
                    char *error_buffer, uint32_t error_buffer_size, char *error,
                    char *error_details) {
  opj_stream_destroy(stream);
  opj_destroy_codec(codec);
  opj_image_destroy(image);

  if (error_buffer != NULL && error != NULL) {
    strncpy(error_buffer, error, error_buffer_size - 1);

    // If there are error details present then append them to the error buffer
    if (error_details != NULL && strlen(error_details) > 0) {
      int chars_remaining = error_buffer_size - strlen(error_buffer) - 1;
      if (chars_remaining < 7) {
        return;
      }

      strcat(error_buffer, " with \"");
      chars_remaining -= 7;

      // Append the details
      while (1) {
        if (*error_details == 0 || *error_details == '\n' ||
            chars_remaining <= 0) {
          break;
        }

        char c[] = {*error_details, 0};
        strncat(error_buffer, c, chars_remaining);

        error_details++;
        chars_remaining--;
      }

      if (chars_remaining > 0) {
        strcat(error_buffer, "\"");
      }
    }
  }
}

int openjpeg_decode(uint8_t *input_data, uint64_t input_data_size,
                    uint32_t width, uint32_t height, uint32_t samples_per_pixel,
                    uint32_t bits_allocated, uint8_t *pixel_representation,
                    uint8_t *output_data, uint64_t output_data_size,
                    char *error_buffer, uint32_t error_buffer_size) {
  // Determine codec by looking at the initial bytes of the input data
  int codec_format = OPJ_CODEC_UNKNOWN;
  if ((input_data_size >= 12 &&
       memcmp(input_data, JP2_RFC3745_MAGIC, 12) == 0) ||
      (input_data_size >= 4 && memcmp(input_data, JP2_MAGIC, 4) == 0)) {
    codec_format = OPJ_CODEC_JP2;
  } else if (input_data_size >= 4 &&
             memcmp(input_data, J2K_CODESTREAM_MAGIC, 4) == 0) {
    codec_format = OPJ_CODEC_J2K;
  } else {
    strcpy(error_buffer, "Input is not JPEG 2000 data");
    return -1;
  }

  // Create decompressor for the codec format
  opj_codec_t *codec = opj_create_decompress(codec_format);
  if (codec == NULL) {
    strcpy(error_buffer, "opj_create_decompress() failed");
    return -1;
  }

  // Setup error handler that captures detailed error messages
  char error_details[ERROR_DETAILS_SIZE] = {0};
  opj_set_error_handler(codec, error_handler, error_details);

  // Setup decoder
  opj_dparameters_t parameters;
  opj_set_default_decoder_parameters(&parameters);
  if (!opj_setup_decoder(codec, &parameters)) {
    cleanup(NULL, codec, NULL, error_buffer, error_buffer_size,
            "opj_setup_decoder() failed", error_details);
    return -1;
  }

  // Create and setup a stream to read from the input data
  opj_stream_t *stream = opj_stream_create(OPJ_J2K_STREAM_CHUNK_SIZE, 1);
  if (stream == NULL) {
    cleanup(stream, codec, NULL, error_buffer, error_buffer_size,
            "opj_stream_create() failed", error_details);
    return -1;
  }

  openjpeg_data_source data_source = {input_data, input_data_size, 0};
  opj_stream_set_user_data(stream, &data_source, NULL);
  opj_stream_set_user_data_length(stream, input_data_size);
  opj_stream_set_read_function(stream, stream_read);
  opj_stream_set_skip_function(stream, stream_skip);
  opj_stream_set_seek_function(stream, stream_seek);

  // Read the header
  opj_image_t *image = NULL;
  if (!opj_read_header(stream, codec, &image)) {
    cleanup(stream, codec, image, error_buffer, error_buffer_size,
            "opj_read_header() failed", error_details);
    return -1;
  }

  // Validate that the dimensions and samples per pixel are as expected
  if (image->x1 != width || image->y1 != height ||
      image->numcomps != samples_per_pixel) {
    cleanup(
        stream, codec, image, error_buffer, error_buffer_size,
        "Image does not have the expected width, height, or samples per pixel",
        error_details);
    return -1;
  }

  // Return the pixel representation of the data being read
  *pixel_representation = (uint8_t)image->comps[0].sgnd;

  // Validate that each component has a valid precision
  for (uint32_t i = 0; i < image->numcomps; i++) {
    if (image->comps[i].prec > bits_allocated) {
      cleanup(stream, codec, image, error_buffer, error_buffer_size,
              "Image precision exceeds the bits allocated",
              error_details);
      return -1;
    }
  }

  // Perform decode
  if (!opj_decode(codec, stream, image)) {
    cleanup(stream, codec, image, error_buffer, error_buffer_size,
            "opj_decode() failed", error_details);
    return -1;
  }

  // Clean up decompressor
  if (!opj_end_decompress(codec, stream)) {
    cleanup(stream, codec, image, error_buffer, error_buffer_size,
            "opj_end_decompress() failed", error_details);
    return -1;
  }

  // Copy decoded pixels into the output data
  if (image->numcomps == 1) {
    if (bits_allocated == 1 || bits_allocated == 8) {
      if (output_data_size != width * height) {
        cleanup(stream, codec, image, error_buffer, error_buffer_size,
                "Output data is not the expected size", error_details);
        return -1;
      }

      if (pixel_representation == 0) {
        for (uint32_t i = 0; i < width * height; i++) {
          output_data[i] = image->comps[0].data[i];
        }
      } else {
        for (uint32_t i = 0; i < width * height; i++) {
          ((int8_t *)output_data)[i] = image->comps[0].data[i];
        }
      }
    } else if (bits_allocated == 16) {
      if (output_data_size != width * height * 2) {
        cleanup(stream, codec, image, error_buffer, error_buffer_size,
                "Output data is not the expected size", error_details);
        return -1;
      }

      if (pixel_representation == 0) {
        for (uint32_t i = 0; i < width * height; i++) {
          ((uint16_t *)output_data)[i] = image->comps[0].data[i];
        }
      } else {
        for (uint32_t i = 0; i < width * height; i++) {
          ((int16_t *)output_data)[i] = image->comps[0].data[i];
        }
      }
    } else if (bits_allocated == 32) {
      if (output_data_size != width * height * 4) {
        cleanup(stream, codec, image, error_buffer, error_buffer_size,
                "Output data is not the expected size", error_details);
        return -1;
      }

      if (pixel_representation == 0) {
        for (uint32_t i = 0; i < width * height; i++) {
          ((uint32_t *)output_data)[i] = image->comps[0].data[i];
        }
      } else {
        for (uint32_t i = 0; i < width * height; i++) {
          ((int32_t *)output_data)[i] = image->comps[0].data[i];
        }
      }
    } else {
      cleanup(stream, codec, image, error_buffer, error_buffer_size,
              "Precision not supported", error_details);
      return -1;
    }
  } else if (image->numcomps == 3) {
    OPJ_INT32 *red_data = image->comps[0].data;
    OPJ_INT32 *green_data = image->comps[1].data;
    OPJ_INT32 *blue_data = image->comps[2].data;

    if (bits_allocated == 1 || bits_allocated == 8) {
      if (output_data_size != width * height * 3) {
        cleanup(stream, codec, image, error_buffer, error_buffer_size,
                "Output data is not the expected size", error_details);
        return -1;
      }

      for (uint32_t i = 0; i < width * height; i++) {
        output_data[i * 3] = red_data[i];
        output_data[i * 3 + 1] = green_data[i];
        output_data[i * 3 + 2] = blue_data[i];
      }
    } else if (bits_allocated == 16) {
      if (output_data_size != width * height * 2 * 3) {
        cleanup(stream, codec, image, error_buffer, error_buffer_size,
                "Output data is not the expected size", error_details);
        return -1;
      }

      for (uint32_t i = 0; i < width * height; i++) {
        ((uint16_t *)output_data)[i * 3] = red_data[i];
        ((uint16_t *)output_data)[i * 3 + 1] = green_data[i];
        ((uint16_t *)output_data)[i * 3 + 2] = blue_data[i];
      }
    } else if (bits_allocated == 32) {
      if (output_data_size != width * height * 4 * 3) {
        cleanup(stream, codec, image, error_buffer, error_buffer_size,
                "Output data is not the expected size", error_details);
        return -1;
      }

      for (uint32_t i = 0; i < width * height; i++) {
        ((uint32_t *)output_data)[i * 3] = red_data[i];
        ((uint32_t *)output_data)[i * 3 + 1] = green_data[i];
        ((uint32_t *)output_data)[i * 3 + 2] = blue_data[i];
      }
    } else {
      cleanup(stream, codec, image, error_buffer, error_buffer_size,
              "Precision not supported", error_details);
      return -1;
    }
  } else {
    cleanup(stream, codec, image, error_buffer, error_buffer_size,
            "Number of components not supported", error_details);
    return -1;
  }

  cleanup(stream, codec, image, NULL, 0, NULL, NULL);

  return 0;
}
