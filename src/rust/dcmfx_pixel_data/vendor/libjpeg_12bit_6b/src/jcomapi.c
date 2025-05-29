/*
 * jcomapi.c
 *
 * Copyright (C) 1994-1997, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains application interface routines that are used for both
 * compression and decompression.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"


/*
 * Abort processing of a JPEG compression or decompression operation,
 * but don't destroy the object itself.
 *
 * For this, we merely clean up all the nonpermanent memory pools.
 * Note that temp files (virtual arrays) are not allowed to belong to
 * the permanent pool, so we will be able to close all temp files here.
 * Closing a data source or destination, if necessary, is the application's
 * responsibility.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_abort (j_common_ptr cinfo)
{
  int pool;

  /* Do nothing if called on a not-initialized or destroyed JPEG object. */
  if (cinfo->mem == NULL)
    return OK_VOID;

  /* Releasing pools in reverse order might help avoid fragmentation
   * with some (brain-damaged) malloc libraries.
   */
  for (pool = JPOOL_NUMPOOLS-1; pool > JPOOL_PERMANENT; pool--) {
    void_result_t free_pool_result = (*cinfo->mem->free_pool) (cinfo, pool);
    if (free_pool_result.is_err)
      return free_pool_result;
  }

  /* Reset overall state for possible reuse of object */
  if (cinfo->is_decompressor) {
    cinfo->global_state = DSTATE_START;
    /* Try to keep application from accessing now-deleted marker list.
     * A bit kludgy to do it here, but this is the most central place.
     */
    ((j_decompress_ptr) cinfo)->marker_list = NULL;
  } else {
    cinfo->global_state = CSTATE_START;
  }

  return OK_VOID;
}


/*
 * Destruction of a JPEG object.
 *
 * Everything gets deallocated except the master jpeg_compress_struct itself
 * and the error manager struct.  Both of these are supplied by the application
 * and must be freed, if necessary, by the application.  (Often they are on
 * the stack and so don't need to be freed anyway.)
 * Closing a data source or destination, if necessary, is the application's
 * responsibility.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_destroy (j_common_ptr cinfo)
{
  /* We need only tell the memory manager to release everything. */
  /* NB: mem pointer is NULL if memory mgr failed to initialize. */
  if (cinfo->mem != NULL) {
    void_result_t self_destruct_result = (*cinfo->mem->self_destruct) (cinfo);
    if (self_destruct_result.is_err)
      return self_destruct_result;
  }
  cinfo->mem = NULL;		/* be safe if jpeg_destroy is called twice */
  cinfo->global_state = 0;	/* mark it destroyed */

  return OK_VOID;
}


/*
 * Convenience routines for allocating quantization and Huffman tables.
 * (Would jutils.c be a more reasonable place to put these?)
 */

J_WARN_UNUSED_RESULT GLOBAL(jquant_tbl_ptr_result_t)
jpeg_alloc_quant_table (j_common_ptr cinfo)
{
  JQUANT_TBL *tbl;

  void_ptr_result_t alloc_small_result = (*cinfo->mem->alloc_small) (cinfo, JPOOL_PERMANENT, SIZEOF(JQUANT_TBL));
  if (alloc_small_result.is_err)
    return RESULT_ERR(jquant_tbl_ptr, alloc_small_result.err_code);
  tbl = (JQUANT_TBL *) alloc_small_result.value;
  tbl->sent_table = FALSE;	/* make sure this is false in any new table */
  return RESULT_OK(jquant_tbl_ptr, tbl);
}


J_WARN_UNUSED_RESULT GLOBAL(jhuff_tbl_ptr_result_t)
jpeg_alloc_huff_table (j_common_ptr cinfo)
{
  JHUFF_TBL *tbl;

  void_ptr_result_t alloc_small_result = (*cinfo->mem->alloc_small) (cinfo, JPOOL_PERMANENT, SIZEOF(JHUFF_TBL));
  if (alloc_small_result.is_err)
    return RESULT_ERR(jhuff_tbl_ptr, alloc_small_result.err_code);
  tbl = (JHUFF_TBL *) alloc_small_result.value;
  tbl->sent_table = FALSE;	/* make sure this is false in any new table */
  return RESULT_OK(jhuff_tbl_ptr, tbl);
}
