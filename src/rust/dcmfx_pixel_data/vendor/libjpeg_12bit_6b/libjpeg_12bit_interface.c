// This file contains the C entry point called from Rust to perform decoding of
// 12-bit JPEG data.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef __wasm__
#include <setjmp.h>
#endif

#include "./src/jpeglib12.h"
#include "./src/jerror12.h"

struct my_error_mgr
{
  struct jpeg_error_mgr pub;

#ifndef __wasm__
  jmp_buf setjmp_buffer;
#endif
};

#ifndef __wasm__
// This function is called from inside libjpeg to terminate JPEG decoding on an
// error. It outputs the message and then longjmp's back to exit from the
// decoding function higher up the stack.
static void on_jpeg_error(j_common_ptr cinfo)
{
  (*cinfo->err->output_message)(cinfo);

  longjmp(((struct my_error_mgr *)cinfo->err)->setjmp_buffer, 1);
}
#endif

static void output_message(j_common_ptr _cinfo) {}

static void init_source(j_decompress_ptr _dinfo) {}

static boolean fill_input_buffer(j_decompress_ptr _dinfo)
{
  return FALSE;
}

static void skip_input_data(j_decompress_ptr dinfo, long num_bytes)
{
  if (num_bytes <= 0)
    return;

  if ((size_t)num_bytes > dinfo->src->bytes_in_buffer)
    num_bytes = dinfo->src->bytes_in_buffer;

  dinfo->src->bytes_in_buffer -= num_bytes;
  dinfo->src->next_input_byte += num_bytes;
}

static void term_source(j_decompress_ptr _dinfo) {}

// Decodes the given bytes as a 12-bit JPEG.
int ijg_decode_jpeg_12bit(unsigned char *jpeg_data, size_t jpeg_size,
                          int *width, int *height, int *channels,
                          JSAMPLE *output_buffer,
                          size_t output_buffer_size,
                          char error_message[JMSG_LENGTH_MAX])
{
  struct jpeg_decompress_struct dinfo;

  struct my_error_mgr jerr;
  dinfo.err = jpeg_std_error(&jerr.pub);

  // Silence all output messages. Comment out the following line to see any
  // warning messages on stdout.
  jerr.pub.output_message = output_message;

  // Setup jump-based error handling
#ifndef __wasm__
  jerr.pub.error_exit = on_jpeg_error;
  if (setjmp(jerr.setjmp_buffer))
  {
    // Put error details into the error message output
    (dinfo.err->format_message)((j_common_ptr)&dinfo, error_message);

    jpeg_destroy_decompress(&dinfo);
    return -1;
  }
#endif

  // Initialize decompression object
  jpeg_create_decompress(&dinfo);

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
  if (jpeg_read_header(&dinfo, TRUE) != JPEG_HEADER_OK)
  {
    strcpy(error_message, "Failed reading JPEG header");
    return -1;
  }

  // Check that the data uses the expected 12-bit precision
  if (dinfo.data_precision != 12)
  {
    sprintf(error_message, "Data precision is not 12-bit", dinfo.data_precision);
    return -1;
  }

  // Start decompression
  jpeg_start_decompress(&dinfo);

  // Set output color space to RGB for color images
  if (dinfo.output_components == 1)
    dinfo.out_color_space = JCS_GRAYSCALE;
  else if (dinfo.output_components == 3)
    dinfo.out_color_space = JCS_RGB;
  else
  {
    jpeg_destroy_decompress(&dinfo);

    sprintf(
        error_message,
        "Output components is %i but only 1 and 3 components are supported",
        dinfo.output_components);

    return -1;
  }

  // Get image dimensions and allocate output buffer
  *width = dinfo.output_width;
  *height = dinfo.output_height;
  *channels = dinfo.output_components;
  if (output_buffer_size < *width * *height * *channels)
  {
    jpeg_destroy_decompress(&dinfo);
    strcpy(error_message, "Output buffer is too small");
    return -1;
  }

  // Allocate buffer to store a single scanline
  int row_stride = dinfo.output_width * dinfo.output_components;
  JSAMPARRAY buffer = (*dinfo.mem->alloc_sarray)((j_common_ptr)&dinfo, JPOOL_IMAGE,
                                                 row_stride, 1);

  // Read scanlines and accumulate in the output buffer
  while (dinfo.output_scanline < dinfo.output_height)
  {
    jpeg_read_scanlines(&dinfo, buffer, 1);
    memcpy(output_buffer, buffer[0], row_stride * sizeof(JSAMPLE));
    output_buffer += row_stride;
  }

  // Clean up
  jpeg_finish_decompress(&dinfo);
  jpeg_destroy_decompress(&dinfo);

  return 0;
}
