/*
 * jccoefct.c
 *
 * Copyright (C) 1994-1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains the coefficient buffer controller for compression.
 * This controller is the top level of the JPEG compressor proper.
 * The coefficient buffer lies between forward-DCT and entropy encoding steps.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"
#include "jlossy12.h"       /* Private declarations for lossy codec */


/* We use a full-image coefficient buffer when doing Huffman optimization,
 * and also for writing multiple-scan JPEG files.  In all cases, the DCT
 * step is run during the first pass, and subsequent passes need only read
 * the buffered coefficients.
 */
#ifdef ENTROPY_OPT_SUPPORTED
#define FULL_COEF_BUFFER_SUPPORTED
#else
#ifdef C_MULTISCAN_FILES_SUPPORTED
#define FULL_COEF_BUFFER_SUPPORTED
#endif
#endif


/* Private buffer controller object */

typedef struct {
  JDIMENSION iMCU_row_num;  /* iMCU row # within image */
  JDIMENSION mcu_ctr;       /* counts MCUs processed in current row */
  int MCU_vert_offset;      /* counts MCU rows within iMCU row */
  int MCU_rows_per_iMCU_row;    /* number of such rows needed */

  /* For single-pass compression, it's sufficient to buffer just one MCU
   * (although this may prove a bit slow in practice).  We allocate a
   * workspace of C_MAX_DATA_UNITS_IN_MCU coefficient blocks, and reuse it for
   * each MCU constructed and sent.  (On 80x86, the workspace is FAR even
   * though it's not really very big; this is to keep the module interfaces
   * unchanged when a large coefficient buffer is necessary.)
   * In multi-pass modes, this array points to the current MCU's blocks
   * within the virtual arrays.
   */
  JBLOCKROW MCU_buffer[C_MAX_DATA_UNITS_IN_MCU];

  /* In multi-pass modes, we need a virtual block array for each component. */
  jvirt_barray_ptr whole_image[MAX_COMPONENTS];
} c_coef_controller;

typedef c_coef_controller * c_coef_ptr;


/* Forward declarations */
J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t) compress_data
    JPP((j_compress_ptr cinfo, JSAMPIMAGE input_buf));
#ifdef FULL_COEF_BUFFER_SUPPORTED
J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t) compress_first_pass
    JPP((j_compress_ptr cinfo, JSAMPIMAGE input_buf));
J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t) compress_output
    JPP((j_compress_ptr cinfo, JSAMPIMAGE input_buf));
#endif


LOCAL(void)
start_iMCU_row (j_compress_ptr cinfo)
/* Reset within-iMCU-row counters for a new row */
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  c_coef_ptr coef = (c_coef_ptr) lossyc->coef_private;

  /* In an interleaved scan, an MCU row is the same as an iMCU row.
   * In a noninterleaved scan, an iMCU row has v_samp_factor MCU rows.
   * But at the bottom of the image, process only what's left.
   */
  if (cinfo->comps_in_scan > 1) {
    coef->MCU_rows_per_iMCU_row = 1;
  } else {
    if (coef->iMCU_row_num < (cinfo->total_iMCU_rows-1))
      coef->MCU_rows_per_iMCU_row = cinfo->cur_comp_info[0]->v_samp_factor;
    else
      coef->MCU_rows_per_iMCU_row = cinfo->cur_comp_info[0]->last_row_height;
  }

  coef->mcu_ctr = 0;
  coef->MCU_vert_offset = 0;
}


