/*
 * jdapimin.c
 *
 * Copyright (C) 1994-1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains application interface code for the decompression half
 * of the JPEG library.  These are the "minimum" API routines that may be
 * needed in either the normal full-decompression case or the
 * transcoding-only case.
 *
 * Most of the routines intended to be called directly by an application
 * are in this file or in jdapistd.c.  But also see jcomapi.c for routines
 * shared by compression and decompression, and jdtrans.c for the transcoding
 * case.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"


/*
 * Initialization of a JPEG decompression object.
 * The error manager must already be set up (in case memory manager fails).
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_CreateDecompress (j_decompress_ptr cinfo, int version, size_t structsize)
{
  int i;

  /* Guard against version mismatches between library and caller. */
  cinfo->mem = NULL;		/* so jpeg_destroy knows mem mgr not called */
  if (version != JPEG_LIB_VERSION)
    ERREXIT2(cinfo, JERR_BAD_LIB_VERSION, JPEG_LIB_VERSION, version, ERR_VOID);
  if (structsize != SIZEOF(struct jpeg_decompress_struct))
    ERREXIT2(cinfo, JERR_BAD_STRUCT_SIZE, 
	     (int) SIZEOF(struct jpeg_decompress_struct), (int) structsize, ERR_VOID);

  /* For debugging purposes, we zero the whole master structure.
   * But the application has already set the err pointer, and may have set
   * client_data, so we have to save and restore those fields.
   * Note: if application hasn't set client_data, tools like Purify may
   * complain here.
   */
  {
    struct jpeg_error_mgr * err = cinfo->err;
    void * client_data = cinfo->client_data; /* ignore Purify complaint here */
    MEMZERO(cinfo, SIZEOF(struct jpeg_decompress_struct));
    cinfo->err = err;
    cinfo->client_data = client_data;
  }
  cinfo->is_decompressor = TRUE;

  /* Initialize a memory manager instance for this object */
  void_result_t jinit_memory_mgr_result = jinit_memory_mgr((j_common_ptr) cinfo);
  if (jinit_memory_mgr_result.is_err)
    return jinit_memory_mgr_result;

  /* Zero out pointers to permanent structures. */
  cinfo->progress = NULL;
  cinfo->src = NULL;

  for (i = 0; i < NUM_QUANT_TBLS; i++)
    cinfo->quant_tbl_ptrs[i] = NULL;

  for (i = 0; i < NUM_HUFF_TBLS; i++) {
    cinfo->dc_huff_tbl_ptrs[i] = NULL;
    cinfo->ac_huff_tbl_ptrs[i] = NULL;
  }

  /* Initialize marker processor so application can override methods
   * for COM, APPn markers before calling jpeg_read_header.
   */
  cinfo->marker_list = NULL;
  void_result_t jinit_marker_reader_result = jinit_marker_reader(cinfo);
  if (jinit_marker_reader_result.is_err)
    return jinit_marker_reader_result;

  /* And initialize the overall input controller. */
  void_result_t jinit_input_controller_result = jinit_input_controller(cinfo);
  if (jinit_input_controller_result.is_err)
    return jinit_input_controller_result;

  /* OK, I'm ready */
  cinfo->global_state = DSTATE_START;

  return OK_VOID;
}


/*
 * Destruction of a JPEG decompression object
 */

GLOBAL(void_result_t)
jpeg_destroy_decompress (j_decompress_ptr cinfo)
{
  return jpeg_destroy((j_common_ptr) cinfo); /* use common routine */
}


/*
 * Abort processing of a JPEG decompression operation,
 * but don't destroy the object itself.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_abort_decompress (j_decompress_ptr cinfo)
{
  return jpeg_abort((j_common_ptr) cinfo); /* use common routine */
}


/*
 * Set default decompression parameters.
 */

