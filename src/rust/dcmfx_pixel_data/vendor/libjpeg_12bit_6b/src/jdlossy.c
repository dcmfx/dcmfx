/*
 * jdlossy.c
 *
 * Copyright (C) 1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains the control logic for the lossy JPEG decompressor.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"
#include "jlossy12.h"


/*
 * Compute output image dimensions and related values.
 */

METHODDEF(void)
calc_output_dimensions (j_decompress_ptr cinfo)
{
#ifdef IDCT_SCALING_SUPPORTED
  int ci;
  jpeg_component_info *compptr;

  /* Compute actual output image dimensions and DCT scaling choices. */
  if (cinfo->scale_num * 8 <= cinfo->scale_denom) {
    /* Provide 1/8 scaling */
    cinfo->output_width = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_width, 8L);
    cinfo->output_height = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_height, 8L);
    cinfo->min_codec_data_unit = 1;
  } else if (cinfo->scale_num * 4 <= cinfo->scale_denom) {
    /* Provide 1/4 scaling */
    cinfo->output_width = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_width, 4L);
    cinfo->output_height = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_height, 4L);
    cinfo->min_codec_data_unit = 2;
  } else if (cinfo->scale_num * 2 <= cinfo->scale_denom) {
    /* Provide 1/2 scaling */
    cinfo->output_width = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_width, 2L);
    cinfo->output_height = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_height, 2L);
    cinfo->min_codec_data_unit = 4;
  } else {
    /* Provide 1/1 scaling */
    cinfo->output_width = cinfo->image_width;
    cinfo->output_height = cinfo->image_height;
    cinfo->min_codec_data_unit = DCTSIZE;
  }
  /* In selecting the actual DCT scaling for each component, we try to
   * scale up the chroma components via IDCT scaling rather than upsampling.
   * This saves time if the upsampler gets to use 1:1 scaling.
   * Note this code assumes that the supported DCT scalings are powers of 2.
   */
  for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    int ssize = cinfo->min_codec_data_unit;
    while (ssize < DCTSIZE &&
	   (compptr->h_samp_factor * ssize * 2 <=
	    cinfo->max_h_samp_factor * cinfo->min_codec_data_unit) &&
	   (compptr->v_samp_factor * ssize * 2 <=
	    cinfo->max_v_samp_factor * cinfo->min_codec_data_unit)) {
      ssize = ssize * 2;
    }
    compptr->codec_data_unit = ssize;
  }

  /* Recompute downsampled dimensions of components;
   * application needs to know these if using raw downsampled data.
   */
  for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    /* Size in samples, after IDCT scaling */
    compptr->downsampled_width = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_width *
		    (long) (compptr->h_samp_factor * compptr->codec_data_unit),
		    (long) (cinfo->max_h_samp_factor * DCTSIZE));
    compptr->downsampled_height = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_height *
		    (long) (compptr->v_samp_factor * compptr->codec_data_unit),
		    (long) (cinfo->max_v_samp_factor * DCTSIZE));
  }

#else /* !IDCT_SCALING_SUPPORTED */

  /* Hardwire it to "no scaling" */
  cinfo->output_width = cinfo->image_width;
  cinfo->output_height = cinfo->image_height;
  /* jdinput.c has already initialized codec_data_unit to DCTSIZE,
   * and has computed unscaled downsampled_width and downsampled_height.
   */

#endif /* IDCT_SCALING_SUPPORTED */
}


/*
 * Save away a copy of the Q-table referenced by each component present
 * in the current scan, unless already saved during a prior scan.
 *
 * In a multiple-scan JPEG file, the encoder could assign different components
 * the same Q-table slot number, but change table definitions between scans
 * so that each component uses a different Q-table.  (The IJG encoder is not
 * currently capable of doing this, but other encoders might.)  Since we want
 * to be able to dequantize all the components at the end of the file, this
 * means that we have to save away the table actually used for each component.
 * We do this by copying the table at the start of the first scan containing
 * the component.
 * The JPEG spec prohibits the encoder from changing the contents of a Q-table
 * slot between scans of a component using that slot.  If the encoder does so
 * anyway, this decoder will simply use the Q-table values that were current
 * at the start of the first scan for the component.
 *
 * The decompressor output side looks only at the saved quant tables,
 * not at the current Q-table slots.
 */

