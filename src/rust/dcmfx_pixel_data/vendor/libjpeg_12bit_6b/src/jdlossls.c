/*
 * jdlossls.c
 *
 * Copyright (C) 1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains the control logic for the lossless JPEG decompressor.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"
#include "jlossls12.h"


#ifdef D_LOSSLESS_SUPPORTED

/*
 * Compute output image dimensions and related values.
 */

METHODDEF(void)
calc_output_dimensions (j_decompress_ptr cinfo)
{
  /* Hardwire it to "no scaling" */
  cinfo->output_width = cinfo->image_width;
  cinfo->output_height = cinfo->image_height;
  /* jdinput.c has already initialized codec_data_unit to 1,
   * and has computed unscaled downsampled_width and downsampled_height.
   */
}


/*
 * Initialize for an input processing pass.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
start_input_pass (j_decompress_ptr cinfo)
{
  j_lossless_d_ptr losslsd = (j_lossless_d_ptr) cinfo->codec;

  void_result_t entropy_start_pass = ((*losslsd->entropy_start_pass) (cinfo));
  if (entropy_start_pass.is_err)
    return entropy_start_pass;
  void_result_t predict_start_pass_result = ((*losslsd->predict_start_pass) (cinfo));
  if (predict_start_pass_result.is_err)
    return predict_start_pass_result;
  (*losslsd->scaler_start_pass) (cinfo);
  void_result_t diff_start_input_pass_result = ((*losslsd->diff_start_input_pass) (cinfo));
  if (diff_start_input_pass_result.is_err)
    return diff_start_input_pass_result;

  return OK_VOID;
}


/*
 * Initialize the lossless decompression codec.
 * This is called only once, during master selection.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t) 
jinit_lossless_d_codec(j_decompress_ptr cinfo)
{
  j_lossless_d_ptr losslsd;
  boolean use_c_buffer;

  /* Create subobject in permanent pool */
  void_ptr_result_t alloc_small_result =
    (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_PERMANENT,
				SIZEOF(jpeg_lossless_d_codec));
  if (alloc_small_result.is_err)
    return ERR_VOID(alloc_small_result.err_code);
  losslsd = (j_lossless_d_ptr) alloc_small_result.value;
  cinfo->codec = (struct jpeg_d_codec *) losslsd;

  /* Initialize sub-modules */
  /* Entropy decoding: either Huffman or arithmetic coding. */
  if (cinfo->arith_code) {
#ifdef WITH_ARITHMETIC_PATCH
    jinit_arith_decoder(cinfo);
#else
    ERREXIT(cinfo, JERR_ARITH_NOTIMPL, ERR_VOID);
#endif
  } else {
    void_result_t jinit_lhuff_decoder_result = jinit_lhuff_decoder(cinfo);
    if (jinit_lhuff_decoder_result.is_err)
      return jinit_lhuff_decoder_result;
  }

  /* Undifferencer */
  jinit_undifferencer(cinfo);

  /* Scaler */
  void_result_t jinit_d_scaler_result = jinit_d_scaler(cinfo);
  if (jinit_d_scaler_result.is_err)
    return jinit_d_scaler_result;

  use_c_buffer = cinfo->inputctl->has_multiple_scans || cinfo->buffered_image;
  void_result_t jinit_d_diff_controller_result = jinit_d_diff_controller(cinfo, use_c_buffer);
  if (jinit_d_diff_controller_result.is_err)
    return jinit_d_diff_controller_result;

  /* Initialize method pointers.
   *
   * Note: consume_data, start_output_pass and decompress_data are
   * assigned in jddiffct.c.
   */
  losslsd->pub.calc_output_dimensions = calc_output_dimensions;
  losslsd->pub.start_input_pass = start_input_pass;

  return OK_VOID;
}

#endif /* D_LOSSLESS_SUPPORTED */
