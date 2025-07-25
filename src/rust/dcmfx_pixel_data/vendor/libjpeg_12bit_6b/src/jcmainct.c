/*
 * jcmainct.c
 *
 * Copyright (C) 1994-1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains the main buffer controller for compression.
 * The main buffer lies between the pre-processor and the JPEG
 * compressor proper; it holds downsampled data in the JPEG colorspace.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"


/* Note: currently, there is no operating mode in which a full-image buffer
 * is needed at this step.  If there were, that mode could not be used with
 * "raw data" input, since this module is bypassed in that case.  However,
 * we've left the code here for possible use in special applications.
 */
#undef FULL_MAIN_BUFFER_SUPPORTED


/* Private buffer controller object */

typedef struct {
  struct jpeg_c_main_controller pub; /* public fields */

  JDIMENSION cur_iMCU_row;  /* number of current iMCU row */
  JDIMENSION rowgroup_ctr;  /* counts row groups received in iMCU row */
  boolean suspended;        /* remember if we suspended output */
  J_BUF_MODE pass_mode;     /* current operating mode */

  /* If using just a strip buffer, this points to the entire set of buffers
   * (we allocate one for each component).  In the full-image case, this
   * points to the currently accessible strips of the virtual arrays.
   */
  JSAMPARRAY buffer[MAX_COMPONENTS];

#ifdef FULL_MAIN_BUFFER_SUPPORTED
  /* If using full-image storage, this array holds pointers to virtual-array
   * control blocks for each component.  Unused if not full-image storage.
   */
  jvirt_sarray_ptr whole_image[MAX_COMPONENTS];
#endif
} my_main_controller;

typedef my_main_controller * my_main_ptr;


/* Forward declarations */
J_WARN_UNUSED_RESULT METHODDEF(void_result_t) process_data_simple_main
    JPP((j_compress_ptr cinfo, JSAMPARRAY input_buf,
         JDIMENSION *in_row_ctr, JDIMENSION in_rows_avail));
#ifdef FULL_MAIN_BUFFER_SUPPORTED
METHODDEF(void) process_data_buffer_main
    JPP((j_compress_ptr cinfo, JSAMPARRAY input_buf,
         JDIMENSION *in_row_ctr, JDIMENSION in_rows_avail));
#endif


/*
 * Initialize for a processing pass.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
start_pass_main (j_compress_ptr cinfo, J_BUF_MODE pass_mode)
{
  my_main_ptr mymain = (my_main_ptr) cinfo->main;

  /* Do nothing in raw-data mode. */
  if (cinfo->raw_data_in)
    return OK_VOID;

  mymain->cur_iMCU_row = 0; /* initialize counters */
  mymain->rowgroup_ctr = 0;
  mymain->suspended = FALSE;
  mymain->pass_mode = pass_mode;    /* save mode for use by process_data */

  switch (pass_mode) {
  case JBUF_PASS_THRU:
#ifdef FULL_MAIN_BUFFER_SUPPORTED
    if (mymain->whole_image[0] != NULL)
      ERREXIT(cinfo, JERR_BAD_BUFFER_MODE);
#endif
    mymain->pub.process_data = process_data_simple_main;
    break;
#ifdef FULL_MAIN_BUFFER_SUPPORTED
  case JBUF_SAVE_SOURCE:
  case JBUF_CRANK_DEST:
  case JBUF_SAVE_AND_PASS:
    if (mymain->whole_image[0] == NULL)
      ERREXIT(cinfo, JERR_BAD_BUFFER_MODE);
    mymain->pub.process_data = process_data_buffer_main;
    break;
#endif
  default:
    ERREXIT(cinfo, JERR_BAD_BUFFER_MODE, ERR_VOID);
    break;
  }

  return OK_VOID;
}


/*
 * Process some data.
 * This routine handles the simple pass-through mode,
 * where we have only a strip buffer.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
process_data_simple_main (j_compress_ptr cinfo,
              JSAMPARRAY input_buf, JDIMENSION *in_row_ctr,
              JDIMENSION in_rows_avail)
{
  my_main_ptr mymain = (my_main_ptr) cinfo->main;
  JDIMENSION data_unit = (JDIMENSION)(cinfo->data_unit);

  while (mymain->cur_iMCU_row < cinfo->total_iMCU_rows) {
    /* Read input data if we haven't filled the main buffer yet */
    if (mymain->rowgroup_ctr < data_unit)
      (*cinfo->prep->pre_process_data) (cinfo,
                    input_buf, in_row_ctr, in_rows_avail,
                    mymain->buffer, &mymain->rowgroup_ctr,
                    (JDIMENSION) data_unit);

    /* If we don't have a full iMCU row buffered, return to application for
     * more data.  Note that preprocessor will always pad to fill the iMCU row
     * at the bottom of the image.
     */
    if (mymain->rowgroup_ctr != data_unit)
      return OK_VOID;

    /* Send the completed row to the compressor */
    boolean_result_t compress_data_result = (*cinfo->codec->compress_data) (cinfo, mymain->buffer);
    if (compress_data_result.is_err) {
      return ERR_VOID(compress_data_result.err_code);
    }
    if (! compress_data_result.value) {
      /* If compressor did not consume the whole row, then we must need to
       * suspend processing and return to the application.  In this situation
       * we pretend we didn't yet consume the last input row; otherwise, if
       * it happened to be the last row of the image, the application would
       * think we were done.
       */
      if (! mymain->suspended) {
    (*in_row_ctr)--;
    mymain->suspended = TRUE;
      }
      return OK_VOID;
    }
    /* We did finish the row.  Undo our little suspension hack if a previous
     * call suspended; then mark the main buffer empty.
     */
    if (mymain->suspended) {
      (*in_row_ctr)++;
      mymain->suspended = FALSE;
    }
    mymain->rowgroup_ctr = 0;
    mymain->cur_iMCU_row++;
  }

  return OK_VOID;
}


