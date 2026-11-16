// This file contains the C entry points called from Rust to perform 8-bit JPEG
// decoding and encoding using libjpeg-turbo.
//
// Errors raised by libjpeg-turbo are handled using setjmp()/longjmp(), which
// means this interface isn't usable on WASM.

#include <setjmp.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "./src/src/jerror.h"
#include "./src/src/jpeglib.h"

// Color spaces passed in from Rust
#define DCMFX_COLOR_SPACE_GRAYSCALE 0
#define DCMFX_COLOR_SPACE_RGB 1
#define DCMFX_COLOR_SPACE_YCBCR 2

// Error manager that jumps back to the calling function on a fatal error, and
// stores the error message so it can be returned to Rust
typedef struct {
  struct jpeg_error_mgr pub;
  jmp_buf setjmp_buffer;
  char *error_message;
} error_mgr;

static void error_exit(j_common_ptr cinfo) {
  error_mgr *err = (error_mgr *)cinfo->err;

  (*cinfo->err->format_message)(cinfo, err->error_message);

  longjmp(err->setjmp_buffer, 1);
}

// Silence all warning and trace messages
static void output_message(j_common_ptr cinfo) { (void)cinfo; }

static J_COLOR_SPACE to_j_color_space(size_t color_space) {
  switch (color_space) {
  case DCMFX_COLOR_SPACE_GRAYSCALE:
    return JCS_GRAYSCALE;
  case DCMFX_COLOR_SPACE_RGB:
    return JCS_RGB;
  case DCMFX_COLOR_SPACE_YCBCR:
    return JCS_YCbCr;
  default:
    return JCS_UNKNOWN;
  }
}

// The maximum number of scanlines passed to libjpeg-turbo in one call
#define MAX_SCANLINES_PER_CALL 16

