/*
 * jddiffct.c
 *
 * Copyright (C) 1994-1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains the [un]difference buffer controller for decompression.
 * This controller is the top level of the lossless JPEG decompressor proper.
 * The difference buffer lies between the entropy decoding and
 * prediction/undifferencing steps.  The undifference buffer lies between the
 * prediction/undifferencing and scaling steps.
 *
 * In buffered-image mode, this controller is the interface between
 * input-oriented processing and output-oriented processing.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"
#include "jlossls12.h"


#ifdef D_LOSSLESS_SUPPORTED

/* Private buffer controller object */

typedef struct {
  /* These variables keep track of the current location of the input side. */
  /* cinfo->input_iMCU_row is also used for this. */
  JDIMENSION MCU_ctr;       /* counts MCUs processed in current row */
  unsigned int restart_rows_to_go;  /* MCU-rows left in this restart interval */
  unsigned int MCU_vert_offset;     /* counts MCU rows within iMCU row */
  unsigned int MCU_rows_per_iMCU_row;   /* number of such rows needed */

  /* The output side's location is represented by cinfo->output_iMCU_row. */

  JDIFFARRAY diff_buf[MAX_COMPONENTS];  /* iMCU row of differences */
  JDIFFARRAY undiff_buf[MAX_COMPONENTS]; /* iMCU row of undiff'd samples */

#ifdef D_MULTISCAN_FILES_SUPPORTED
  /* In multi-pass modes, we need a virtual sample array for each component. */
  jvirt_sarray_ptr whole_image[MAX_COMPONENTS];
#endif
} d_diff_controller;

typedef d_diff_controller * d_diff_ptr;

/* Forward declarations */
J_WARN_UNUSED_RESULT METHODDEF(int_result_t) decompress_data
    JPP((j_decompress_ptr cinfo, JSAMPIMAGE output_buf));
#ifdef D_MULTISCAN_FILES_SUPPORTED
J_WARN_UNUSED_RESULT METHODDEF(int_result_t) output_data
    JPP((j_decompress_ptr cinfo, JSAMPIMAGE output_buf));
#endif


LOCAL(void)
start_iMCU_row (j_decompress_ptr cinfo)
/* Reset within-iMCU-row counters for a new row (input side) */
{
  j_lossless_d_ptr losslsd = (j_lossless_d_ptr) cinfo->codec;
  d_diff_ptr diff = (d_diff_ptr) losslsd->diff_private;

  /* In an interleaved scan, an MCU row is the same as an iMCU row.
   * In a noninterleaved scan, an iMCU row has v_samp_factor MCU rows.
   * But at the bottom of the image, process only what's left.
   */
  if (cinfo->comps_in_scan > 1) {
    diff->MCU_rows_per_iMCU_row = 1;
  } else {
    if (cinfo->input_iMCU_row < (cinfo->total_iMCU_rows-1))
      diff->MCU_rows_per_iMCU_row = (JDIMENSION)cinfo->cur_comp_info[0]->v_samp_factor;
    else
      diff->MCU_rows_per_iMCU_row = (JDIMENSION)cinfo->cur_comp_info[0]->last_row_height;
  }

  diff->MCU_ctr = 0;
  diff->MCU_vert_offset = 0;
}


/*
 * Initialize for an input processing pass.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
start_input_pass (j_decompress_ptr cinfo)
{
  j_lossless_d_ptr losslsd = (j_lossless_d_ptr) cinfo->codec;
  d_diff_ptr diff = (d_diff_ptr) losslsd->diff_private;

  /* Check that the restart interval is an integer multiple of the number 
   * of MCU in an MCU-row.
   */
  if (cinfo->restart_interval % cinfo->MCUs_per_row != 0)
    ERREXIT2(cinfo, JERR_BAD_RESTART,
         (int)cinfo->restart_interval, (int)cinfo->MCUs_per_row, ERR_VOID);

  /* Initialize restart counter */
  diff->restart_rows_to_go = cinfo->restart_interval / cinfo->MCUs_per_row;

  cinfo->input_iMCU_row = 0;
  start_iMCU_row(cinfo);

  return OK_VOID;
}


/*
 * Check for a restart marker & resynchronize decoder, undifferencer.
 * Returns FALSE if must suspend.
 */

