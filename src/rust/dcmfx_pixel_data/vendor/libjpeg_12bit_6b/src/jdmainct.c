/*
 * jdmainct.c
 *
 * Copyright (C) 1994-1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains the main buffer controller for decompression.
 * The main buffer lies between the JPEG decompressor proper and the
 * post-processor; it holds downsampled data in the JPEG colorspace.
 *
 * Note that this code is bypassed in raw-data mode, since the application
 * supplies the equivalent of the main buffer in that case.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"


/*
 * In the current system design, the main buffer need never be a full-image
 * buffer; any full-height buffers will be found inside the coefficient or
 * postprocessing controllers.  Nonetheless, the main controller is not
 * trivial.  Its responsibility is to provide context rows for upsampling/
 * rescaling, and doing this in an efficient fashion is a bit tricky.
 *
 * Postprocessor input data is counted in "row groups".  A row group
 * is defined to be (v_samp_factor * codec_data_unit / min_codec_data_unit)
 * sample rows of each component.  (We require codec_data_unit values to be
 * chosen such that these numbers are integers.  In practice codec_data_unit
 * values will likely be powers of two, so we actually have the stronger
 * condition that codec_data_unit / min_codec_data_unit is an integer.)
 * Upsampling will typically produce max_v_samp_factor pixel rows from each
 * row group (times any additional scale factor that the upsampler is
 * applying).
 *
 * The decompression codec will deliver data to us one iMCU row at a time;
 * each iMCU row contains v_samp_factor * codec_data_unit sample rows, or
 * exactly min_codec_data_unit row groups.  (This amount of data corresponds
 * to one row of MCUs when the image is fully interleaved.)  Note that the
 * number of sample rows varies across components, but the number of row
 * groups does not.  Some garbage sample rows may be included in the last iMCU
 * row at the bottom of the image.
 *
 * Depending on the vertical scaling algorithm used, the upsampler may need
 * access to the sample row(s) above and below its current input row group.
 * The upsampler is required to set need_context_rows TRUE at global selection
 * time if so.  When need_context_rows is FALSE, this controller can simply
 * obtain one iMCU row at a time from the coefficient controller and dole it
 * out as row groups to the postprocessor.
 *
 * When need_context_rows is TRUE, this controller guarantees that the buffer
 * passed to postprocessing contains at least one row group's worth of samples
 * above and below the row group(s) being processed.  Note that the context
 * rows "above" the first passed row group appear at negative row offsets in
 * the passed buffer.  At the top and bottom of the image, the required
 * context rows are manufactured by duplicating the first or last real sample
 * row; this avoids having special cases in the upsampling inner loops.
 *
 * The amount of context is fixed at one row group just because that's a
 * convenient number for this controller to work with.  The existing
 * upsamplers really only need one sample row of context.  An upsampler
 * supporting arbitrary output rescaling might wish for more than one row
 * group of context when shrinking the image; tough, we don't handle that.
 * (This is justified by the assumption that downsizing will be handled mostly
 * by adjusting the codec_data_unit values, so that the actual scale factor at
 * the upsample step needn't be much less than one.)
 *
 * To provide the desired context, we have to retain the last two row groups
 * of one iMCU row while reading in the next iMCU row.  (The last row group
 * can't be processed until we have another row group for its below-context,
 * and so we have to save the next-to-last group too for its above-context.)
 * We could do this most simply by copying data around in our buffer, but
 * that'd be very slow.  We can avoid copying any data by creating a rather
 * strange pointer structure.  Here's how it works.  We allocate a workspace
 * consisting of M+2 row groups (where M = min_codec_data_unit is the number
 * of row groups per iMCU row).  We create two sets of redundant pointers to
 * the workspace.  Labeling the physical row groups 0 to M+1, the synthesized
 * pointer lists look like this:
 *                   M+1                          M-1
 * master pointer --> 0         master pointer --> 0
 *                    1                            1
 *                   ...                          ...
 *                   M-3                          M-3
 *                   M-2                           M
 *                   M-1                          M+1
 *                    M                           M-2
 *                   M+1                          M-1
 *                    0                            0
 * We read alternate iMCU rows using each master pointer; thus the last two
 * row groups of the previous iMCU row remain un-overwritten in the workspace.
 * The pointer lists are set up so that the required context rows appear to
 * be adjacent to the proper places when we pass the pointer lists to the
 * upsampler.
 *
 * The above pictures describe the normal state of the pointer lists.
 * At top and bottom of the image, we diddle the pointer lists to duplicate
 * the first or last sample row as necessary (this is cheaper than copying
 * sample rows around).
 *
 * This scheme breaks down if M < 2, ie, min_codec_data_unit is 1.  In that
 * situation each iMCU row provides only one row group so the buffering logic
 * must be different (eg, we must read two iMCU rows before we can emit the
 * first row group).  For now, we simply do not support providing context
 * rows when min_codec_data_unit is 1.  That combination seems unlikely to
 * be worth providing --- if someone wants a 1/8th-size preview, they probably
 * want it quick and dirty, so a context-free upsampler is sufficient.
 */


