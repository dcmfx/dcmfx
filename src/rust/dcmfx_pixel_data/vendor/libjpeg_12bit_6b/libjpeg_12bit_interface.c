// This file contains the C entry points called from Rust to perform 12-bit JPEG
// decoding and encoding.

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#ifndef __wasm__
#include <stdio.h>
#endif

#include "./src/jerror12.h"
#include "./src/jpeglib12.h"

static void output_message(j_common_ptr cinfo) {}
static void error_exit(j_common_ptr cinfo) {}
static void init_source(j_decompress_ptr dinfo) {}
static boolean_result_t fill_input_buffer(j_decompress_ptr dinfo);
static void skip_input_data(j_decompress_ptr dinfo, long num_bytes);
static void term_source(j_decompress_ptr dinfo) {}

// Decodes the given bytes as a 12-bit JPEG.
size_t libjpeg_12bit_decode(const void *input_data, size_t input_data_size,
                            size_t width, size_t height,
                            size_t samples_per_pixel, size_t is_ybr_color_space,
                            uint16_t *output_buffer, size_t output_buffer_size,
                            char error_message[JMSG_LENGTH_MAX]) {
  struct jpeg_decompress_struct dinfo;
  struct jpeg_error_mgr jerr;
  dinfo.err = jpeg_std_error(&jerr);

  // Silence all output messages. Comment out the following line to see any
  // warning messages on stdout.
  dinfo.err->output_message = output_message;

  // Initialize decompression object
  if (jpeg_create_decompress(&dinfo).is_err) {
    strcpy(error_message, "jpeg_create_decompress() failed");
    return 1;
  }

  // Use an in-memory data source
  struct jpeg_source_mgr src;
  memset(&src, 0, sizeof(src));
  src.init_source = init_source;
  src.fill_input_buffer = fill_input_buffer;
  src.skip_input_data = skip_input_data;
  src.resync_to_restart = jpeg_resync_to_restart;
  src.term_source = term_source;
  src.bytes_in_buffer = input_data_size;
  src.next_input_byte = input_data;
  dinfo.src = &src;

  // Read JPEG header
  int_result_t read_result = jpeg_read_header(&dinfo, TRUE);
  if (read_result.is_err || read_result.value != JPEG_HEADER_OK) {
    strcpy(error_message, "jpeg_read_header() failed");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  // Check that the data uses the expected 12-bit precision
  if (dinfo.data_precision != 12) {
    strcpy(error_message, "Data precision is not 12-bit");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  // Start decompression
  if (jpeg_start_decompress(&dinfo).is_err) {
    strcpy(error_message, "jpeg_start_decompress() failed");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  // Set output color space to RGB for color images
  if (dinfo.output_components == 1) {
    dinfo.out_color_space = JCS_GRAYSCALE;
  } else if (dinfo.output_components == 3) {
    if (is_ybr_color_space == 1) {
      dinfo.out_color_space = JCS_YCbCr;
    } else {
      dinfo.out_color_space = JCS_RGB;
    }
  } else {
    strcpy(error_message, "Output components is not 1 or 3");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  // Check image dimensions
  if (dinfo.output_width != width || dinfo.output_height != height ||
      dinfo.output_components != (int)samples_per_pixel) {
    strcpy(
        error_message,
        "Image does not have the expected width, height, or samples per pixel");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  };

  // Check output buffer size
  if (output_buffer_size != width * height * samples_per_pixel) {
    strcpy(error_message, "Output buffer has incorrect size");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  // Allocate buffer to store a single scanline
  size_t row_stride = dinfo.output_width * dinfo.output_components;
  jsamparray_result_t buffer_alloc_result = (*dinfo.mem->alloc_sarray)(
      (j_common_ptr)&dinfo, JPOOL_IMAGE, row_stride, 1);
  if (buffer_alloc_result.is_err) {
    strcpy(error_message, "Scanline allocation failed");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  JSAMPARRAY buffer = buffer_alloc_result.value;

  // Read scanlines and accumulate in the output buffer
  while (dinfo.output_scanline < dinfo.output_height) {
    if (jpeg_read_scanlines(&dinfo, buffer, 1).is_err) {
      strcpy(error_message, "jpeg_read_scanlines() failed");
      jpeg_destroy_decompress(&dinfo);
      return 1;
    }

    memcpy(output_buffer, buffer[0], row_stride * sizeof(JSAMPLE));
    output_buffer += row_stride;
  }

  // Finish decompression
  if (jpeg_finish_decompress(&dinfo).is_err) {
    strcpy(error_message, "jpeg_finish_decompress() failed");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  jpeg_destroy_decompress(&dinfo);

  return 0;
}

static boolean_result_t fill_input_buffer(j_decompress_ptr _dinfo) {
  return RESULT_OK(boolean, FALSE);
}

static void skip_input_data(j_decompress_ptr dinfo, long num_bytes) {
  if (num_bytes <= 0) {
    return;
  }

  if ((size_t)num_bytes > dinfo->src->bytes_in_buffer) {
    num_bytes = (long)dinfo->src->bytes_in_buffer;
  }

  dinfo->src->bytes_in_buffer -= num_bytes;
  dinfo->src->next_input_byte += num_bytes;
}

// Callback that receives compressed data
typedef void (*output_data_callback_t)(const uint8_t *data, size_t len,
                                       void *context);

// This struct defines a JPEG destination that emits chunks to an output data
// callback
typedef struct {
  struct jpeg_destination_mgr pub;

  // Reusable buffer that is sent to the output callback once full
  JOCTET buffer[16384];

  // Output callback and context
  output_data_callback_t output_data_callback;
  void *output_data_context;
} jpeg_mem_destination_mgr;

// Forward declarations
static void_result_t init_destination(j_compress_ptr cinfo);
static boolean_result_t empty_output_buffer(j_compress_ptr cinfo);
static void_result_t term_destination(j_compress_ptr cinfo);
static void jpeg_mem_dest(j_compress_ptr cinfo,
                          output_data_callback_t output_data_callback,
                          void *output_data_context);

// Encodes the given image as a 12-bit JPEG.
size_t libjpeg_12bit_encode(int16_t *input_data, size_t width, size_t height,
                            size_t samples_per_pixel,
                            size_t photometric_interpretation,
                            size_t color_space, size_t quality,
                            output_data_callback_t output_data_callback,
                            void *output_data_context,
                            char error_message[JMSG_LENGTH_MAX]) {

  struct jpeg_compress_struct cinfo;
  struct jpeg_error_mgr jerr;
  cinfo.err = jpeg_std_error(&jerr);
  cinfo.err->error_exit = error_exit;

  // Silence all output messages. Comment out the following line to see any
  // warning messages on stdout.
  cinfo.err->output_message = output_message;

  if (jpeg_create_compress(&cinfo).is_err) {
    strcpy(error_message, "jpeg_create_compress() failed");
    return 1;
  }

  // Setup destination that sends chunks to the output callback
  jpeg_mem_destination_mgr dest;
  memset(&dest, 0, sizeof(dest));
  cinfo.dest = &dest.pub;
  jpeg_mem_dest(&cinfo, output_data_callback, output_data_context);

  // Setup compressor info
  cinfo.image_width = (JDIMENSION)width;
  cinfo.image_height = (JDIMENSION)height;
  cinfo.input_components = (int)samples_per_pixel;
  cinfo.in_color_space = (J_COLOR_SPACE)color_space;

  if (jpeg_set_defaults(&cinfo).is_err) {
    strcpy(error_message, "jpeg_set_defaults() failed");
    jpeg_destroy_compress(&cinfo);
    return 1;
  }

  if (jpeg_set_quality(&cinfo, quality, FALSE).is_err) {
    strcpy(error_message, "jpeg_set_quality() failed");
    jpeg_destroy_compress(&cinfo);
    return 1;
  }

  // Set sampling factors for RGB/YBR_FULL/YBR_FULL_422
  if (samples_per_pixel == 3) {
    if (photometric_interpretation == 3 || photometric_interpretation == 4) {
      cinfo.comp_info[0].h_samp_factor = 1;
    } else if (photometric_interpretation == 5) {
      cinfo.comp_info[0].h_samp_factor = 2;
    }
    cinfo.comp_info[0].v_samp_factor = 1;
    cinfo.comp_info[1].h_samp_factor = 1;
    cinfo.comp_info[1].v_samp_factor = 1;
    cinfo.comp_info[2].h_samp_factor = 1;
    cinfo.comp_info[2].v_samp_factor = 1;
  }

  // Bootstrap the compressor
  if (jpeg_start_compress(&cinfo, TRUE).is_err) {
    strcpy(error_message, "jpeg_start_compress() failed");
    jpeg_destroy_compress(&cinfo);
    return 1;
  }

  JSAMPROW row_pointer[1];
  size_t row_stride = width * samples_per_pixel;

  // Write all scanlines into the compressor
  while (cinfo.next_scanline < cinfo.image_height) {
    row_pointer[0] = &input_data[cinfo.next_scanline * row_stride];
    if (jpeg_write_scanlines(&cinfo, row_pointer, 1).is_err) {
      strcpy(error_message, "jpeg_write_scanlines() failed");
      jpeg_destroy_compress(&cinfo);
      return 1;
    }
  }

  // Finish the compression
  if (jpeg_finish_compress(&cinfo).is_err) {
    strcpy(error_message, "jpeg_finish_compress() failed");
    jpeg_destroy_compress(&cinfo);
    return 1;
  }

  jpeg_destroy_compress(&cinfo);

  return 0;
}

static void_result_t init_destination(j_compress_ptr cinfo) { return OK_VOID; }

static boolean_result_t empty_output_buffer(j_compress_ptr cinfo) {
  jpeg_mem_destination_mgr *dest = (jpeg_mem_destination_mgr *)cinfo->dest;

  dest->output_data_callback(dest->buffer,
                             sizeof(dest->buffer) - dest->pub.free_in_buffer,
                             dest->output_data_context);

  dest->pub.next_output_byte = dest->buffer;
  dest->pub.free_in_buffer = sizeof(dest->buffer);

  return RESULT_OK(boolean, TRUE);
}

static void_result_t term_destination(j_compress_ptr cinfo) {
  jpeg_mem_destination_mgr *dest = (jpeg_mem_destination_mgr *)cinfo->dest;

  dest->output_data_callback(dest->buffer,
                             sizeof(dest->buffer) - dest->pub.free_in_buffer,
                             dest->output_data_context);

  return OK_VOID;
}

static void jpeg_mem_dest(j_compress_ptr cinfo,
                          void (*output_data_callback)(const uint8_t *data,
                                                       size_t len, void *ctx),
                          void *output_data_context) {
  jpeg_mem_destination_mgr *dest = (jpeg_mem_destination_mgr *)cinfo->dest;

  dest->output_data_callback = output_data_callback;
  dest->output_data_context = output_data_context;

  dest->pub.next_output_byte = dest->buffer;
  dest->pub.free_in_buffer = sizeof(dest->buffer);

  dest->pub.init_destination = init_destination;
  dest->pub.empty_output_buffer = empty_output_buffer;
  dest->pub.term_destination = term_destination;
}