/*
 * Initialize for a processing pass.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
start_pass_coef (j_compress_ptr cinfo, J_BUF_MODE pass_mode)
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  c_coef_ptr coef = (c_coef_ptr) lossyc->coef_private;

  coef->iMCU_row_num = 0;
  start_iMCU_row(cinfo);

  switch (pass_mode) {
  case JBUF_PASS_THRU:
    if (coef->whole_image[0] != NULL)
      ERREXIT(cinfo, JERR_BAD_BUFFER_MODE, ERR_VOID);
    lossyc->pub.compress_data = compress_data;
    break;
#ifdef FULL_COEF_BUFFER_SUPPORTED
  case JBUF_SAVE_AND_PASS:
    if (coef->whole_image[0] == NULL)
      ERREXIT(cinfo, JERR_BAD_BUFFER_MODE, ERR_VOID);
    lossyc->pub.compress_data = compress_first_pass;
    break;
  case JBUF_CRANK_DEST:
    if (coef->whole_image[0] == NULL)
      ERREXIT(cinfo, JERR_BAD_BUFFER_MODE, ERR_VOID);
    lossyc->pub.compress_data = compress_output;
    break;
#endif
  default:
    ERREXIT(cinfo, JERR_BAD_BUFFER_MODE, ERR_VOID);
    break;
  }

  return OK_VOID;
}


/*
 * Process some data in the single-pass case.
 * We process the equivalent of one fully interleaved MCU row ("iMCU" row)
 * per call, ie, v_samp_factor block rows for each component in the image.
 * Returns TRUE if the iMCU row is completed, FALSE if suspended.
 *
 * NB: input_buf contains a plane for each component in image,
 * which we index according to the component's SOF position.
 */

J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t)
compress_data (j_compress_ptr cinfo, JSAMPIMAGE input_buf)
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  c_coef_ptr coef = (c_coef_ptr) lossyc->coef_private;
  JDIMENSION MCU_col_num;   /* index of current MCU within row */
  JDIMENSION last_MCU_col = cinfo->MCUs_per_row - 1;
  JDIMENSION last_iMCU_row = cinfo->total_iMCU_rows - 1;
  int blkn, bi, ci, yindex, yoffset, blockcnt;
  JDIMENSION ypos, xpos;
  jpeg_component_info *compptr;

  /* Loop to write as much as one whole iMCU row */
  for (yoffset = coef->MCU_vert_offset; yoffset < coef->MCU_rows_per_iMCU_row;
       yoffset++) {
    for (MCU_col_num = coef->mcu_ctr; MCU_col_num <= last_MCU_col;
     MCU_col_num++) {
      /* Determine where data comes from in input_buf and do the DCT thing.
       * Each call on forward_DCT processes a horizontal row of DCT blocks
       * as wide as an MCU; we rely on having allocated the MCU_buffer[] blocks
       * sequentially.  Dummy blocks at the right or bottom edge are filled in
       * specially.  The data in them does not matter for image reconstruction,
       * so we fill them with values that will encode to the smallest amount of
       * data, viz: all zeroes in the AC entries, DC entries equal to previous
       * block's DC value.  (Thanks to Thomas Kinsman for this idea.)
       */
      blkn = 0;
      for (ci = 0; ci < cinfo->comps_in_scan; ci++) {
    compptr = cinfo->cur_comp_info[ci];
    blockcnt = (MCU_col_num < last_MCU_col) ? compptr->MCU_width
                        : compptr->last_col_width;
    xpos = MCU_col_num * (JDIMENSION)compptr->MCU_sample_width;
    ypos = (JDIMENSION)(yoffset * DCTSIZE); /* ypos == (yoffset+yindex) * DCTSIZE */
    for (yindex = 0; yindex < compptr->MCU_height; yindex++) {
      if (coef->iMCU_row_num < last_iMCU_row ||
          yoffset+yindex < compptr->last_row_height) {
        (*lossyc->fdct_forward_DCT) (cinfo, compptr,
                    input_buf[compptr->component_index],
                    coef->MCU_buffer[blkn],
                    ypos, xpos, (JDIMENSION) blockcnt);
        if (blockcnt < compptr->MCU_width) {
          /* Create some dummy blocks at the right edge of the image. */
          jzero_far((void FAR *) coef->MCU_buffer[blkn + blockcnt],
            (size_t)(compptr->MCU_width - blockcnt) * SIZEOF(JBLOCK));
          for (bi = blockcnt; bi < compptr->MCU_width; bi++) {
        coef->MCU_buffer[blkn+bi][0][0] = coef->MCU_buffer[blkn+bi-1][0][0];
          }
        }
      } else {
        /* Create a row of dummy blocks at the bottom of the image. */
        jzero_far((void FAR *) coef->MCU_buffer[blkn],
              (size_t)compptr->MCU_width * SIZEOF(JBLOCK));
        for (bi = 0; bi < compptr->MCU_width; bi++) {
          coef->MCU_buffer[blkn+bi][0][0] = coef->MCU_buffer[blkn-1][0][0];
        }
      }
      blkn += compptr->MCU_width;
      ypos += DCTSIZE;
    }
      }
      /* Try to write the MCU.  In event of a suspension failure, we will
       * re-DCT the MCU on restart (a bit inefficient, could be fixed...)
       */
      boolean_result_t entropy_encode_mcu_result = (*lossyc->entropy_encode_mcu) (cinfo, coef->MCU_buffer);
      if (entropy_encode_mcu_result.is_err) {
        return entropy_encode_mcu_result;
      }
      if (! entropy_encode_mcu_result.value) {
    /* Suspension forced; update state counters and exit */
    coef->MCU_vert_offset = yoffset;
    coef->mcu_ctr = MCU_col_num;
    return RESULT_OK(boolean, FALSE);
      }
    }
    /* Completed an MCU row, but perhaps not an iMCU row */
    coef->mcu_ctr = 0;
  }
  /* Completed the iMCU row, advance counters for next one */
  coef->iMCU_row_num++;
  start_iMCU_row(cinfo);
  return RESULT_OK(boolean, TRUE);
}