/* Private buffer controller object */

typedef struct {
  struct jpeg_d_main_controller pub; /* public fields */

  /* Pointer to allocated workspace (M or M+2 row groups). */
  JSAMPARRAY buffer[MAX_COMPONENTS];

  boolean buffer_full;		/* Have we gotten an iMCU row from decoder? */
  JDIMENSION rowgroup_ctr;	/* counts row groups output to postprocessor */

  /* Remaining fields are only used in the context case. */

  /* These are the master pointers to the funny-order pointer lists. */
  JSAMPIMAGE xbuffer[2];	/* pointers to weird pointer lists */

  int whichptr;			/* indicates which pointer set is now in use */
  int context_state;		/* process_data state machine status */
  JDIMENSION rowgroups_avail;	/* row groups available to postprocessor */
  JDIMENSION iMCU_row_ctr;	/* counts iMCU rows to detect image top/bot */
} my_main_controller;

typedef my_main_controller * my_main_ptr;

/* context_state values: */
#define CTX_PREPARE_FOR_IMCU	0	/* need to prepare for MCU row */
#define CTX_PROCESS_IMCU	1	/* feeding iMCU to postprocessor */
#define CTX_POSTPONED_ROW	2	/* feeding postponed row group */


/* Forward declarations */
J_WARN_UNUSED_RESULT METHODDEF(void_result_t) process_data_simple_main
	JPP((j_decompress_ptr cinfo, JSAMPARRAY output_buf,
	     JDIMENSION *out_row_ctr, JDIMENSION out_rows_avail));
J_WARN_UNUSED_RESULT METHODDEF(void_result_t) process_data_context_main
	JPP((j_decompress_ptr cinfo, JSAMPARRAY output_buf,
	     JDIMENSION *out_row_ctr, JDIMENSION out_rows_avail));
#ifdef QUANT_2PASS_SUPPORTED
J_WARN_UNUSED_RESULT METHODDEF(void_result_t) process_data_crank_post
	JPP((j_decompress_ptr cinfo, JSAMPARRAY output_buf,
	     JDIMENSION *out_row_ctr, JDIMENSION out_rows_avail));
#endif