#ifdef FULL_MAIN_BUFFER_SUPPORTED

/*
 * Process some data.
 * This routine handles all of the modes that use a full-size buffer.
 */

METHODDEF(void)
process_data_buffer_main (j_compress_ptr cinfo,
              JSAMPARRAY input_buf, JDIMENSION *in_row_ctr,
              JDIMENSION in_rows_avail)
{
  my_main_ptr mymain = (my_main_ptr) cinfo->main;
  int ci;
  jpeg_component_info *compptr;
  boolean writing = (mymain->pass_mode != JBUF_CRANK_DEST);
  JDIMENSION data_unit = (JDIMENSION)(cinfo->data_unit);

  while (mymain->cur_iMCU_row < cinfo->total_iMCU_rows) {
    /* Realign the virtual buffers if at the start of an iMCU row. */
    if (mymain->rowgroup_ctr == 0) {
      for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    mymain->buffer[ci] = (*cinfo->mem->access_virt_sarray)
      ((j_common_ptr) cinfo, mymain->whole_image[ci],
       mymain->cur_iMCU_row * (compptr->v_samp_factor * data_unit),
       (JDIMENSION) (compptr->v_samp_factor * data_unit), writing);
      }
      /* In a read pass, pretend we just read some source data. */
      if (! writing) {
    *in_row_ctr += cinfo->max_v_samp_factor * data_unit;
    mymain->rowgroup_ctr = data_unit;
      }
    }

    /* If a write pass, read input data until the current iMCU row is full. */
    /* Note: preprocessor will pad if necessary to fill the last iMCU row. */
    if (writing) {
      (*cinfo->prep->pre_process_data) (cinfo,
                    input_buf, in_row_ctr, in_rows_avail,
                    mymain->buffer, &mymain->rowgroup_ctr,
                    (JDIMENSION) data_unit);
      /* Return to application if we need more data to fill the iMCU row. */
      if (mymain->rowgroup_ctr < data_unit)
    return;
    }

    /* Emit data, unless this is a sink-only pass. */
    if (mymain->pass_mode != JBUF_SAVE_SOURCE) {
      if (! (*cinfo->codec->compress_data) (cinfo, mymain->buffer)) {
    /* If compressor did not consume the whole row, then we must need to
     * suspend processing and return to the application.  In this situation
     * we pretend we didn't yet consume the last input row; otherwise, if
     * it happened to be the last row of the image, the application would
     * think we were done.
     */
    if (! mymain->suspended) {
      (*in_row_ctr)--;
      mymain->suspended = TRUE;
    }
    return;
      }
      /* We did finish the row.  Undo our little suspension hack if a previous
       * call suspended; then mark the main buffer empty.
       */
      if (mymain->suspended) {
    (*in_row_ctr)++;
    mymain->suspended = FALSE;
      }
    }

    /* If get here, we are done with this iMCU row.  Mark buffer empty. */
    mymain->rowgroup_ctr = 0;
    mymain->cur_iMCU_row++;
  }
}

#endif /* FULL_MAIN_BUFFER_SUPPORTED */


/*
 * Initialize main buffer controller.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_c_main_controller (j_compress_ptr cinfo, boolean need_full_buffer)
{
  my_main_ptr mymain;
  int ci;
  jpeg_component_info *compptr;
  int data_unit = cinfo->data_unit;

  void_ptr_result_t alloc_small_result =
    (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
                SIZEOF(my_main_controller));
  if (alloc_small_result.is_err) {
    return ERR_VOID(alloc_small_result.err_code);
  }
  mymain = (my_main_ptr) alloc_small_result.value;
  cinfo->main = (struct jpeg_c_main_controller *) mymain;
  mymain->pub.start_pass = start_pass_main;

  /* We don't need to create a buffer in raw-data mode. */
  if (cinfo->raw_data_in)
    return OK_VOID;

  /* Create the buffer.  It holds downsampled data, so each component
   * may be of a different size.
   */
  if (need_full_buffer) {
#ifdef FULL_MAIN_BUFFER_SUPPORTED
    /* Allocate a full-image virtual array for each component */
    /* Note we pad the bottom to a multiple of the iMCU height */
    for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
     ci++, compptr++) {
      mymain->whole_image[ci] = (*cinfo->mem->request_virt_sarray)
    ((j_common_ptr) cinfo, JPOOL_IMAGE, FALSE,
     compptr->width_in_data_units * data_unit,
     (JDIMENSION) jround_up((long) compptr->height_in_data_units,
                (long) compptr->v_samp_factor) * data_unit,
     (JDIMENSION) (compptr->v_samp_factor * data_unit));
    }
#else
    ERREXIT(cinfo, JERR_BAD_BUFFER_MODE, ERR_VOID);
#endif
  } else {
#ifdef FULL_MAIN_BUFFER_SUPPORTED
    mymain->whole_image[0] = NULL; /* flag for no virtual arrays */
#endif
    /* Allocate a strip buffer for each component */
    for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
     ci++, compptr++) {
      jsamparray_result_t alloc_sarray_result = (*cinfo->mem->alloc_sarray)
    ((j_common_ptr) cinfo, JPOOL_IMAGE,
     compptr->width_in_data_units * (JDIMENSION)data_unit,
     (JDIMENSION) (compptr->v_samp_factor * data_unit));
      if (alloc_sarray_result.is_err) {
        return ERR_VOID(alloc_sarray_result.err_code);
      }
      mymain->buffer[ci] = alloc_sarray_result.value;
    }
  }

  return OK_VOID;
}
