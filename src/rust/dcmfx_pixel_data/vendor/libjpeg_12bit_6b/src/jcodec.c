/*
 * jcodec.c
 *
 * Copyright (C) 1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains utility functions for the JPEG codec(s).
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"
#include "jlossy12.h"
#include "jlossls12.h"

#if 0
/*
 * Initialize the compression codec.
 * This is called only once, during master selection.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_c_codec (j_compress_ptr cinfo)
{
  if (cinfo->process == JPROC_LOSSLESS) {
#ifdef C_LOSSLESS_SUPPORTED
    jinit_lossless_c_codec(cinfo);
#else
    ERREXIT(cinfo, JERR_NOT_COMPILED, ERR_VOID);
#endif
  } else
    jinit_lossy_c_codec(cinfo);

  return OK_VOID;
}
#endif


/*
 * Initialize the decompression codec.
 * This is called only once, during master selection.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_d_codec (j_decompress_ptr cinfo)
{
  if (cinfo->process == JPROC_LOSSLESS) {
#ifdef D_LOSSLESS_SUPPORTED
    return jinit_lossless_d_codec(cinfo);
#else
    ERREXIT(cinfo, JERR_NOT_COMPILED, ERR_VOID);
#endif
  } else
    return jinit_lossy_d_codec(cinfo);
}