J_WARN_UNUSED_RESULT LOCAL(void_result_t)
alloc_funny_pointers (j_decompress_ptr cinfo)
/* Allocate space for the funny pointer lists.
 * This is done only once, not once per pass.
 */
{
  my_main_ptr mymain = (my_main_ptr) cinfo->main;
  int ci, rgroup;
  int M = cinfo->min_codec_data_unit;
  jpeg_component_info *compptr;
  JSAMPARRAY xbuf;

  /* Get top-level space for component array pointers.
   * We alloc both arrays with one call to save a few cycles.
   */
  void_ptr_result_t alloc_small_result =
    (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
				(size_t)cinfo->num_components * 2 * SIZEOF(JSAMPARRAY));
  if (alloc_small_result.is_err)
    return ERR_VOID(alloc_small_result.err_code);
  mymain->xbuffer[0] = (JSAMPIMAGE) alloc_small_result.value;
  mymain->xbuffer[1] = mymain->xbuffer[0] + cinfo->num_components;

  for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    rgroup = (compptr->v_samp_factor * compptr->codec_data_unit) /
      cinfo->min_codec_data_unit; /* height of a row group of component */
    /* Get space for pointer lists --- M+4 row groups in each list.
     * We alloc both pointer lists with one call to save a few cycles.
     */
    alloc_small_result =
      (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
				  2 * (size_t)(rgroup * (M + 4)) * SIZEOF(JSAMPROW));
    if (alloc_small_result.is_err)
      return ERR_VOID(alloc_small_result.err_code);
    xbuf = (JSAMPARRAY) alloc_small_result.value;
    xbuf += rgroup;		/* want one row group at negative offsets */
    mymain->xbuffer[0][ci] = xbuf;
    xbuf += rgroup * (M + 4);
    mymain->xbuffer[1][ci] = xbuf;
  }

  return OK_VOID;
}


LOCAL(void)
make_funny_pointers (j_decompress_ptr cinfo)
/* Create the funny pointer lists discussed in the comments above.
 * The actual workspace is already allocated (in main->buffer),
 * and the space for the pointer lists is allocated too.
 * This routine just fills in the curiously ordered lists.
 * This will be repeated at the beginning of each pass.
 */
{
  my_main_ptr mymain = (my_main_ptr) cinfo->main;
  int ci, i, rgroup;
  int M = cinfo->min_codec_data_unit;
  jpeg_component_info *compptr;
  JSAMPARRAY buf, xbuf0, xbuf1;

  for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    rgroup = (compptr->v_samp_factor * compptr->codec_data_unit) /
      cinfo->min_codec_data_unit; /* height of a row group of component */
    xbuf0 = mymain->xbuffer[0][ci];
    xbuf1 = mymain->xbuffer[1][ci];
    /* First copy the workspace pointers as-is */
    buf = mymain->buffer[ci];
    for (i = 0; i < rgroup * (M + 2); i++) {
      xbuf0[i] = xbuf1[i] = buf[i];
    }
    /* In the second list, put the last four row groups in swapped order */
    for (i = 0; i < rgroup * 2; i++) {
      xbuf1[rgroup*(M-2) + i] = buf[rgroup*M + i];
      xbuf1[rgroup*M + i] = buf[rgroup*(M-2) + i];
    }
    /* The wraparound pointers at top and bottom will be filled later
     * (see set_wraparound_pointers, below).  Initially we want the "above"
     * pointers to duplicate the first actual data line.  This only needs
     * to happen in xbuffer[0].
     */
    for (i = 0; i < rgroup; i++) {
      xbuf0[i - rgroup] = xbuf0[0];
    }
  }
}


LOCAL(void)
set_wraparound_pointers (j_decompress_ptr cinfo)
/* Set up the "wraparound" pointers at top and bottom of the pointer lists.
 * This changes the pointer list state from top-of-image to the normal state.
 */
{
  my_main_ptr mymain = (my_main_ptr) cinfo->main;
  int ci, i, rgroup;
  int M = cinfo->min_codec_data_unit;
  jpeg_component_info *compptr;
  JSAMPARRAY xbuf0, xbuf1;

  for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    rgroup = (compptr->v_samp_factor * compptr->codec_data_unit) /
      cinfo->min_codec_data_unit; /* height of a row group of component */
    xbuf0 = mymain->xbuffer[0][ci];
    xbuf1 = mymain->xbuffer[1][ci];
    for (i = 0; i < rgroup; i++) {
      xbuf0[i - rgroup] = xbuf0[rgroup*(M+1) + i];
      xbuf1[i - rgroup] = xbuf1[rgroup*(M+1) + i];
      xbuf0[rgroup*(M+2) + i] = xbuf0[i];
      xbuf1[rgroup*(M+2) + i] = xbuf1[i];
    }
  }
}