J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t)
process_restart (j_decompress_ptr cinfo)
{
  j_lossless_d_ptr losslsd = (j_lossless_d_ptr) cinfo->codec;
  d_diff_ptr diff = (d_diff_ptr) losslsd->diff_private;

  boolean_result_t entropy_process_restart_result = (*losslsd->entropy_process_restart) (cinfo);
  if (entropy_process_restart_result.is_err)
    return entropy_process_restart_result;
  if (!entropy_process_restart_result.value)
    return RESULT_OK(boolean, FALSE);

  void_result_t predict_process_restart_result = ((*losslsd->predict_process_restart) (cinfo));
  if (predict_process_restart_result.is_err)
    return RESULT_ERR(boolean, predict_process_restart_result.err_code);

  /* Reset restart counter */
  diff->restart_rows_to_go = cinfo->restart_interval / cinfo->MCUs_per_row;

  return RESULT_OK(boolean, TRUE);
}


/*
 * Initialize for an output processing pass.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
start_output_pass (j_decompress_ptr cinfo)
{
  cinfo->output_iMCU_row = 0;

  return OK_VOID;
}


/*
 * Decompress and return some data in the supplied buffer.
 * Always attempts to emit one fully interleaved MCU row ("iMCU" row).
 * Input and output must run in lockstep since we have only a one-MCU buffer.
 * Return value is JPEG_ROW_COMPLETED, JPEG_SCAN_COMPLETED, or JPEG_SUSPENDED.
 *
 * NB: output_buf contains a plane for each component in image,
 * which we index according to the component's SOF position.
 */

J_WARN_UNUSED_RESULT METHODDEF(int_result_t)
decompress_data (j_decompress_ptr cinfo, JSAMPIMAGE output_buf)
{
  j_lossless_d_ptr losslsd = (j_lossless_d_ptr) cinfo->codec;
  d_diff_ptr diff = (d_diff_ptr) losslsd->diff_private;
  JDIMENSION MCU_col_num;   /* index of current MCU within row */
  JDIMENSION MCU_count;     /* number of MCUs decoded */
  JDIMENSION last_iMCU_row = cinfo->total_iMCU_rows - 1;
  int comp, ci, row, prev_row;
  unsigned int yoffset;
  jpeg_component_info *compptr;

  /* Loop to process as much as one whole iMCU row */
  for (yoffset = diff->MCU_vert_offset; yoffset < diff->MCU_rows_per_iMCU_row;
       yoffset++) {

    /* Process restart marker if needed; may have to suspend */
    if (cinfo->restart_interval) {
      if (diff->restart_rows_to_go == 0) {
        boolean_result_t process_restart_result = process_restart(cinfo);
        if (process_restart_result.is_err)
          return RESULT_ERR(int, process_restart_result.err_code);
        if (!process_restart_result.value)
          return RESULT_OK(int, JPEG_SUSPENDED);
      }
    }

    MCU_col_num = diff->MCU_ctr;
    /* Try to fetch an MCU-row (or remaining portion of suspended MCU-row). */
    jdimension_result_t entropy_decode_mcus_result =
      (*losslsd->entropy_decode_mcus) (cinfo,
                       diff->diff_buf, yoffset, MCU_col_num,
                       cinfo->MCUs_per_row - MCU_col_num);
    if (entropy_decode_mcus_result.is_err)
      return RESULT_ERR(int, entropy_decode_mcus_result.err_code);
    MCU_count = entropy_decode_mcus_result.value;
    if (MCU_count != cinfo->MCUs_per_row - MCU_col_num) {
      /* Suspension forced; update state counters and exit */
      diff->MCU_vert_offset = yoffset;
      diff->MCU_ctr += MCU_count;
      return RESULT_OK(int, JPEG_SUSPENDED);
    }

    /* Account for restart interval (no-op if not using restarts) */
    diff->restart_rows_to_go--;

    /* Completed an MCU row, but perhaps not an iMCU row */
    diff->MCU_ctr = 0;
  }

  /*
   * Undifference and scale each scanline of the disassembled MCU-row
   * separately.  We do not process dummy samples at the end of a scanline
   * or dummy rows at the end of the image.
   */
  for (comp = 0; comp < cinfo->comps_in_scan; comp++) {
    compptr = cinfo->cur_comp_info[comp];
    ci = compptr->component_index;
    for (row = 0, prev_row = compptr->v_samp_factor - 1;
     row < (cinfo->input_iMCU_row == last_iMCU_row ?
        compptr->last_row_height : compptr->v_samp_factor);
     prev_row = row, row++) {
      (*losslsd->predict_undifference[ci]) (cinfo, ci,
                        diff->diff_buf[ci][row],
                        diff->undiff_buf[ci][prev_row],
                        diff->undiff_buf[ci][row],
                        compptr->width_in_data_units);
      (*losslsd->scaler_scale) (cinfo, diff->undiff_buf[ci][row],
                output_buf[ci][row],
                compptr->width_in_data_units);
    }
  }

  /* Completed the iMCU row, advance counters for next one.
   *
   * NB: output_data will increment output_iMCU_row.
   * This counter is not needed for the single-pass case
   * or the input side of the multi-pass case.
   */
  if (++(cinfo->input_iMCU_row) < cinfo->total_iMCU_rows) {
    start_iMCU_row(cinfo);
    return RESULT_OK(int, JPEG_ROW_COMPLETED);
  }
  /* Completed the scan */
  (*cinfo->inputctl->finish_input_pass) (cinfo);
  return RESULT_OK(int, JPEG_SCAN_COMPLETED);
}


