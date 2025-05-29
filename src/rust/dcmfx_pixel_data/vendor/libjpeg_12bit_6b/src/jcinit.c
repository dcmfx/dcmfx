/*
 * jcinit.c
 *
 * Copyright (C) 1991-1997, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains initialization logic for the JPEG compressor.
 * This routine is in charge of selecting the modules to be executed and
 * making an initialization call to each one.
 *
 * Logically, this code belongs in jcmaster.c.  It's split out because
 * linking this routine implies linking the entire compression library.
 * For a transcoding-only application, we want to be able to use jcmaster.c
 * without linking in the whole library.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"


/*
 * Master selection of compression modules.
 * This is done once at the start of processing an image.  We determine
 * which modules will be used and give them appropriate initialization calls.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_compress_master (j_compress_ptr cinfo)
{
  /* Initialize master control (includes parameter checking/processing) */
  void_result_t jinit_c_master_control_result = jinit_c_master_control(cinfo, FALSE /* full compression */);
  if (jinit_c_master_control_result.is_err) {
    return jinit_c_master_control_result;
  }

  /* Initialize compression codec */
  void_result_t jinit_c_codec_result = jinit_c_codec(cinfo);
  if (jinit_c_codec_result.is_err) {
    return jinit_c_codec_result;
  }

  /* Preprocessing */
  if (! cinfo->raw_data_in) {
    void_result_t jinit_color_converter_result = jinit_color_converter(cinfo);
    if (jinit_color_converter_result.is_err) {
      return jinit_color_converter_result;
    }
    void_result_t jinit_downsampler_result = jinit_downsampler(cinfo);
    if (jinit_downsampler_result.is_err) {
      return jinit_downsampler_result;
    }
    void_result_t jinit_c_prep_controller_result = jinit_c_prep_controller(cinfo, FALSE /* never need full buffer here */);
    if (jinit_c_prep_controller_result.is_err) {
      return jinit_c_prep_controller_result;
    }
  }

  void_result_t jinit_c_main_controller_result = jinit_c_main_controller(cinfo, FALSE /* never need full buffer here */);
  if (jinit_c_main_controller_result.is_err) {
    return jinit_c_main_controller_result;
  }

  void_result_t jinit_marker_writer_result = jinit_marker_writer(cinfo);
  if (jinit_marker_writer_result.is_err) {
    return jinit_marker_writer_result;
  }

  /* We can now tell the memory manager to allocate virtual arrays. */
  void_result_t realize_virt_arrays_result = (*cinfo->mem->realize_virt_arrays) ((j_common_ptr) cinfo);
  if (realize_virt_arrays_result.is_err) {
    return realize_virt_arrays_result;
  }

  /* Write the datastream header (SOI) immediately.
   * Frame and scan headers are postponed till later.
   * This lets application insert special markers after the SOI.
   */
  void_result_t write_file_header_result = (*cinfo->marker->write_file_header) (cinfo);
  if (write_file_header_result.is_err) {
    return write_file_header_result;
  }

  return OK_VOID;
}
