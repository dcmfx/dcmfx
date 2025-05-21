// This file contains the C entry point called from Rust to perform decoding of
// JPEG 2000 data.

#include <math.h>
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

static void cleanup(opj_codec_t *codec, opj_stream_t *stream,
                    opj_image_t *image, char *error_buffer,
                    uint32_t error_buffer_size, char *error,
                    char *error_details) {
  opj_image_destroy(image);
  opj_stream_destroy(stream);
  opj_destroy_codec(codec);

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

int32_t openjpeg_decode(uint8_t *input_data, uint64_t input_data_size,
                        uint32_t width, uint32_t height,
                        uint32_t samples_per_pixel, uint32_t bits_allocated,
                        uint8_t *pixel_representation, uint8_t *output_data,
                        uint64_t output_data_size, char *error_buffer,
                        uint32_t error_buffer_size) {
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
    cleanup(codec, NULL, NULL, error_buffer, error_buffer_size,
            "opj_setup_decoder() failed", error_details);
    return -1;
  }

  // Create and setup a stream to read from the input data
  opj_stream_t *stream = opj_stream_create(OPJ_J2K_STREAM_CHUNK_SIZE, 1);
  if (stream == NULL) {
    cleanup(codec, stream, NULL, error_buffer, error_buffer_size,
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
    cleanup(codec, stream, image, error_buffer, error_buffer_size,
            "opj_read_header() failed", error_details);
    return -1;
  }

  // Validate that the dimensions and samples per pixel are as expected
  if (image->x1 != width || image->y1 != height ||
      image->numcomps != samples_per_pixel) {
    cleanup(codec, stream, image, error_buffer, error_buffer_size,
            "Image does not have the expected dimensions or samples per pixel",
            error_details);
    return -1;
  }

  // Return the pixel representation of the data being read
  *pixel_representation = (uint8_t)image->comps[0].sgnd;

  // Validate each image component
  for (uint32_t i = 0; i < image->numcomps; i++) {
    if (image->comps[i].prec > bits_allocated) {
      cleanup(codec, stream, image, error_buffer, error_buffer_size,
              "Image component precision exceeds the bits allocated",
              error_details);
      return -1;
    }

    if (image->comps[i].w != width || image->comps[i].h != height) {
      cleanup(codec, stream, image, error_buffer, error_buffer_size,
              "Image component does not have the expected dimensions",
              error_details);
      return -1;
    }
  }

  // Perform decode
  if (!opj_decode(codec, stream, image)) {
    cleanup(codec, stream, image, error_buffer, error_buffer_size,
            "opj_decode() failed", error_details);
    return -1;
  }

  // Clean up decompressor
  if (!opj_end_decompress(codec, stream)) {
    cleanup(codec, stream, image, error_buffer, error_buffer_size,
            "opj_end_decompress() failed", error_details);
    return -1;
  }

  // Copy decoded pixels into the output data
  if (image->numcomps == 1) {
    if (bits_allocated == 1 || bits_allocated == 8) {
      if (output_data_size != width * height) {
        cleanup(codec, stream, image, error_buffer, error_buffer_size,
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
        cleanup(codec, stream, image, error_buffer, error_buffer_size,
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
        cleanup(codec, stream, image, error_buffer, error_buffer_size,
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
      cleanup(codec, stream, image, error_buffer, error_buffer_size,
              "Precision not supported", error_details);
      return -1;
    }
  } else if (image->numcomps == 3) {
    OPJ_INT32 *red_data = image->comps[0].data;
    OPJ_INT32 *green_data = image->comps[1].data;
    OPJ_INT32 *blue_data = image->comps[2].data;

    if (bits_allocated == 1 || bits_allocated == 8) {
      if (output_data_size != width * height * 3) {
        cleanup(codec, stream, image, error_buffer, error_buffer_size,
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
        cleanup(codec, stream, image, error_buffer, error_buffer_size,
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
        cleanup(codec, stream, image, error_buffer, error_buffer_size,
                "Output data is not the expected size", error_details);
        return -1;
      }

      for (uint32_t i = 0; i < width * height; i++) {
        ((uint32_t *)output_data)[i * 3] = red_data[i];
        ((uint32_t *)output_data)[i * 3 + 1] = green_data[i];
        ((uint32_t *)output_data)[i * 3 + 2] = blue_data[i];
      }
    } else {
      cleanup(codec, stream, image, error_buffer, error_buffer_size,
              "Precision not supported", error_details);
      return -1;
    }
  } else {
    cleanup(codec, stream, image, error_buffer, error_buffer_size,
            "Number of components not supported", error_details);
    return -1;
  }

  cleanup(codec, stream, image, NULL, 0, NULL, NULL);

  return 0;
}

// This output stream directs all writes it receives through a callback that
// is implemented on the Rust side where the data is accumulated in a Vec<u8>.
typedef struct {
  void (*callback)(const uint8_t *data, uint32_t len, void *ctx);
  void *context;
} output_stream_t;

static OPJ_SIZE_T output_stream_write(void *p_buffer, OPJ_SIZE_T p_size,
                                      void *p_user_data) {
  output_stream_t *stream = (output_stream_t *)p_user_data;
  stream->callback(p_buffer, p_size, stream->context);

  return p_size;
}

static void output_stream_free(void *p_user_data) {}

int32_t openjpeg_encode(
    uint8_t *input_data, uint64_t input_data_size, uint32_t width,
    uint32_t height, uint32_t samples_per_pixel, uint32_t bits_allocated,
    uint32_t bits_stored, uint8_t pixel_representation,
    uint32_t color_photometric_interpretation, float tcp_distoratio,
    void (*output_data_callback)(const uint8_t *data, uint32_t len, void *ctx),
    void *output_data_context, char *error_buffer, uint32_t error_buffer_size) {
  // Create compressor
  opj_codec_t *codec = opj_create_compress(OPJ_CODEC_J2K);
  if (codec == NULL) {
    strcpy(error_buffer, "opj_create_compress() failed");
    return -1;
  }

  // Setup error handler that captures detailed error messages
  char error_details[ERROR_DETAILS_SIZE] = {0};
  opj_set_error_handler(codec, error_handler, error_details);

  // Configure encoder parameters
  opj_cparameters_t parameters;
  opj_set_default_encoder_parameters(&parameters);
  parameters.tcp_numlayers = 1;

  // Configure lossy encoding if quality != 0
  if (tcp_distoratio != 0) {
    parameters.cp_fixed_quality = 1;
    parameters.irreversible = 1;
    parameters.tcp_distoratio[0] = tcp_distoratio;
  }

  // Set number of resolutions such that the lowest resolution will be 64x64 in
  // order to avoid over-decomposition
  uint32_t min_dimension = width < height ? width : height;
  if (min_dimension < 64) {
    min_dimension = 64;
  }
  parameters.numresolution = floor(log2(min_dimension / 64)) + 1;
  if (parameters.numresolution > 6) {
    parameters.numresolution = 6;
  }

  // Determine color space and setup compressor parameters appropriately for it
  OPJ_COLOR_SPACE color_space = OPJ_CLRSPC_SYCC;
  if (samples_per_pixel == 3) {
    if (color_photometric_interpretation == 1) { // RGB
      color_space = OPJ_CLRSPC_SRGB;
      parameters.tcp_mct = 0;
    } else if (color_photometric_interpretation == 2) { // YBR_FULL
      parameters.tcp_mct = 0;
    } else if (color_photometric_interpretation == 3) { // YBR_ICT
      parameters.tcp_mct = 1;
    } else if (color_photometric_interpretation == 4) { // YBR_RCT
      parameters.irreversible = 0;
      parameters.tcp_mct = 1;
    } else {
      cleanup(codec, NULL, NULL, error_buffer, error_buffer_size,
              "Invalid color_photometric_interpretation", error_details);
      return -1;
    }
  } else {
    color_space = OPJ_CLRSPC_GRAY;
  }

  // Create image component specifications
  opj_image_cmptparm_t component_parameters[3];
  for (uint32_t i = 0; i < samples_per_pixel; i++) {
    component_parameters[i].dx = 1;
    component_parameters[i].dy = 1;
    component_parameters[i].w = width;
    component_parameters[i].h = height;
    component_parameters[i].x0 = 0;
    component_parameters[i].y0 = 0;
    component_parameters[i].sgnd = pixel_representation;
    component_parameters[i].prec = bits_allocated;
  }

  // Create image to compress
  opj_image_t *image =
      opj_image_create(samples_per_pixel, component_parameters, color_space);
  if (image == NULL) {
    cleanup(codec, NULL, NULL, error_buffer, error_buffer_size,
            "opj_image_create() failed", error_details);
    return -1;
  }

  // Set reference grid dimensions
  image->x1 = width;
  image->y1 = height;

  // Set image content
  size_t index = 0;
  if (bits_allocated == 8) {
    if (pixel_representation == 0) {
      for (uint32_t y = 0; y < height; y++) {
        for (uint32_t x = 0; x < width; x++, index++) {
          for (uint32_t i = 0; i < samples_per_pixel; i++) {
            image->comps[i].data[index] =
                input_data[samples_per_pixel * index + i];
          }
        }
      }
    } else {
      for (uint32_t y = 0; y < height; y++) {
        for (uint32_t x = 0; x < width; x++, index++) {
          for (uint32_t i = 0; i < samples_per_pixel; i++) {
            image->comps[i].data[index] =
                ((int8_t *)input_data)[samples_per_pixel * index + i];
          }
        }
      }
    }
  } else if (bits_allocated == 16) {
    if (pixel_representation == 0) {
      for (uint32_t y = 0; y < height; y++) {
        for (uint32_t x = 0; x < width; x++, index++) {
          for (uint32_t i = 0; i < samples_per_pixel; i++) {
            image->comps[i].data[index] =
                ((uint16_t *)input_data)[samples_per_pixel * index + i];
          }
        }
      }
    } else {
      for (uint32_t y = 0; y < height; y++) {
        for (uint32_t x = 0; x < width; x++, index++) {
          for (uint32_t i = 0; i < samples_per_pixel; i++) {
            image->comps[i].data[index] =
                ((int16_t *)input_data)[samples_per_pixel * index + i];
          }
        }
      }
    }
  } else {
    cleanup(codec, NULL, image, error_buffer, error_buffer_size,
            "Bits allocated value is not 8 or 16", error_details);
    return -1;
  }

  // Setup encoder
  if (!opj_setup_encoder(codec, &parameters, image)) {
    cleanup(codec, NULL, image, error_buffer, error_buffer_size,
            "opj_setup_encoder() failed", error_details);
    return -1;
  }

  output_stream_t output_stream = {output_data_callback, output_data_context};

  // Create and setup a stream to receive the compressed data
  opj_stream_t *stream =
      opj_stream_create(OPJ_J2K_STREAM_CHUNK_SIZE, OPJ_FALSE);
  if (stream == NULL) {
    cleanup(codec, stream, image, error_buffer, error_buffer_size,
            "opj_stream_create() failed", error_details);
    return -1;
  }

  opj_stream_set_write_function(stream, output_stream_write);
  opj_stream_set_user_data(stream, &output_stream, output_stream_free);
  opj_stream_set_user_data_length(stream, (OPJ_UINT64)-1);

  // Start compressor
  if (!opj_start_compress(codec, image, stream)) {
    cleanup(codec, stream, image, error_buffer, error_buffer_size,
            "opj_start_compress() failed", error_details);
    return -1;
  }

  // Perform encode
  if (!opj_encode(codec, stream)) {
    cleanup(codec, stream, image, error_buffer, error_buffer_size,
            "opj_encode() failed", error_details);
    return -1;
  }

  // End compression
  if (!opj_end_compress(codec, stream)) {
    cleanup(codec, stream, image, error_buffer, error_buffer_size,
            "opj_end_compress() failed", error_details);
    return -1;
  }

  cleanup(codec, stream, image, NULL, 0, NULL, NULL);

  return 0;
}