/*
 * Dummy consume-input routine for single-pass operation.
 */

J_WARN_UNUSED_RESULT METHODDEF(int_result_t)
dummy_consume_data (j_decompress_ptr cinfo)
{
  (void)cinfo;
  return RESULT_OK(int, JPEG_SUSPENDED);    /* Always indicate nothing was done */
}


#ifdef D_MULTISCAN_FILES_SUPPORTED

/*
 * Consume input data and store it in the full-image sample buffer.
 * We read as much as one fully interleaved MCU row ("iMCU" row) per call,
 * ie, v_samp_factor rows for each component in the scan.
 * Return value is JPEG_ROW_COMPLETED, JPEG_SCAN_COMPLETED, or JPEG_SUSPENDED.
 */

J_WARN_UNUSED_RESULT METHODDEF(int_result_t)
consume_data (j_decompress_ptr cinfo)
{
  j_lossless_d_ptr losslsd = (j_lossless_d_ptr) cinfo->codec;
  d_diff_ptr diff = (d_diff_ptr) losslsd->diff_private;
  /* JDIMENSION MCU_col_num; */ /* index of current MCU within row */
  /* JDIMENSION MCU_count; */   /* number of MCUs decoded */
  /* JDIMENSION last_iMCU_row = cinfo->total_iMCU_rows - 1; */
  int comp, ci /* , yoffset, row, prev_row */;
  JSAMPARRAY buffer[MAX_COMPS_IN_SCAN];
  jpeg_component_info *compptr;

  /* Align the virtual buffers for the components used in this scan. */
  for (comp = 0; comp < cinfo->comps_in_scan; comp++) {
    compptr = cinfo->cur_comp_info[comp];
    ci = compptr->component_index;

    jsamparray_result_t access_virt_sarray_result =
      (*cinfo->mem->access_virt_sarray)
        ((j_common_ptr) cinfo, diff->whole_image[ci],
         cinfo->input_iMCU_row * (JDIMENSION)compptr->v_samp_factor,
         (JDIMENSION) compptr->v_samp_factor, TRUE);
    if (access_virt_sarray_result.is_err)
      return RESULT_ERR(int, access_virt_sarray_result.err_code);

    buffer[ci] = access_virt_sarray_result.value;
  }

  return decompress_data(cinfo, buffer);
}


/*
 * Output some data from the full-image buffer sample in the multi-pass case.
 * Always attempts to emit one fully interleaved MCU row ("iMCU" row).
 * Return value is JPEG_ROW_COMPLETED, JPEG_SCAN_COMPLETED, or JPEG_SUSPENDED.
 *
 * NB: output_buf contains a plane for each component in image.
 */

J_WARN_UNUSED_RESULT METHODDEF(int_result_t)
output_data (j_decompress_ptr cinfo, JSAMPIMAGE output_buf)
{
  j_lossless_d_ptr losslsd = (j_lossless_d_ptr) cinfo->codec;
  d_diff_ptr diff = (d_diff_ptr) losslsd->diff_private;
  JDIMENSION last_iMCU_row = cinfo->total_iMCU_rows - 1;
  int ci, samp_rows, row;
  JSAMPARRAY buffer;
  jpeg_component_info *compptr;

  /* Force some input to be done if we are getting ahead of the input. */
  while (cinfo->input_scan_number < cinfo->output_scan_number ||
     (cinfo->input_scan_number == cinfo->output_scan_number &&
      cinfo->input_iMCU_row <= cinfo->output_iMCU_row)) {
    int_result_t consume_input_result = (*cinfo->inputctl->consume_input)(cinfo);
    if (consume_input_result.is_err)
      return consume_input_result;
    if (consume_input_result.value == JPEG_SUSPENDED)
      return RESULT_OK(int, JPEG_SUSPENDED);
  }

  /* OK, output from the virtual arrays. */
  for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    /* Align the virtual buffer for this component. */
    jsamparray_result_t access_virt_sarray_result =
      (*cinfo->mem->access_virt_sarray)
        ((j_common_ptr) cinfo, diff->whole_image[ci],
         cinfo->output_iMCU_row * (JDIMENSION)compptr->v_samp_factor,
         (JDIMENSION) compptr->v_samp_factor, FALSE);
    if (access_virt_sarray_result.is_err)
      return RESULT_ERR(int, access_virt_sarray_result.err_code);
    buffer = access_virt_sarray_result.value;
    if (cinfo->output_iMCU_row < last_iMCU_row)
      samp_rows = compptr->v_samp_factor;
    else {
      /* NB: can't use last_row_height here; it is input-side-dependent! */
      samp_rows = (int)compptr->height_in_data_units % compptr->v_samp_factor;
      if (samp_rows == 0) samp_rows = compptr->v_samp_factor;
    }

    for (row = 0; row < samp_rows; row++) {
      MEMCOPY(output_buf[ci][row], buffer[row],
          compptr->width_in_data_units * SIZEOF(JSAMPLE));
    }
  }

  if (++(cinfo->output_iMCU_row) < cinfo->total_iMCU_rows)
    return RESULT_OK(int, JPEG_ROW_COMPLETED);
  return RESULT_OK(int, JPEG_SCAN_COMPLETED);
}

