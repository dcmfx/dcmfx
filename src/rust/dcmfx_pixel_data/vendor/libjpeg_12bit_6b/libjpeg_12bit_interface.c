// This file contains the C entry point called from Rust to perform decoding of
// 12-bit JPEG data.

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#ifndef __wasm__
#include <stdio.h>
#endif

#include "./src/jerror12.h"
#include "./src/jpeglib12.h"

static void output_message(j_common_ptr _cinfo) {}

static void init_source(j_decompress_ptr _dinfo) {}

static boolean fill_input_buffer(j_decompress_ptr _dinfo) { return FALSE; }

static void skip_input_data(j_decompress_ptr dinfo, long num_bytes) {
  if (num_bytes <= 0) {
    return;
  }

  if ((size_t)num_bytes > dinfo->src->bytes_in_buffer) {
    num_bytes = dinfo->src->bytes_in_buffer;
  }

  dinfo->src->bytes_in_buffer -= num_bytes;
  dinfo->src->next_input_byte += num_bytes;
}

static void term_source(j_decompress_ptr _dinfo) {}

// Decodes the given bytes as a 12-bit JPEG.
int libjpeg_12bit_decode(uint8_t *jpeg_data, uint64_t jpeg_size, uint32_t width,
                         uint32_t height, uint32_t samples_per_pixel,
                         uint32_t is_ybr_color_space, uint16_t *output_buffer,
                         uint64_t output_buffer_size,
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
    return -1;
  }

  // Use an in-memory data source
  struct jpeg_source_mgr src;
  memset(&src, 0, sizeof(src));
  src.init_source = init_source;
  src.fill_input_buffer = fill_input_buffer;
  src.skip_input_data = skip_input_data;
  src.resync_to_restart = jpeg_resync_to_restart;
  src.term_source = term_source;
  src.bytes_in_buffer = jpeg_size;
  src.next_input_byte = jpeg_data;
  dinfo.src = &src;

  // Read JPEG header
  int_result_t read_result = jpeg_read_header(&dinfo, TRUE);
  if (read_result.is_err || read_result.value != JPEG_HEADER_OK) {
    strcpy(error_message, "jpeg_read_header() failed");
    (void)jpeg_destroy_decompress(&dinfo);
    return -1;
  }

  // Check that the data uses the expected 12-bit precision
  if (dinfo.data_precision != 12) {
    strcpy(error_message, "Data precision is not 12-bit");
    (void)jpeg_destroy_decompress(&dinfo);
    return -1;
  }

  // Start decompression
  if (jpeg_start_decompress(&dinfo).is_err) {
    strcpy(error_message, "jpeg_start_decompress() failed");
    (void)jpeg_destroy_decompress(&dinfo);
    return -1;
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
    (void)jpeg_destroy_decompress(&dinfo);
    return -1;
  }

  // Check image dimensions
  if (dinfo.output_width != width || dinfo.output_height != height ||
      dinfo.output_components != (int)samples_per_pixel) {
    strcpy(error_message,
           "Image does not have the expected width, height, or samples per pixel");
    (void)jpeg_destroy_decompress(&dinfo);
    return -1;
  };

  // Check output buffer size
  if (output_buffer_size != width * height * samples_per_pixel) {
    strcpy(error_message, "Output buffer has incorrect size");
    (void)jpeg_destroy_decompress(&dinfo);
    return -1;
  }

  // Allocate buffer to store a single scanline
  int row_stride = dinfo.output_width * dinfo.output_components;
  jsamparray_result_t buffer_alloc_result = (*dinfo.mem->alloc_sarray)(
      (j_common_ptr)&dinfo, JPOOL_IMAGE, row_stride, 1);
  if (buffer_alloc_result.is_err) {
    strcpy(error_message, "Scanline allocation failed");
    (void)jpeg_destroy_decompress(&dinfo);
    return -1;
  }

  JSAMPARRAY buffer = buffer_alloc_result.value;

  // Read scanlines and accumulate in the output buffer
  while (dinfo.output_scanline < dinfo.output_height) {
    if (jpeg_read_scanlines(&dinfo, buffer, 1).is_err) {
      strcpy(error_message, "jpeg_read_scanlines() failed");
      (void)jpeg_destroy_decompress(&dinfo);
      return -1;
    }

    memcpy(output_buffer, buffer[0], row_stride * sizeof(JSAMPLE));
    output_buffer += row_stride;
  }

  // Clean up
  (void)jpeg_finish_decompress(&dinfo);
  (void)jpeg_destroy_decompress(&dinfo);

  return 0;
}
