/*
 * jcapimin.c
 *
 * Copyright (C) 1994-1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains application interface code for the compression half
 * of the JPEG library.  These are the "minimum" API routines that may be
 * needed in either the normal full-compression case or the transcoding-only
 * case.
 *
 * Most of the routines intended to be called directly by an application
 * are in this file or in jcapistd.c.  But also see jcparam.c for
 * parameter-setup helper routines, jcomapi.c for routines shared by
 * compression and decompression, and jctrans.c for the transcoding case.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"


/*
 * Initialization of a JPEG compression object.
 * The error manager must already be set up (in case memory manager fails).
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_CreateCompress (j_compress_ptr cinfo, int version, size_t structsize)
{
  int i;

  /* Guard against version mismatches between library and caller. */
  cinfo->mem = NULL;		/* so jpeg_destroy knows mem mgr not called */
  if (version != JPEG_LIB_VERSION)
    ERREXIT2(cinfo, JERR_BAD_LIB_VERSION, JPEG_LIB_VERSION, version, ERR_VOID);
  if (structsize != SIZEOF(struct jpeg_compress_struct))
    ERREXIT2(cinfo, JERR_BAD_STRUCT_SIZE, 
	     (int) SIZEOF(struct jpeg_compress_struct), (int) structsize, ERR_VOID);

  /* For debugging purposes, we zero the whole master structure.
   * But the application has already set the err pointer, and may have set
   * client_data, so we have to save and restore those fields.
   * Note: if application hasn't set client_data, tools like Purify may
   * complain here.
   */
  {
    struct jpeg_error_mgr * err = cinfo->err;
    void * client_data = cinfo->client_data; /* ignore Purify complaint here */
    MEMZERO(cinfo, SIZEOF(struct jpeg_compress_struct));
    cinfo->err = err;
    cinfo->client_data = client_data;
  }
  cinfo->is_decompressor = FALSE;

  /* Initialize a memory manager instance for this object */
  void_result_t jinit_memory_mgr_result = jinit_memory_mgr((j_common_ptr) cinfo);
  if (jinit_memory_mgr_result.is_err) {
    return jinit_memory_mgr_result;
  }

  /* Zero out pointers to permanent structures. */
  cinfo->progress = NULL;
  cinfo->dest = NULL;

  cinfo->comp_info = NULL;

  for (i = 0; i < NUM_QUANT_TBLS; i++)
    cinfo->quant_tbl_ptrs[i] = NULL;

  for (i = 0; i < NUM_HUFF_TBLS; i++) {
    cinfo->dc_huff_tbl_ptrs[i] = NULL;
    cinfo->ac_huff_tbl_ptrs[i] = NULL;
  }

  cinfo->script_space = NULL;

  cinfo->input_gamma = 1.0;	/* in case application forgets */

  /* OK, I'm ready */
  cinfo->global_state = CSTATE_START;

  return OK_VOID;
}


/*
 * Destruction of a JPEG compression object
 */

GLOBAL(void_result_t)
jpeg_destroy_compress (j_compress_ptr cinfo)
{
  return jpeg_destroy((j_common_ptr) cinfo); /* use common routine */
}


/*
 * Abort processing of a JPEG compression operation,
 * but don't destroy the object itself.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_abort_compress (j_compress_ptr cinfo)
{
  return jpeg_abort((j_common_ptr) cinfo); /* use common routine */
}


/*
 * Forcibly suppress or un-suppress all quantization and Huffman tables.
 * Marks all currently defined tables as already written (if suppress)
 * or not written (if !suppress).  This will control whether they get emitted
 * by a subsequent jpeg_start_compress call.
 *
 * This routine is exported for use by applications that want to produce
 * abbreviated JPEG datastreams.  It logically belongs in jcparam.c, but
 * since it is called by jpeg_start_compress, we put it here --- otherwise
 * jcparam.o would be linked whether the application used it or not.
 */