// Decodes the given bytes as an 8-bit JPEG. The decoded pixels are written to
// the output buffer, which must be exactly width * height * samples_per_pixel
// bytes in size.
//
// The horizontal and vertical sampling factors of the first component are
// returned so the caller can tell whether the chroma components were
// subsampled.
//
// Returns zero on success. On error, a non-zero value is returned and the
// error message is written to `error_message`.
size_t libjpeg_turbo_decode_8bit(const uint8_t *input_data,
                                 size_t input_data_size, size_t width,
                                 size_t height, size_t samples_per_pixel,
                                 size_t output_color_space,
                                 uint8_t *output_buffer,
                                 size_t output_buffer_size,
                                 size_t *h_samp_factor, size_t *v_samp_factor,
                                 char error_message[JMSG_LENGTH_MAX]) {
  struct jpeg_decompress_struct dinfo;
  error_mgr jerr;

  dinfo.err = jpeg_std_error(&jerr.pub);
  jerr.pub.error_exit = error_exit;
  jerr.pub.output_message = output_message;
  jerr.error_message = error_message;

  if (setjmp(jerr.setjmp_buffer)) {
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  jpeg_create_decompress(&dinfo);

  if (input_data_size > (unsigned long)-1) {
    strcpy(error_message, "Input data is too large");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  jpeg_mem_src(&dinfo, input_data, (unsigned long)input_data_size);

  jpeg_read_header(&dinfo, TRUE);

  if (dinfo.data_precision != 8) {
    strcpy(error_message, "Data precision is not 8-bit");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  if (dinfo.image_width != width || dinfo.image_height != height) {
    strcpy(error_message, "JPEG image has incorrect dimensions");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  if (dinfo.num_components != (int)samples_per_pixel) {
    strcpy(error_message, "JPEG image has incorrect number of components");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  *h_samp_factor = (size_t)dinfo.comp_info[0].h_samp_factor;
  *v_samp_factor = (size_t)dinfo.comp_info[0].v_samp_factor;

  dinfo.out_color_space = to_j_color_space(output_color_space);

  jpeg_start_decompress(&dinfo);

  if (dinfo.output_width != width || dinfo.output_height != height ||
      dinfo.output_components != (int)samples_per_pixel) {
    strcpy(error_message, "Image does not have the expected width, height, "
                          "or samples per pixel");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  size_t row_stride = width * samples_per_pixel;
  if (output_buffer_size != row_stride * height) {
    strcpy(error_message, "Output buffer has incorrect size");
    jpeg_destroy_decompress(&dinfo);
    return 1;
  }

  // Decode scanlines directly into the output buffer
  JSAMPROW rows[MAX_SCANLINES_PER_CALL];
  while (dinfo.output_scanline < dinfo.output_height) {
    JDIMENSION remaining = dinfo.output_height - dinfo.output_scanline;
    JDIMENSION count = remaining < MAX_SCANLINES_PER_CALL
                           ? remaining
                           : MAX_SCANLINES_PER_CALL;

    for (JDIMENSION i = 0; i < count; i++) {
      rows[i] = output_buffer + (dinfo.output_scanline + i) * row_stride;
    }

    jpeg_read_scanlines(&dinfo, rows, count);
  }

  jpeg_finish_decompress(&dinfo);
  jpeg_destroy_decompress(&dinfo);

  return 0;
}

// Callback that receives compressed data
typedef void (*output_data_callback_t)(const uint8_t *data, size_t len,
                                       void *context);

// JPEG destination that emits chunks of compressed data to a callback
typedef struct {
  struct jpeg_destination_mgr pub;

  // Reusable buffer that is sent to the output callback once full
  JOCTET buffer[65536];

  output_data_callback_t output_data_callback;
  void *output_data_context;
} callback_destination_mgr;

static void init_destination(j_compress_ptr cinfo) {
  callback_destination_mgr *dest = (callback_destination_mgr *)cinfo->dest;

  dest->pub.next_output_byte = dest->buffer;
  dest->pub.free_in_buffer = sizeof(dest->buffer);
}

static boolean empty_output_buffer(j_compress_ptr cinfo) {
  callback_destination_mgr *dest = (callback_destination_mgr *)cinfo->dest;

  // When this function is called the whole buffer is to be emptied, regardless
  // of the current value of free_in_buffer
  dest->output_data_callback(dest->buffer, sizeof(dest->buffer),
                             dest->output_data_context);

  dest->pub.next_output_byte = dest->buffer;
  dest->pub.free_in_buffer = sizeof(dest->buffer);

  return TRUE;
}

static void term_destination(j_compress_ptr cinfo) {
  callback_destination_mgr *dest = (callback_destination_mgr *)cinfo->dest;

  size_t len = sizeof(dest->buffer) - dest->pub.free_in_buffer;
  if (len > 0) {
    dest->output_data_callback(dest->buffer, len, dest->output_data_context);
  }
}

// Encodes the given image as an 8-bit baseline JPEG. Compressed data is passed
// to the output data callback.
//
// The input color space is one of the DCMFX_COLOR_SPACE_* values. RGB input is
// converted to YCbCr by libjpeg-turbo. YCbCr input is encoded as-is. For color
// images, `is_422` specifies whether the chroma components are horizontally
// subsampled by a factor of two.
//
// Returns zero on success. On error, a non-zero value is returned and the
// error message is written to `error_message`.
size_t libjpeg_turbo_encode_8bit(const uint8_t *input_data, size_t width,
                                 size_t height, size_t samples_per_pixel,
                                 size_t input_color_space, size_t is_422,
                                 size_t quality,
                                 output_data_callback_t output_data_callback,
                                 void *output_data_context,
                                 char error_message[JMSG_LENGTH_MAX]) {
  struct jpeg_compress_struct cinfo;
  error_mgr jerr;
  callback_destination_mgr dest;

  cinfo.err = jpeg_std_error(&jerr.pub);
  jerr.pub.error_exit = error_exit;
  jerr.pub.output_message = output_message;
  jerr.error_message = error_message;

  if (setjmp(jerr.setjmp_buffer)) {
    jpeg_destroy_compress(&cinfo);
    return 1;
  }

  jpeg_create_compress(&cinfo);

  // Setup destination that sends chunks to the output callback
  memset(&dest, 0, sizeof(dest));
  dest.pub.init_destination = init_destination;
  dest.pub.empty_output_buffer = empty_output_buffer;
  dest.pub.term_destination = term_destination;
  dest.output_data_callback = output_data_callback;
  dest.output_data_context = output_data_context;
  cinfo.dest = &dest.pub;

  cinfo.image_width = (JDIMENSION)width;
  cinfo.image_height = (JDIMENSION)height;
  cinfo.input_components = (int)samples_per_pixel;
  cinfo.in_color_space = to_j_color_space(input_color_space);

  jpeg_set_defaults(&cinfo);

  // Force baseline-compatible quantization tables, which is required for the
  // JPEG Baseline (Process 1) transfer syntax
  jpeg_set_quality(&cinfo, (int)quality, TRUE);

  // Set sampling factors for color images, either 4:4:4 or 4:2:2
  if (samples_per_pixel == 3) {
    cinfo.comp_info[0].h_samp_factor = is_422 ? 2 : 1;
    cinfo.comp_info[0].v_samp_factor = 1;
    cinfo.comp_info[1].h_samp_factor = 1;
    cinfo.comp_info[1].v_samp_factor = 1;
    cinfo.comp_info[2].h_samp_factor = 1;
    cinfo.comp_info[2].v_samp_factor = 1;
  }

  jpeg_start_compress(&cinfo, TRUE);

  // Pass scanlines directly from the input buffer
  size_t row_stride = width * samples_per_pixel;
  JSAMPROW rows[MAX_SCANLINES_PER_CALL];
  while (cinfo.next_scanline < cinfo.image_height) {
    JDIMENSION remaining = cinfo.image_height - cinfo.next_scanline;
    JDIMENSION count = remaining < MAX_SCANLINES_PER_CALL
                           ? remaining
                           : MAX_SCANLINES_PER_CALL;

    for (JDIMENSION i = 0; i < count; i++) {
      rows[i] = (JSAMPROW)(input_data + (cinfo.next_scanline + i) * row_stride);
    }

    jpeg_write_scanlines(&cinfo, rows, count);
  }

  jpeg_finish_compress(&cinfo);
  jpeg_destroy_compress(&cinfo);

  return 0;
}