LOCAL(void)
default_decompress_parms (j_decompress_ptr cinfo)
{
  /* Guess the input colorspace, and set output colorspace accordingly. */
  /* (Wish JPEG committee had provided a real way to specify this...) */
  /* Note application may override our guesses. */
  switch (cinfo->num_components) {
  case 1:
    cinfo->jpeg_color_space = JCS_GRAYSCALE;
    cinfo->out_color_space = JCS_GRAYSCALE;
    break;
    
  case 3:
    if (cinfo->saw_JFIF_marker) {
      cinfo->jpeg_color_space = JCS_YCbCr; /* JFIF implies YCbCr */
    } else if (cinfo->saw_Adobe_marker) {
      switch (cinfo->Adobe_transform) {
      case 0:
	cinfo->jpeg_color_space = JCS_RGB;
	break;
      case 1:
	cinfo->jpeg_color_space = JCS_YCbCr;
	break;
      default:
	WARNMS1(cinfo, JWRN_ADOBE_XFORM, cinfo->Adobe_transform);
	cinfo->jpeg_color_space = JCS_YCbCr; /* assume it's YCbCr */
	break;
      }
    } else {
      /* Saw no special markers, try to guess from the component IDs */
      int cid0 = cinfo->comp_info[0].component_id;
      int cid1 = cinfo->comp_info[1].component_id;
      int cid2 = cinfo->comp_info[2].component_id;

      if (cid0 == 1 && cid1 == 2 && cid2 == 3)
	cinfo->jpeg_color_space = JCS_YCbCr; /* assume JFIF w/out marker */
      else if (cid0 == 82 && cid1 == 71 && cid2 == 66)
	cinfo->jpeg_color_space = JCS_RGB; /* ASCII 'R', 'G', 'B' */
      else {
	if (cinfo->process == JPROC_LOSSLESS) {
	  TRACEMS3(cinfo, 1, JTRC_UNKNOWN_LOSSLESS_IDS, cid0, cid1, cid2);
	  cinfo->jpeg_color_space = JCS_RGB; /* assume it's RGB */
	}
	else {  /* Lossy processes */
	  TRACEMS3(cinfo, 1, JTRC_UNKNOWN_LOSSY_IDS, cid0, cid1, cid2);
	  cinfo->jpeg_color_space = JCS_YCbCr; /* assume it's YCbCr */
	}
      }
    }
    /* Always guess RGB is proper output colorspace. */
    cinfo->out_color_space = JCS_RGB;
    break;
    
  case 4:
    if (cinfo->saw_Adobe_marker) {
      switch (cinfo->Adobe_transform) {
      case 0:
	cinfo->jpeg_color_space = JCS_CMYK;
	break;
      case 2:
	cinfo->jpeg_color_space = JCS_YCCK;
	break;
      default:
	WARNMS1(cinfo, JWRN_ADOBE_XFORM, cinfo->Adobe_transform);
	cinfo->jpeg_color_space = JCS_YCCK; /* assume it's YCCK */
	break;
      }
    } else {
      /* No special markers, assume straight CMYK. */
      cinfo->jpeg_color_space = JCS_CMYK;
    }
    cinfo->out_color_space = JCS_CMYK;
    break;
    
  default:
    cinfo->jpeg_color_space = JCS_UNKNOWN;
    cinfo->out_color_space = JCS_UNKNOWN;
    break;
  }

  /* Set defaults for other decompression parameters. */
  cinfo->scale_num = 1;		/* 1:1 scaling */
  cinfo->scale_denom = 1;
  cinfo->output_gamma = 1.0;
  cinfo->buffered_image = FALSE;
  cinfo->raw_data_out = FALSE;
  cinfo->dct_method = JDCT_DEFAULT;
  cinfo->do_fancy_upsampling = TRUE;
  cinfo->do_block_smoothing = TRUE;
  cinfo->quantize_colors = FALSE;
  /* We set these in case application only sets quantize_colors. */
  cinfo->dither_mode = JDITHER_FS;
#ifdef QUANT_2PASS_SUPPORTED
  cinfo->two_pass_quantize = TRUE;
#else
  cinfo->two_pass_quantize = FALSE;
#endif
  cinfo->desired_number_of_colors = 256;
  cinfo->colormap = NULL;
  /* Initialize for no mode change in buffered-image mode. */
  cinfo->enable_1pass_quant = FALSE;
  cinfo->enable_external_quant = FALSE;
  cinfo->enable_2pass_quant = FALSE;
}