#ifdef FULL_COEF_BUFFER_SUPPORTED

/*
 * Process some data in the first pass of a multi-pass case.
 * We process the equivalent of one fully interleaved MCU row ("iMCU" row)
 * per call, ie, v_samp_factor block rows for each component in the image.
 * This amount of data is read from the source buffer, DCT'd and quantized,
 * and saved into the virtual arrays.  We also generate suitable dummy blocks
 * as needed at the right and lower edges.  (The dummy blocks are constructed
 * in the virtual arrays, which have been padded appropriately.)  This makes
 * it possible for subsequent passes not to worry about real vs. dummy blocks.
 *
 * We must also emit the data to the entropy encoder.  This is conveniently
 * done by calling compress_output() after we've loaded the current strip
 * of the virtual arrays.
 *
 * NB: input_buf contains a plane for each component in image.  All
 * components are DCT'd and loaded into the virtual arrays in this pass.
 * However, it may be that only a subset of the components are emitted to
 * the entropy encoder during this first pass; be careful about looking
 * at the scan-dependent variables (MCU dimensions, etc).
 */

J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t)
compress_first_pass (j_compress_ptr cinfo, JSAMPIMAGE input_buf)
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  c_coef_ptr coef = (c_coef_ptr) lossyc->coef_private;
  JDIMENSION last_iMCU_row = cinfo->total_iMCU_rows - 1;
  JDIMENSION blocks_across, MCUs_across, MCUindex;
  int bi, ci, h_samp_factor, block_row, block_rows, ndummy;
  JCOEF lastDC;
  jpeg_component_info *compptr;
  JBLOCKARRAY buffer;
  JBLOCKROW thisblockrow, lastblockrow;

  for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    /* Align the virtual buffer for this component. */
    jblockarray_result_t access_virt_barray_result = (*cinfo->mem->access_virt_barray)
      ((j_common_ptr) cinfo, coef->whole_image[ci],
       coef->iMCU_row_num * (JDIMENSION)compptr->v_samp_factor,
       (JDIMENSION)compptr->v_samp_factor, TRUE);
    if (access_virt_barray_result.is_err) {
      return RESULT_ERR(boolean, access_virt_barray_result.err_code);
    }
    buffer = access_virt_barray_result.value;
    /* Count non-dummy DCT block rows in this iMCU row. */
    if (coef->iMCU_row_num < last_iMCU_row)
      block_rows = compptr->v_samp_factor;
    else {
      /* NB: can't use last_row_height here, since may not be set! */
      block_rows = (int)compptr->height_in_data_units % compptr->v_samp_factor;
      if (block_rows == 0) block_rows = compptr->v_samp_factor;
    }
    blocks_across = (JDIMENSION)compptr->width_in_data_units;
    h_samp_factor = compptr->h_samp_factor;
    /* Count number of dummy blocks to be added at the right margin. */
    ndummy = (int)blocks_across % h_samp_factor;
    if (ndummy > 0)
      ndummy = h_samp_factor - ndummy;
    /* Perform DCT for all non-dummy blocks in this iMCU row.  Each call
     * on forward_DCT processes a complete horizontal row of DCT blocks.
     */
    for (block_row = 0; block_row < block_rows; block_row++) {
      thisblockrow = buffer[block_row];
      (*lossyc->fdct_forward_DCT) (cinfo, compptr,
                   input_buf[ci], thisblockrow,
                   (JDIMENSION) (block_row * DCTSIZE),
                   (JDIMENSION) 0, blocks_across);
      if (ndummy > 0) {
    /* Create dummy blocks at the right edge of the image. */
    thisblockrow += blocks_across; /* => first dummy block */
    jzero_far((void FAR *) thisblockrow, (size_t)ndummy * SIZEOF(JBLOCK));
    lastDC = thisblockrow[-1][0];
    for (bi = 0; bi < ndummy; bi++) {
      thisblockrow[bi][0] = lastDC;
    }
      }
    }
    /* If at end of image, create dummy block rows as needed.
     * The tricky part here is that within each MCU, we want the DC values
     * of the dummy blocks to match the last real block's DC value.
     * This squeezes a few more bytes out of the resulting file...
     */
    if (coef->iMCU_row_num == last_iMCU_row) {
      blocks_across += (JDIMENSION)ndummy;  /* include lower right corner */
      MCUs_across = blocks_across / (JDIMENSION)h_samp_factor;
      for (block_row = block_rows; block_row < compptr->v_samp_factor;
       block_row++) {
    thisblockrow = buffer[block_row];
    lastblockrow = buffer[block_row-1];
    jzero_far((void FAR *) thisblockrow,
          (size_t) (blocks_across * SIZEOF(JBLOCK)));
    for (MCUindex = 0; MCUindex < MCUs_across; MCUindex++) {
      lastDC = lastblockrow[h_samp_factor-1][0];
      for (bi = 0; bi < h_samp_factor; bi++) {
        thisblockrow[bi][0] = lastDC;
      }
      thisblockrow += h_samp_factor; /* advance to next MCU in row */
      lastblockrow += h_samp_factor;
    }
      }
    }
  }
  /* NB: compress_output will increment iMCU_row_num if successful.
   * A suspension return will result in redoing all the work above next time.
   */

  /* Emit data to the entropy encoder, sharing code with subsequent passes */
  return compress_output(cinfo, input_buf);
}