LOCAL(void)
set_bottom_pointers (j_decompress_ptr cinfo)
/* Change the pointer lists to duplicate the last sample row at the bottom
 * of the image.  whichptr indicates which xbuffer holds the final iMCU row.
 * Also sets rowgroups_avail to indicate number of nondummy row groups in row.
 */
{
  my_main_ptr mymain = (my_main_ptr) cinfo->main;
  int ci, i, rgroup, iMCUheight, rows_left;
  jpeg_component_info *compptr;
  JSAMPARRAY xbuf;

  for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    /* Count sample rows in one iMCU row and in one row group */
    iMCUheight = compptr->v_samp_factor * compptr->codec_data_unit;
    rgroup = iMCUheight / cinfo->min_codec_data_unit;
    /* Count nondummy sample rows remaining for this component */
    rows_left = (int) (compptr->downsampled_height % (JDIMENSION) iMCUheight);
    if (rows_left == 0) rows_left = iMCUheight;
    /* Count nondummy row groups.  Should get same answer for each component,
     * so we need only do it once.
     */
    if (ci == 0) {
      mymain->rowgroups_avail = (JDIMENSION) ((rows_left-1) / rgroup + 1);
    }
    /* Duplicate the last real sample row rgroup*2 times; this pads out the
     * last partial rowgroup and ensures at least one full rowgroup of context.
     */
    xbuf = mymain->xbuffer[mymain->whichptr][ci];
    for (i = 0; i < rgroup * 2; i++) {
      xbuf[rows_left + i] = xbuf[rows_left-1];
    }
  }
}


/*
 * Initialize for a processing pass.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
start_pass_main (j_decompress_ptr cinfo, J_BUF_MODE pass_mode)
{
  my_main_ptr mymain = (my_main_ptr) cinfo->main;

  switch (pass_mode) {
  case JBUF_PASS_THRU:
    if (cinfo->upsample->need_context_rows) {
      mymain->pub.process_data = process_data_context_main;
      make_funny_pointers(cinfo); /* Create the xbuffer[] lists */
      mymain->whichptr = 0;	/* Read first iMCU row into xbuffer[0] */
      mymain->context_state = CTX_PREPARE_FOR_IMCU;
      mymain->iMCU_row_ctr = 0;
    } else {
      /* Simple case with no context needed */
      mymain->pub.process_data = process_data_simple_main;
    }
    mymain->buffer_full = FALSE;	/* Mark buffer empty */
    mymain->rowgroup_ctr = 0;
    break;
#ifdef QUANT_2PASS_SUPPORTED
  case JBUF_CRANK_DEST:
    /* For last pass of 2-pass quantization, just crank the postprocessor */
    mymain->pub.process_data = process_data_crank_post;
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
 * This handles the simple case where no context is required.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