J_WARN_UNUSED_RESULT LOCAL(void_result_t)
latch_quant_tables (j_decompress_ptr cinfo)
{
  int ci, qtblno;
  jpeg_component_info *compptr;
  JQUANT_TBL * qtbl;

  for (ci = 0; ci < cinfo->comps_in_scan; ci++) {
    compptr = cinfo->cur_comp_info[ci];
    /* No work if we already saved Q-table for this component */
    if (compptr->quant_table != NULL)
      continue;
    /* Make sure specified quantization table is present */
    qtblno = compptr->quant_tbl_no;
    if (qtblno < 0 || qtblno >= NUM_QUANT_TBLS ||
	cinfo->quant_tbl_ptrs[qtblno] == NULL)
      ERREXIT1(cinfo, JERR_NO_QUANT_TABLE, qtblno, ERR_VOID);
    /* OK, save away the quantization table */
    void_ptr_result_t alloc_small_result =
      (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
				  SIZEOF(JQUANT_TBL));
    if (alloc_small_result.is_err)
      return ERR_VOID(alloc_small_result.err_code);
    qtbl = (JQUANT_TBL *) alloc_small_result.value;
    MEMCOPY(qtbl, cinfo->quant_tbl_ptrs[qtblno], SIZEOF(JQUANT_TBL));
    compptr->quant_table = qtbl;
  }

  return OK_VOID;
}


/*
 * Initialize for an input processing pass.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
start_input_pass (j_decompress_ptr cinfo)
{
  j_lossy_d_ptr lossyd = (j_lossy_d_ptr) cinfo->codec;

  void_result_t latch_quant_tables_result = latch_quant_tables(cinfo);
  if (latch_quant_tables_result.is_err)
    return latch_quant_tables_result;

  void_result_t entropy_start_pass_result = ((*lossyd->entropy_start_pass) (cinfo));
  if (entropy_start_pass_result.is_err)
    return entropy_start_pass_result;

  (*lossyd->coef_start_input_pass) (cinfo);

  return OK_VOID;
}


/*
 * Initialize for an output processing pass.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
start_output_pass (j_decompress_ptr cinfo)
{
  j_lossy_d_ptr lossyd = (j_lossy_d_ptr) cinfo->codec;

  void_result_t idct_start_pass_result = ((*lossyd->idct_start_pass) (cinfo));
  if (idct_start_pass_result.is_err)
    return idct_start_pass_result;
  void_result_t coef_start_output_pass_result = ((*lossyd->coef_start_output_pass) (cinfo));
  if (coef_start_output_pass_result.is_err)
    return coef_start_output_pass_result;

  return OK_VOID;
}

/*
 * Initialize the lossy decompression codec.
 * This is called only once, during master selection.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_lossy_d_codec (j_decompress_ptr cinfo)
{
  j_lossy_d_ptr lossyd;
  boolean use_c_buffer;

  /* Create subobject in permanent pool */
  void_ptr_result_t alloc_small_result =
    (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_PERMANENT,
				SIZEOF(jpeg_lossy_d_codec));
  if (alloc_small_result.is_err)
    return ERR_VOID(alloc_small_result.err_code);
  lossyd = (j_lossy_d_ptr) alloc_small_result.value;
  cinfo->codec = (struct jpeg_d_codec *) lossyd;

  /* Initialize sub-modules */

  /* Inverse DCT */
  void_result_t jinit_inverse_dct_result = jinit_inverse_dct(cinfo);
  if (jinit_inverse_dct_result.is_err)
    return jinit_inverse_dct_result;
  /* Entropy decoding: either Huffman or arithmetic coding. */
  if (cinfo->arith_code) {
#ifdef WITH_ARITHMETIC_PATCH
    jinit_arith_decoder(cinfo);
#else
    ERREXIT(cinfo, JERR_ARITH_NOTIMPL, ERR_VOID);
#endif
  } else {
    if (cinfo->process == JPROC_PROGRESSIVE) {
#ifdef D_PROGRESSIVE_SUPPORTED
      void_result_t jinit_phuff_decoder_result = jinit_phuff_decoder(cinfo);
      if (jinit_phuff_decoder_result.is_err)
        return jinit_phuff_decoder_result;
#else
      ERREXIT(cinfo, JERR_NOT_COMPILED, ERR_VOID);
#endif
    } else {
      void_result_t jinit_shuff_decoder_result = jinit_shuff_decoder(cinfo);
      if (jinit_shuff_decoder_result.is_err)
        return jinit_shuff_decoder_result;
    }
  }

  use_c_buffer = cinfo->inputctl->has_multiple_scans || cinfo->buffered_image;
  void_result_t jinit_d_coef_controller_result = jinit_d_coef_controller(cinfo, use_c_buffer);
  if (jinit_d_coef_controller_result.is_err)
    return jinit_d_coef_controller_result;

  /* Initialize method pointers.
   *
   * Note: consume_data and decompress_data are assigned in jdcoefct.c.
   */
  lossyd->pub.calc_output_dimensions = calc_output_dimensions;
  lossyd->pub.start_input_pass = start_input_pass;
  lossyd->pub.start_output_pass = start_output_pass;

  return OK_VOID;
}