/*
 * Decompression startup: read start of JPEG datastream to see what's there.
 * Need only initialize JPEG object and supply a data source before calling.
 *
 * This routine will read as far as the first SOS marker (ie, actual start of
 * compressed data), and will save all tables and parameters in the JPEG
 * object.  It will also initialize the decompression parameters to default
 * values, and finally return JPEG_HEADER_OK.  On return, the application may
 * adjust the decompression parameters and then call jpeg_start_decompress.
 * (Or, if the application only wanted to determine the image parameters,
 * the data need not be decompressed.  In that case, call jpeg_abort or
 * jpeg_destroy to release any temporary space.)
 * If an abbreviated (tables only) datastream is presented, the routine will
 * return JPEG_HEADER_TABLES_ONLY upon reaching EOI.  The application may then
 * re-use the JPEG object to read the abbreviated image datastream(s).
 * It is unnecessary (but OK) to call jpeg_abort in this case.
 * The JPEG_SUSPENDED return code only occurs if the data source module
 * requests suspension of the decompressor.  In this case the application
 * should load more source data and then re-call jpeg_read_header to resume
 * processing.
 * If a non-suspending data source is used and require_image is TRUE, then the
 * return code need not be inspected since only JPEG_HEADER_OK is possible.
 *
 * This routine is now just a front end to jpeg_consume_input, with some
 * extra error checking.
 */

J_WARN_UNUSED_RESULT GLOBAL(int_result_t)
jpeg_read_header (j_decompress_ptr cinfo, boolean require_image)
{
  int retcode;

  if (cinfo->global_state != DSTATE_START &&
      cinfo->global_state != DSTATE_INHEADER)
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_INT);

  int_result_t jpeg_consume_input_result = jpeg_consume_input(cinfo);
  if (jpeg_consume_input_result.is_err)
    return jpeg_consume_input_result;

  retcode = jpeg_consume_input_result.value;

  switch (retcode) {
  case JPEG_REACHED_SOS:
    retcode = JPEG_HEADER_OK;
    break;
  case JPEG_REACHED_EOI: {
    if (require_image)		/* Complain if application wanted an image */
      ERREXIT(cinfo, JERR_NO_IMAGE, ERR_INT);
    /* Reset to start state; it would be safer to require the application to
     * call jpeg_abort, but we can't change it now for compatibility reasons.
     * A side effect is to free any temporary memory (there shouldn't be any).
     */
    void_result_t jpeg_abort_result = jpeg_abort((j_common_ptr) cinfo); /* sets state = DSTATE_START */
    if (jpeg_abort_result.is_err)
      return RESULT_ERR(int, jpeg_abort_result.err_code);
    retcode = JPEG_HEADER_TABLES_ONLY;
    break;
  }
  case JPEG_SUSPENDED:
    /* no work */
    break;
  }

  return RESULT_OK(int, retcode);
}


/*
 * Consume data in advance of what the decompressor requires.
 * This can be called at any time once the decompressor object has
 * been created and a data source has been set up.
 *
 * This routine is essentially a state machine that handles a couple
 * of critical state-transition actions, namely initial setup and
 * transition from header scanning to ready-for-start_decompress.
 * All the actual input is done via the input controller's consume_input
 * method.
 */