process_data_simple_main (j_decompress_ptr cinfo,
			  JSAMPARRAY output_buf, JDIMENSION *out_row_ctr,
			  JDIMENSION out_rows_avail)
{
  my_main_ptr mymain = (my_main_ptr) cinfo->main;
  JDIMENSION rowgroups_avail;

  /* Read input data if we haven't filled the main buffer yet */
  if (! mymain->buffer_full) {
    int_result_t decompress_data_result = (*cinfo->codec->decompress_data) (cinfo, mymain->buffer);
    if (decompress_data_result.is_err)
      return ERR_VOID(decompress_data_result.err_code);
    if (!decompress_data_result.value)
      return OK_VOID;			/* suspension forced, can do nothing more */
    mymain->buffer_full = TRUE;	/* OK, we have an iMCU row to work with */
  }

  /* There are always min_codec_data_unit row groups in an iMCU row. */
  rowgroups_avail = (JDIMENSION) cinfo->min_codec_data_unit;
  /* Note: at the bottom of the image, we may pass extra garbage row groups
   * to the postprocessor.  The postprocessor has to check for bottom
   * of image anyway (at row resolution), so no point in us doing it too.
   */

  /* Feed the postprocessor */
  void_result_t post_process_data_result = (*cinfo->post->post_process_data) (cinfo, mymain->buffer,
				     &mymain->rowgroup_ctr, rowgroups_avail,
				     output_buf, out_row_ctr, out_rows_avail);
  if (post_process_data_result.is_err)
    return post_process_data_result;

  /* Has postprocessor consumed all the data yet? If so, mark buffer empty */
  if (mymain->rowgroup_ctr >= rowgroups_avail) {
    mymain->buffer_full = FALSE;
    mymain->rowgroup_ctr = 0;
  }

  return OK_VOID;
}


/*
 * Process some data.
 * This handles the case where context rows must be provided.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
process_data_context_main (j_decompress_ptr cinfo,
			   JSAMPARRAY output_buf, JDIMENSION *out_row_ctr,
			   JDIMENSION out_rows_avail)
{
  my_main_ptr mymain = (my_main_ptr) cinfo->main;

  /* Read input data if we haven't filled the main buffer yet */
  if (! mymain->buffer_full) {
    int_result_t decompress_data_result = (*cinfo->codec->decompress_data) (cinfo,
      mymain->xbuffer[mymain->whichptr]);
    if (decompress_data_result.is_err)
      return ERR_VOID(decompress_data_result.err_code);
    if (!decompress_data_result.value)
      return OK_VOID;			/* suspension forced, can do nothing more */
    mymain->buffer_full = TRUE;	/* OK, we have an iMCU row to work with */
    mymain->iMCU_row_ctr++;	/* count rows received */
  }

  /* Postprocessor typically will not swallow all the input data it is handed
   * in one call (due to filling the output buffer first).  Must be prepared
   * to exit and restart.  This switch lets us keep track of how far we got.
   * Note that each case falls through to the next on successful completion.
   */
  switch (mymain->context_state) {
  case CTX_POSTPONED_ROW: {
    /* Call postprocessor using previously set pointers for postponed row */
    void_result_t post_process_data_result = (*cinfo->post->post_process_data) (cinfo, mymain->xbuffer[mymain->whichptr],
			&mymain->rowgroup_ctr, mymain->rowgroups_avail,
			output_buf, out_row_ctr, out_rows_avail);
    if (post_process_data_result.is_err)
      return post_process_data_result;
    if (mymain->rowgroup_ctr < mymain->rowgroups_avail)
      return OK_VOID;			/* Need to suspend */
    mymain->context_state = CTX_PREPARE_FOR_IMCU;
    if (*out_row_ctr >= out_rows_avail)
      return OK_VOID;			/* Postprocessor exactly filled output buf */
    /*FALLTHROUGH*/
  }
  case CTX_PREPARE_FOR_IMCU:
    /* Prepare to process first M-1 row groups of this iMCU row */
    mymain->rowgroup_ctr = 0;
    mymain->rowgroups_avail = (JDIMENSION) (cinfo->min_codec_data_unit - 1);
    /* Check for bottom of image: if so, tweak pointers to "duplicate"
     * the last sample row, and adjust rowgroups_avail to ignore padding rows.
     */
    if (mymain->iMCU_row_ctr == cinfo->total_iMCU_rows)
      set_bottom_pointers(cinfo);
    mymain->context_state = CTX_PROCESS_IMCU;
    /*FALLTHROUGH*/
  case CTX_PROCESS_IMCU: {
    /* Call postprocessor using previously set pointers */
    void_result_t post_process_data_result = (*cinfo->post->post_process_data) (cinfo, mymain->xbuffer[mymain->whichptr],
			&mymain->rowgroup_ctr, mymain->rowgroups_avail,
			output_buf, out_row_ctr, out_rows_avail);
    if (post_process_data_result.is_err)
      return post_process_data_result;
    if (mymain->rowgroup_ctr < mymain->rowgroups_avail)
      return OK_VOID;			/* Need to suspend */
    /* After the first iMCU, change wraparound pointers to normal state */
    if (mymain->iMCU_row_ctr == 1)
      set_wraparound_pointers(cinfo);
    /* Prepare to load new iMCU row using other xbuffer list */
    mymain->whichptr ^= 1;	/* 0=>1 or 1=>0 */
    mymain->buffer_full = FALSE;
    /* Still need to process last row group of this iMCU row, */
    /* which is saved at index M+1 of the other xbuffer */
    mymain->rowgroup_ctr = (JDIMENSION) (cinfo->min_codec_data_unit + 1);
    mymain->rowgroups_avail = (JDIMENSION) (cinfo->min_codec_data_unit + 2);
    mymain->context_state = CTX_POSTPONED_ROW;
  }
  }

  return OK_VOID;
}