/*
 * Process some data in subsequent passes of a multi-pass case.
 * We process the equivalent of one fully interleaved MCU row ("iMCU" row)
 * per call, ie, v_samp_factor block rows for each component in the scan.
 * The data is obtained from the virtual arrays and fed to the entropy coder.
 * Returns TRUE if the iMCU row is completed, FALSE if suspended.
 *
 * NB: input_buf is ignored; it is likely to be a NULL pointer.
 */

J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t)
compress_output (j_compress_ptr cinfo, JSAMPIMAGE input_buf)
{
  (void)input_buf;
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  c_coef_ptr coef = (c_coef_ptr) lossyc->coef_private;
  JDIMENSION MCU_col_num;   /* index of current MCU within row */
  int blkn, ci, xindex, yindex, yoffset;
  JDIMENSION start_col;
  JBLOCKARRAY buffer[MAX_COMPS_IN_SCAN];
  JBLOCKROW buffer_ptr;
  jpeg_component_info *compptr;

  /* Align the virtual buffers for the components used in this scan.
   * NB: during first pass, this is safe only because the buffers will
   * already be aligned properly, so jmemmgr.c won't need to do any I/O.
   */
  for (ci = 0; ci < cinfo->comps_in_scan; ci++) {
    compptr = cinfo->cur_comp_info[ci];
    jblockarray_result_t access_virt_barray_result = (*cinfo->mem->access_virt_barray)
      ((j_common_ptr) cinfo, coef->whole_image[compptr->component_index],
       coef->iMCU_row_num * (JDIMENSION)compptr->v_samp_factor,
       (JDIMENSION)compptr->v_samp_factor, FALSE);
    if (access_virt_barray_result.is_err) {
      return RESULT_ERR(boolean, access_virt_barray_result.err_code);
    }
    buffer[ci] = access_virt_barray_result.value;
  }

  /* Loop to process one whole iMCU row */
  for (yoffset = coef->MCU_vert_offset; yoffset < coef->MCU_rows_per_iMCU_row;
       yoffset++) {
    for (MCU_col_num = coef->mcu_ctr; MCU_col_num < cinfo->MCUs_per_row;
     MCU_col_num++) {
      /* Construct list of pointers to DCT blocks belonging to this MCU */
      blkn = 0;         /* index of current DCT block within MCU */
      for (ci = 0; ci < cinfo->comps_in_scan; ci++) {
    compptr = cinfo->cur_comp_info[ci];
    start_col = MCU_col_num * (JDIMENSION)compptr->MCU_width;
    for (yindex = 0; yindex < compptr->MCU_height; yindex++) {
      buffer_ptr = buffer[ci][yindex+yoffset] + start_col;
      for (xindex = 0; xindex < compptr->MCU_width; xindex++) {
        coef->MCU_buffer[blkn++] = buffer_ptr++;
      }
    }
      }
      /* Try to write the MCU. */
      boolean_result_t entropy_encode_mcu_result = (*lossyc->entropy_encode_mcu) (cinfo, coef->MCU_buffer);
      if (entropy_encode_mcu_result.is_err) {
        return entropy_encode_mcu_result;
      }
      if (! entropy_encode_mcu_result.value) {
    /* Suspension forced; update state counters and exit */
    coef->MCU_vert_offset = yoffset;
    coef->mcu_ctr = MCU_col_num;
    return RESULT_OK(boolean, FALSE);
      }
    }
    /* Completed an MCU row, but perhaps not an iMCU row */
    coef->mcu_ctr = 0;
  }
  /* Completed the iMCU row, advance counters for next one */
  coef->iMCU_row_num++;
  start_iMCU_row(cinfo);
  return RESULT_OK(boolean, TRUE);
}