#endif /* D_MULTISCAN_FILES_SUPPORTED */


/*
 * Initialize difference buffer controller.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_d_diff_controller (j_decompress_ptr cinfo, boolean need_full_buffer)
{
  j_lossless_d_ptr losslsd = (j_lossless_d_ptr) cinfo->codec;
  d_diff_ptr diff;
  int ci;
  jpeg_component_info *compptr;

  void_ptr_result_t alloc_small_result = 
    (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
                SIZEOF(d_diff_controller));
  if (alloc_small_result.is_err)
    return ERR_VOID(alloc_small_result.err_code);
  diff = (d_diff_ptr) alloc_small_result.value;
  losslsd->diff_private = (void *) diff;
  losslsd->diff_start_input_pass = start_input_pass;
  losslsd->pub.start_output_pass = start_output_pass;

  /* Create the [un]difference buffers. */
  for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    jdiffarray_result_t alloc_darray_result = 
      (*cinfo->mem->alloc_darray)
        ((j_common_ptr) cinfo, JPOOL_IMAGE,
         (JDIMENSION) jround_up((long) compptr->width_in_data_units,
                    (long) compptr->h_samp_factor),
         (JDIMENSION) compptr->v_samp_factor);
    if (alloc_darray_result.is_err)
      return ERR_VOID(alloc_darray_result.err_code);
    diff->diff_buf[ci] = alloc_darray_result.value;
    alloc_darray_result = (*cinfo->mem->alloc_darray)
      ((j_common_ptr) cinfo, JPOOL_IMAGE,
       (JDIMENSION) jround_up((long) compptr->width_in_data_units,
                  (long) compptr->h_samp_factor),
       (JDIMENSION) compptr->v_samp_factor);
    if (alloc_darray_result.is_err)
      return ERR_VOID(alloc_darray_result.err_code);
    diff->undiff_buf[ci] = alloc_darray_result.value;
  }

  if (need_full_buffer) {
#ifdef D_MULTISCAN_FILES_SUPPORTED
    /* Allocate a full-image virtual array for each component. */
    int access_rows;

    for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
     ci++, compptr++) {
      access_rows = compptr->v_samp_factor;
      jvirt_sarray_result_t request_virt_sarray_result =
       (*cinfo->mem->request_virt_sarray)
        ((j_common_ptr) cinfo, JPOOL_IMAGE, FALSE,
        (JDIMENSION) jround_up((long) compptr->width_in_data_units,
                    (long) compptr->h_samp_factor),
        (JDIMENSION) jround_up((long) compptr->height_in_data_units,
                    (long) compptr->v_samp_factor),
        (JDIMENSION) access_rows);
      if (request_virt_sarray_result.is_err)
        return ERR_VOID(request_virt_sarray_result.err_code);
      diff->whole_image[ci] = request_virt_sarray_result.value;
    }
    losslsd->pub.consume_data = consume_data;
    losslsd->pub.decompress_data = output_data;
#else
    ERREXIT(cinfo, JERR_NOT_COMPILED, ERR_VOID);
#endif
  } else {
    losslsd->pub.consume_data = dummy_consume_data;
    losslsd->pub.decompress_data = decompress_data;
    diff->whole_image[0] = NULL; /* flag for no virtual arrays */
  }

  return OK_VOID;
}

#endif /* D_LOSSLESS_SUPPORTED */