/*
 * Process some data.
 * Final pass of two-pass quantization: just call the postprocessor.
 * Source data will be the postprocessor controller's internal buffer.
 */

#ifdef QUANT_2PASS_SUPPORTED

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
process_data_crank_post (j_decompress_ptr cinfo,
			 JSAMPARRAY output_buf, JDIMENSION *out_row_ctr,
			 JDIMENSION out_rows_avail)
{
  return (*cinfo->post->post_process_data) (cinfo, (JSAMPIMAGE) NULL,
				     (JDIMENSION *) NULL, (JDIMENSION) 0,
				     output_buf, out_row_ctr, out_rows_avail);
}

#endif /* QUANT_2PASS_SUPPORTED */


/*
 * Initialize main buffer controller.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_d_main_controller (j_decompress_ptr cinfo, boolean need_full_buffer)
{
  my_main_ptr mymain;
  int ci, rgroup, ngroups;
  jpeg_component_info *compptr;

  void_ptr_result_t alloc_small_result =
    (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
				SIZEOF(my_main_controller));
  if (alloc_small_result.is_err)
    return ERR_VOID(alloc_small_result.err_code);
  mymain = (my_main_ptr) alloc_small_result.value;
  cinfo->main = (struct jpeg_d_main_controller *) mymain;
  mymain->pub.start_pass = start_pass_main;

  if (need_full_buffer)		/* shouldn't happen */
    ERREXIT(cinfo, JERR_BAD_BUFFER_MODE, ERR_VOID);

  /* Allocate the workspace.
   * ngroups is the number of row groups we need.
   */
  if (cinfo->upsample->need_context_rows) {
    if (cinfo->min_codec_data_unit < 2) /* unsupported, see comments above */
      ERREXIT(cinfo, JERR_NOTIMPL, ERR_VOID);
    void_result_t alloc_funny_pointers_result = alloc_funny_pointers(cinfo);
    if (alloc_funny_pointers_result.is_err) /* Alloc space for xbuffer[] lists */
      return alloc_funny_pointers_result;
    ngroups = cinfo->min_codec_data_unit + 2;
  } else {
    ngroups = cinfo->min_codec_data_unit;
  }

  for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    rgroup = (compptr->v_samp_factor * compptr->codec_data_unit) /
      cinfo->min_codec_data_unit; /* height of a row group of component */
    jsamparray_result_t alloc_sarray_result =
      (*cinfo->mem->alloc_sarray)
			  ((j_common_ptr) cinfo, JPOOL_IMAGE,
			   compptr->width_in_data_units * (JDIMENSION) compptr->codec_data_unit,
			   (JDIMENSION) (rgroup * ngroups));
    if (alloc_sarray_result.is_err)
      return ERR_VOID(alloc_sarray_result.err_code);
    mymain->buffer[ci] = alloc_sarray_result.value;
  }

  return OK_VOID;
}
