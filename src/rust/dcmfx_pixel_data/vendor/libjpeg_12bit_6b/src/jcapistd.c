/*
 * jcapistd.c
 *
 * Copyright (C) 1994-1996, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains application interface code for the compression half
 * of the JPEG library.  These are the "standard" API routines that are
 * used in the normal full-compression case.  They are not used by a
 * transcoding-only application.  Note that if an application links in
 * jpeg_start_compress, it will end up linking in the entire compressor.
 * We thus must separate this file from jcapimin.c to avoid linking the
 * whole compression library into a transcoder.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"


/*
 * Compression initialization.
 * Before calling this, all parameters and a data destination must be set up.
 *
 * We require a write_all_tables parameter as a failsafe check when writing
 * multiple datastreams from the same compression object.  Since prior runs
 * will have left all the tables marked sent_table=TRUE, a subsequent run
 * would emit an abbreviated stream (no tables) by default.  This may be what
 * is wanted, but for safety's sake it should not be the default behavior:
 * programmers should have to make a deliberate choice to emit abbreviated
 * images.  Therefore the documentation and examples should encourage people
 * to pass write_all_tables=TRUE; then it will take active thought to do the
 * wrong thing.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_start_compress (j_compress_ptr cinfo, boolean write_all_tables)
{
  if (cinfo->global_state != CSTATE_START)
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_VOID);

  if (write_all_tables)
    jpeg_suppress_tables(cinfo, FALSE);	/* mark all tables to be written */

  /* (Re)initialize error mgr and destination modules */
  (*cinfo->err->reset_error_mgr) ((j_common_ptr) cinfo);
  void_result_t init_destination_result = (*cinfo->dest->init_destination) (cinfo);
  if (init_destination_result.is_err) {
    return init_destination_result;
  }
  /* Perform master selection of active modules */
  void_result_t jinit_compress_master_result = jinit_compress_master(cinfo);
  if (jinit_compress_master_result.is_err) {
    return jinit_compress_master_result;
  }
  /* Set up for the first pass */
  void_result_t prepare_for_pass_result = (*cinfo->master->prepare_for_pass) (cinfo);
  if (prepare_for_pass_result.is_err) {
    return prepare_for_pass_result;
  }
  /* Ready for application to drive first pass through jpeg_write_scanlines
   * or jpeg_write_raw_data.
   */
  cinfo->next_scanline = 0;
  cinfo->global_state = (cinfo->raw_data_in ? CSTATE_RAW_OK : CSTATE_SCANNING);

  return OK_VOID;
}


/*
 * Write some scanlines of data to the JPEG compressor.
 *
 * The return value will be the number of lines actually written.
 * This should be less than the supplied num_lines only in case that
 * the data destination module has requested suspension of the compressor,
 * or if more than image_height scanlines are passed in.
 *
 * Note: we warn about excess calls to jpeg_write_scanlines() since
 * this likely signals an application programmer error.  However,
 * excess scanlines passed in the last valid call are *silently* ignored,
 * so that the application need not adjust num_lines for end-of-image
 * when using a multiple-scanline buffer.
 */

J_WARN_UNUSED_RESULT GLOBAL(jdimension_result_t)
jpeg_write_scanlines (j_compress_ptr cinfo, JSAMPARRAY scanlines,
		      JDIMENSION num_lines)
{
  JDIMENSION row_ctr, rows_left;

  if (cinfo->global_state != CSTATE_SCANNING)
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_JDIMENSION);
  if (cinfo->next_scanline >= cinfo->image_height)
    WARNMS(cinfo, JWRN_TOO_MUCH_DATA);

  /* Call progress monitor hook if present */
  if (cinfo->progress != NULL) {
    cinfo->progress->pass_counter = (long) cinfo->next_scanline;
    cinfo->progress->pass_limit = (long) cinfo->image_height;
    (*cinfo->progress->progress_monitor) ((j_common_ptr) cinfo);
  }

  /* Give master control module another chance if this is first call to
   * jpeg_write_scanlines.  This lets output of the frame/scan headers be
   * delayed so that application can write COM, etc, markers between
   * jpeg_start_compress and jpeg_write_scanlines.
   */
  if (cinfo->master->call_pass_startup) {
    void_result_t pass_startup_result = (*cinfo->master->pass_startup) (cinfo);
    if (pass_startup_result.is_err) {
      return ERR_JDIMENSION(pass_startup_result.err_code);
    }
  }

  /* Ignore any extra scanlines at bottom of image. */
  rows_left = cinfo->image_height - cinfo->next_scanline;
  if (num_lines > rows_left)
    num_lines = rows_left;

  row_ctr = 0;
  void_result_t process_data_result = (*cinfo->main->process_data) (cinfo, scanlines, &row_ctr, num_lines);
  if (process_data_result.is_err) {
    return ERR_JDIMENSION(process_data_result.err_code);
  }
  cinfo->next_scanline += row_ctr;
  return RESULT_OK(jdimension, row_ctr);
}


/*
 * Alternate entry point to write raw data.
 * Processes exactly one iMCU row per call, unless suspended.
 */

J_WARN_UNUSED_RESULT GLOBAL(jdimension_result_t)
jpeg_write_raw_data (j_compress_ptr cinfo, JSAMPIMAGE data,
		     JDIMENSION num_lines)
{
  JDIMENSION lines_per_iMCU_row;

  if (cinfo->global_state != CSTATE_RAW_OK)
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_JDIMENSION);
  if (cinfo->next_scanline >= cinfo->image_height) {
    WARNMS(cinfo, JWRN_TOO_MUCH_DATA);
    return RESULT_OK(jdimension, 0);
  }

  /* Call progress monitor hook if present */
  if (cinfo->progress != NULL) {
    cinfo->progress->pass_counter = (long) cinfo->next_scanline;
    cinfo->progress->pass_limit = (long) cinfo->image_height;
    (*cinfo->progress->progress_monitor) ((j_common_ptr) cinfo);
  }

  /* Give master control module another chance if this is first call to
   * jpeg_write_raw_data.  This lets output of the frame/scan headers be
   * delayed so that application can write COM, etc, markers between
   * jpeg_start_compress and jpeg_write_raw_data.
   */
  if (cinfo->master->call_pass_startup) {
    void_result_t pass_startup_result = (*cinfo->master->pass_startup) (cinfo);
    if (pass_startup_result.is_err) {
      return ERR_JDIMENSION(pass_startup_result.err_code);
    }
  }

  /* Verify that at least one iMCU row has been passed. */
  lines_per_iMCU_row = (JDIMENSION)(cinfo->max_v_samp_factor * cinfo->data_unit);
  if (num_lines < lines_per_iMCU_row)
    ERREXIT(cinfo, JERR_BUFFER_SIZE, ERR_JDIMENSION);

  /* Directly compress the row. */
  boolean_result_t compress_data_result = (*cinfo->codec->compress_data) (cinfo, data);
  if (compress_data_result.is_err) {
    return RESULT_ERR(jdimension, compress_data_result.err_code);
  }
  if (! compress_data_result.value) {
    /* If compressor did not consume the whole row, suspend processing. */
    return RESULT_OK(jdimension, 0);
  }

  /* OK, we processed one iMCU row. */
  cinfo->next_scanline += lines_per_iMCU_row;
  return RESULT_OK(jdimension, lines_per_iMCU_row);
}
