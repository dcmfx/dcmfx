/*
 * jcarith.c
 *
 * Copyright (C) 1991-1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file holds place for arithmetic entropy encoding routines.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"


/*
 * Module initialization routine for arithmetic entropy encoding.
 */
J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_arith_encoder (j_compress_ptr cinfo);

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_arith_encoder (j_compress_ptr cinfo)
{
  ERREXIT(cinfo, JERR_ARITH_NOTIMPL, ERR_VOID);
}
