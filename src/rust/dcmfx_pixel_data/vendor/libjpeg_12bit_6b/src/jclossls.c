/*
 * jclossls.c
 *
 * Copyright (C) 1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains the control logic for the lossless JPEG compressor.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"
#include "jlossls12.h"


#ifdef C_LOSSLESS_SUPPORTED

/*
 * Initialize for a processing pass.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
start_pass (j_compress_ptr cinfo, J_BUF_MODE pass_mode)
{
  j_lossless_c_ptr losslsc = (j_lossless_c_ptr) cinfo->codec;

  (*losslsc->scaler_start_pass) (cinfo);
  void_result_t predict_start_pass_result = (*losslsc->predict_start_pass) (cinfo);
  if (predict_start_pass_result.is_err) {
    return predict_start_pass_result;
  }
  return (*losslsc->diff_start_pass) (cinfo, pass_mode);
}


/*
 * Initialize the lossless compression codec.
 * This is called only once, during master selection.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_lossless_c_codec(j_compress_ptr cinfo)
{
  j_lossless_c_ptr losslsc;

  /* Create subobject in permanent pool */
  void_ptr_result_t alloc_small_result =
    (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_PERMANENT,
				SIZEOF(jpeg_lossless_c_codec));
  if (alloc_small_result.is_err) {
    return ERR_VOID(alloc_small_result.err_code);
  }
  losslsc = (j_lossless_c_ptr) alloc_small_result.value;
  cinfo->codec = (struct jpeg_c_codec *) losslsc;

  /* Initialize sub-modules */

  /* Scaler */
  jinit_c_scaler(cinfo);

  /* Differencer */
  void_result_t jinit_differencer_result = jinit_differencer(cinfo);
  if (jinit_differencer_result.is_err) {
    return jinit_differencer_result;
  }

  /* Entropy encoding: either Huffman or arithmetic coding. */
  if (cinfo->arith_code) {
#ifdef WITH_ARITHMETIC_PATCH
    jinit_arith_encoder(cinfo);
#else
    ERREXIT(cinfo, JERR_ARITH_NOTIMPL, ERR_VOID);
#endif
  } else {
    void_result_t jinit_lhuff_encoder_result = jinit_lhuff_encoder(cinfo);
    if (jinit_lhuff_encoder_result.is_err) {
      return jinit_lhuff_encoder_result;
    }
  }

  /* Need a full-image difference buffer in any multi-pass mode. */
  void_result_t jinit_c_diff_controller_result = jinit_c_diff_controller(cinfo,
			  (boolean) (cinfo->num_scans > 1 ||
				     cinfo->optimize_coding));
  if (jinit_c_diff_controller_result.is_err) {
    return jinit_c_diff_controller_result;
  }

  /* Initialize method pointers.
   *
   * Note: entropy_start_pass and entropy_finish_pass are assigned in
   * jclhuff.c and compress_data is assigned in jcdiffct.c.
   */
  losslsc->pub.start_pass = start_pass;

  return OK_VOID;
}

#endif /* C_LOSSLESS_SUPPORTED */