J_WARN_UNUSED_RESULT GLOBAL(int_result_t)
jpeg_consume_input (j_decompress_ptr cinfo)
{
  int retcode = JPEG_SUSPENDED;

  /* NB: every possible DSTATE value should be listed in this switch */
  switch (cinfo->global_state) {
  case DSTATE_START:
    /* Start-of-datastream actions: reset appropriate modules */
    (*cinfo->inputctl->reset_input_controller) (cinfo);
    /* Initialize application's data source module */
    (*cinfo->src->init_source) (cinfo);
    cinfo->global_state = DSTATE_INHEADER;
    /*FALLTHROUGH*/
  case DSTATE_INHEADER: {
    int_result_t consume_input_result = (*cinfo->inputctl->consume_input) (cinfo);
    if (consume_input_result.is_err)
      return consume_input_result;
    retcode = consume_input_result.value;
    if (retcode == JPEG_REACHED_SOS) { /* Found SOS, prepare to decompress */
      /* Set up default parameters based on header data */
      default_decompress_parms(cinfo);
      /* Set global state: ready for start_decompress */
      cinfo->global_state = DSTATE_READY;
    }
    break;
  }
  case DSTATE_READY:
    /* Can't advance past first SOS until start_decompress is called */
    retcode = JPEG_REACHED_SOS;
    break;
  case DSTATE_PRELOAD:
  case DSTATE_PRESCAN:
  case DSTATE_SCANNING:
  case DSTATE_RAW_OK:
  case DSTATE_BUFIMAGE:
  case DSTATE_BUFPOST:
  case DSTATE_STOPPING: {
    int_result_t consume_input_result = (*cinfo->inputctl->consume_input) (cinfo);
    if (consume_input_result.is_err)
      return consume_input_result;
    retcode = consume_input_result.value;
    break;
  }
  default:
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_INT);
  }
  return RESULT_OK(int, retcode);
}


/*
 * Have we finished reading the input file?
 */

J_WARN_UNUSED_RESULT GLOBAL(boolean_result_t)
jpeg_input_complete (j_decompress_ptr cinfo)
{
  /* Check for valid jpeg object */
  if (cinfo->global_state < DSTATE_START ||
      cinfo->global_state > DSTATE_STOPPING)
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_BOOL);
  return RESULT_OK(boolean, cinfo->inputctl->eoi_reached);
}


/*
 * Is there more than one scan?
 */

J_WARN_UNUSED_RESULT GLOBAL(boolean_result_t)
jpeg_has_multiple_scans (j_decompress_ptr cinfo)
{
  /* Only valid after jpeg_read_header completes */
  if (cinfo->global_state < DSTATE_READY ||
      cinfo->global_state > DSTATE_STOPPING)
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_BOOL);
  return RESULT_OK(boolean, cinfo->inputctl->has_multiple_scans);
}


/*
 * Finish JPEG decompression.
 *
 * This will normally just verify the file trailer and release temp storage.
 *
 * Returns FALSE if suspended.  The return value need be inspected only if
 * a suspending data source is used.
 */

J_WARN_UNUSED_RESULT GLOBAL(boolean_result_t)
jpeg_finish_decompress (j_decompress_ptr cinfo)
{
  if ((cinfo->global_state == DSTATE_SCANNING ||
       cinfo->global_state == DSTATE_RAW_OK) && ! cinfo->buffered_image) {
    /* Terminate final pass of non-buffered mode */
    if (cinfo->output_scanline < cinfo->output_height)
      ERREXIT(cinfo, JERR_TOO_LITTLE_DATA, ERR_BOOL);
    void_result_t finish_output_pass_result = ((*cinfo->master->finish_output_pass) (cinfo));
    if (finish_output_pass_result.is_err)
      return RESULT_ERR(boolean, finish_output_pass_result.err_code);
    cinfo->global_state = DSTATE_STOPPING;
  } else if (cinfo->global_state == DSTATE_BUFIMAGE) {
    /* Finishing after a buffered-image operation */
    cinfo->global_state = DSTATE_STOPPING;
  } else if (cinfo->global_state != DSTATE_STOPPING) {
    /* STOPPING = repeat call after a suspension, anything else is error */
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_BOOL);
  }
  /* Read until EOI */
  while (! cinfo->inputctl->eoi_reached) {
    int_result_t consume_input_result = (*cinfo->inputctl->consume_input) (cinfo);
    if (consume_input_result.is_err)
      return RESULT_OK(boolean, FALSE);
    if (consume_input_result.value == JPEG_SUSPENDED)
      return RESULT_OK(boolean, FALSE);		/* Suspend, come back later */
  }
  /* Do final cleanup */
  (*cinfo->src->term_source) (cinfo);
  /* We can use jpeg_abort to release memory and reset global_state */
  void_result_t jpeg_abort_result = jpeg_abort((j_common_ptr) cinfo);
  if (jpeg_abort_result.is_err)
    return RESULT_ERR(boolean, jpeg_abort_result.err_code);

  return RESULT_OK(boolean, TRUE);
}