GLOBAL(void)
jpeg_suppress_tables (j_compress_ptr cinfo, boolean suppress)
{
  int i;
  JQUANT_TBL * qtbl;
  JHUFF_TBL * htbl;

  for (i = 0; i < NUM_QUANT_TBLS; i++) {
    if ((qtbl = cinfo->quant_tbl_ptrs[i]) != NULL)
      qtbl->sent_table = suppress;
  }

  for (i = 0; i < NUM_HUFF_TBLS; i++) {
    if ((htbl = cinfo->dc_huff_tbl_ptrs[i]) != NULL)
      htbl->sent_table = suppress;
    if ((htbl = cinfo->ac_huff_tbl_ptrs[i]) != NULL)
      htbl->sent_table = suppress;
  }
}


/*
 * Finish JPEG compression.
 *
 * If a multipass operating mode was selected, this may do a great deal of
 * work including most of the actual output.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_finish_compress (j_compress_ptr cinfo)
{
  JDIMENSION iMCU_row;

  if (cinfo->global_state == CSTATE_SCANNING ||
      cinfo->global_state == CSTATE_RAW_OK) {
    /* Terminate first pass */
    if (cinfo->next_scanline < cinfo->image_height)
      ERREXIT(cinfo, JERR_TOO_LITTLE_DATA, ERR_VOID);
    void_result_t finish_pass_result = (*cinfo->master->finish_pass) (cinfo);
    if (finish_pass_result.is_err) {
      return finish_pass_result;
    }
  } else if (cinfo->global_state != CSTATE_WRCOEFS)
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_VOID);
  /* Perform any remaining passes */
  while (! cinfo->master->is_last_pass) {
    void_result_t prepare_for_pass_result = (*cinfo->master->prepare_for_pass) (cinfo);
    if (prepare_for_pass_result.is_err) {
      return prepare_for_pass_result;
    }
    for (iMCU_row = 0; iMCU_row < cinfo->total_iMCU_rows; iMCU_row++) {
      if (cinfo->progress != NULL) {
	cinfo->progress->pass_counter = (long) iMCU_row;
	cinfo->progress->pass_limit = (long) cinfo->total_iMCU_rows;
	(*cinfo->progress->progress_monitor) ((j_common_ptr) cinfo);
      }
      /* We bypass the main controller and invoke coef controller directly;
       * all work is being done from the coefficient buffer.
       */
      boolean_result_t compress_data_result = (*cinfo->codec->compress_data) (cinfo, (JSAMPIMAGE) NULL);
      if (compress_data_result.is_err) {
        return ERR_VOID(compress_data_result.err_code);
      }
      if (! compress_data_result.value)
	ERREXIT(cinfo, JERR_CANT_SUSPEND, ERR_VOID);
    }
    void_result_t finish_pass_result = (*cinfo->master->finish_pass) (cinfo);
    if (finish_pass_result.is_err) {
      return finish_pass_result;
    }
  }
  /* Write EOI, do final cleanup */
  void_result_t write_file_trailer_result = (*cinfo->marker->write_file_trailer) (cinfo);
  if (write_file_trailer_result.is_err) {
    return write_file_trailer_result;
  }
  void_result_t term_destination_result = (*cinfo->dest->term_destination) (cinfo);
  if (term_destination_result.is_err) {
    return term_destination_result;
  }
  /* We can use jpeg_abort to release memory and reset global_state */
  void_result_t jpeg_abort_result = jpeg_abort((j_common_ptr) cinfo);
  if (jpeg_abort_result.is_err) {
    return jpeg_abort_result;
  }

  return OK_VOID;
}