#endif /* FULL_COEF_BUFFER_SUPPORTED */


/*
 * Initialize coefficient buffer controller.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_c_coef_controller (j_compress_ptr cinfo, boolean need_full_buffer)
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  c_coef_ptr coef;

  void_ptr_result_t alloc_small_result =
    (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
                SIZEOF(c_coef_controller));
  coef = (c_coef_ptr) alloc_small_result.value;
  lossyc->coef_private = (void *) coef;
  lossyc->coef_start_pass = start_pass_coef;

  /* Create the coefficient buffer. */
  if (need_full_buffer) {
#ifdef FULL_COEF_BUFFER_SUPPORTED
    /* Allocate a full-image virtual array for each component, */
    /* padded to a multiple of samp_factor DCT blocks in each direction. */
    int ci;
    jpeg_component_info *compptr;

    for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
     ci++, compptr++) { 
      jvirt_barray_result_t request_virt_barray_result = (*cinfo->mem->request_virt_barray)
    ((j_common_ptr) cinfo, JPOOL_IMAGE, FALSE,
     (JDIMENSION) jround_up((long) compptr->width_in_data_units,
                (long) compptr->h_samp_factor),
     (JDIMENSION) jround_up((long) compptr->height_in_data_units,
                (long) compptr->v_samp_factor),
     (JDIMENSION) compptr->v_samp_factor);
      if (request_virt_barray_result.is_err) {
        return ERR_VOID(request_virt_barray_result.err_code);
      }
      coef->whole_image[ci] = request_virt_barray_result.value;
    }
#else
    ERREXIT(cinfo, JERR_BAD_BUFFER_MODE);
#endif
  } else {
    /* We only need a single-MCU buffer. */
    JBLOCKROW buffer;
    int i;

    void_far_ptr_result_t alloc_large_result =
      (*cinfo->mem->alloc_large) ((j_common_ptr) cinfo, JPOOL_IMAGE,
                  C_MAX_DATA_UNITS_IN_MCU * SIZEOF(JBLOCK));
    if (alloc_large_result.is_err) {
      return ERR_VOID(alloc_large_result.err_code);
    }
    buffer = (JBLOCKROW) alloc_large_result.value;
    for (i = 0; i < C_MAX_DATA_UNITS_IN_MCU; i++) {
      coef->MCU_buffer[i] = buffer + i;
    }
    coef->whole_image[0] = NULL; /* flag for no virtual arrays */
  }

  return OK_VOID;
}
