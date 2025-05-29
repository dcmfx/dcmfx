/*
 * jclossy.c
 *
 * Copyright (C) 1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains the control logic for the lossy JPEG compressor.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"
#include "jlossy12.h"


/*
 * Initialize for a processing pass.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
start_pass (j_compress_ptr cinfo, J_BUF_MODE pass_mode)
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;

  void_result_t fdct_start_pass_result = (*lossyc->fdct_start_pass) (cinfo);
  if (fdct_start_pass_result.is_err) {
    return fdct_start_pass_result;
  }
  return (*lossyc->coef_start_pass) (cinfo, pass_mode);
}


/*
 * Initialize the lossy compression codec.
 * This is called only once, during master selection.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_lossy_c_codec (j_compress_ptr cinfo)
{
  j_lossy_c_ptr lossyc;

  /* Create subobject in permanent pool */
  void_ptr_result_t alloc_small_result =
    (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_PERMANENT,
				SIZEOF(jpeg_lossy_c_codec));
  if (alloc_small_result.is_err) {
    return ERR_VOID(alloc_small_result.err_code);
  }
  lossyc = (j_lossy_c_ptr) alloc_small_result.value;
  cinfo->codec = (struct jpeg_c_codec *) lossyc;

  /* Initialize sub-modules */

  /* Forward DCT */
  void_result_t jinit_forward_dct_result = jinit_forward_dct(cinfo);
  if (jinit_forward_dct_result.is_err) {
    return jinit_forward_dct_result;
  }
  /* Entropy encoding: either Huffman or arithmetic coding. */
  if (cinfo->arith_code) {
    ERREXIT(cinfo, JERR_ARITH_NOTIMPL, ERR_VOID);
  } else {
    if (cinfo->process == JPROC_PROGRESSIVE) {
#ifdef C_PROGRESSIVE_SUPPORTED
      void_result_t jinit_phuff_encoder_result = jinit_phuff_encoder(cinfo);
      if (jinit_phuff_encoder_result.is_err) {
        return jinit_phuff_encoder_result;
      }
#else
      ERREXIT(cinfo, JERR_NOT_COMPILED);
#endif
    } else {
      void_result_t jinit_shuff_encoder_result = jinit_shuff_encoder(cinfo);
      if (jinit_shuff_encoder_result.is_err) {
        return jinit_shuff_encoder_result;
      }
    }
  }

  /* Need a full-image coefficient buffer in any multi-pass mode. */
  void_result_t jinit_c_coef_controller_result = jinit_c_coef_controller(cinfo,
			  (boolean) (cinfo->num_scans > 1 ||
				     cinfo->optimize_coding));
  if (jinit_c_coef_controller_result.is_err) {
    return jinit_c_coef_controller_result;
  }

  /* Initialize method pointers.
   *
   * Note: entropy_start_pass and entropy_finish_pass are assigned in
   * jcshuff.c or jcphuff.c and compress_data is assigned in jccoefct.c.
   */
  lossyc->pub.start_pass = start_pass;

  return OK_VOID;
}