/*
 * Write a special marker.
 * This is only recommended for writing COM or APPn markers.
 * Must be called after jpeg_start_compress() and before
 * first call to jpeg_write_scanlines() or jpeg_write_raw_data().
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_write_marker (j_compress_ptr cinfo, int marker,
		   const JOCTET *dataptr, unsigned int datalen)
{
  J_WARN_UNUSED_RESULT JMETHOD(void_result_t, write_marker_byte, (j_compress_ptr info, int val));

  if (cinfo->next_scanline != 0 ||
      (cinfo->global_state != CSTATE_SCANNING &&
       cinfo->global_state != CSTATE_RAW_OK &&
       cinfo->global_state != CSTATE_WRCOEFS))
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_VOID);

  void_result_t write_marker_header_result = (*cinfo->marker->write_marker_header) (cinfo, marker, datalen);
  if (write_marker_header_result.is_err) {
    return write_marker_header_result;
  }

  write_marker_byte = cinfo->marker->write_marker_byte;	/* copy for speed */
  while (datalen--) {
    void_result_t write_marker_byte_result = (*write_marker_byte) (cinfo, *dataptr);
    if (write_marker_byte_result.is_err) {
      return write_marker_byte_result;
    }
    dataptr++;
  }

  return OK_VOID;
}

/* Same, but piecemeal. */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_write_m_header (j_compress_ptr cinfo, int marker, unsigned int datalen)
{
  if (cinfo->next_scanline != 0 ||
      (cinfo->global_state != CSTATE_SCANNING &&
       cinfo->global_state != CSTATE_RAW_OK &&
       cinfo->global_state != CSTATE_WRCOEFS))
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_VOID);

  return (*cinfo->marker->write_marker_header) (cinfo, marker, datalen);
}

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_write_m_byte (j_compress_ptr cinfo, int val)
{
  return (*cinfo->marker->write_marker_byte) (cinfo, val);
}


/*
 * Alternate compression function: just write an abbreviated table file.
 * Before calling this, all parameters and a data destination must be set up.
 *
 * To produce a pair of files containing abbreviated tables and abbreviated
 * image data, one would proceed as follows:
 *
 *		initialize JPEG object
 *		set JPEG parameters
 *		set destination to table file
 *		jpeg_write_tables(cinfo);
 *		set destination to image file
 *		jpeg_start_compress(cinfo, FALSE);
 *		write data...
 *		jpeg_finish_compress(cinfo);
 *
 * jpeg_write_tables has the side effect of marking all tables written
 * (same as jpeg_suppress_tables(..., TRUE)).  Thus a subsequent start_compress
 * will not re-emit the tables unless it is passed write_all_tables=TRUE.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_write_tables (j_compress_ptr cinfo)
{
  if (cinfo->global_state != CSTATE_START)
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_VOID);

  /* (Re)initialize error mgr and destination modules */
  (*cinfo->err->reset_error_mgr) ((j_common_ptr) cinfo);
  void_result_t init_destination_result = (*cinfo->dest->init_destination) (cinfo);
  if (init_destination_result.is_err) {
    return init_destination_result;
  }
  /* Initialize the marker writer ... bit of a crock to do it here. */
  void_result_t jinit_marker_writer_result = jinit_marker_writer(cinfo);
  if (jinit_marker_writer_result.is_err) {
    return jinit_marker_writer_result;
  }
  /* Write them tables! */
  void_result_t write_tables_only_result = (*cinfo->marker->write_tables_only) (cinfo);
  if (write_tables_only_result.is_err) {
    return write_tables_only_result;
  }
  /* And clean up. */
  void_result_t term_destination_result = (*cinfo->dest->term_destination) (cinfo);
  if (term_destination_result.is_err) {
    return term_destination_result;
  }
  /*
   * In library releases up through v6a, we called jpeg_abort() here to free
   * any working memory allocated by the destination manager and marker
   * writer.  Some applications had a problem with that: they allocated space
   * of their own from the library memory manager, and didn't want it to go
   * away during write_tables.  So now we do nothing.  This will cause a
   * memory leak if an app calls write_tables repeatedly without doing a full
   * compression cycle or otherwise resetting the JPEG object.  However, that
   * seems less bad than unexpectedly freeing memory in the normal case.
   * An app that prefers the old behavior can call jpeg_abort for itself after
   * each call to jpeg_write_tables().
   */

   return OK_VOID;
}
