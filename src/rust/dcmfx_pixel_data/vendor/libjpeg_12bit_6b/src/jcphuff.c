/*
 * jcphuff.c
 *
 * Copyright (C) 1995-1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains Huffman entropy encoding routines for progressive JPEG.
 *
 * We do not support output suspension in this module, since the library
 * currently does not allow multiple-scan files to be written with output
 * suspension.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"
#include "jlossy12.h"       /* Private declarations for lossy codec */
#include "jchuff12.h"       /* Declarations shared with jc*huff.c */

#ifdef C_PROGRESSIVE_SUPPORTED

/* Expanded entropy encoder object for progressive Huffman encoding. */

typedef struct {
  /* Mode flag: TRUE for optimization, FALSE for actual data output */
  boolean gather_statistics;

  /* Bit-level coding status.
   * next_output_byte/free_in_buffer are local copies of cinfo->dest fields.
   */
  JOCTET * next_output_byte;    /* => next byte to write in buffer */
  size_t free_in_buffer;    /* # of byte spaces remaining in buffer */
  IJG_INT32 put_buffer;     /* current bit-accumulation buffer */
  int put_bits;         /* # of bits now in it */
  j_compress_ptr cinfo;     /* link to cinfo (needed for dump_buffer) */

  /* Coding status for DC components */
  int last_dc_val[MAX_COMPS_IN_SCAN]; /* last DC coef for each component */

  /* Coding status for AC components */
  int ac_tbl_no;        /* the table number of the single component */
  unsigned int EOBRUN;      /* run length of EOBs */
  unsigned int BE;      /* # of buffered correction bits before MCU */
  char * bit_buffer;        /* buffer for correction bits (1 per char) */
  /* packing correction bits tightly would save some space but cost time... */

  unsigned int restarts_to_go;  /* MCUs left in this restart interval */
  int next_restart_num;     /* next restart number to write (0-7) */

  /* Pointers to derived tables (these workspaces have image lifespan).
   * Since any one scan codes only DC or only AC, we only need one set
   * of tables, not one for DC and one for AC.
   */
  c_derived_tbl * derived_tbls[NUM_HUFF_TBLS];

  /* Statistics tables for optimization; again, one set is enough */
  long * count_ptrs[NUM_HUFF_TBLS];
} phuff_entropy_encoder;

typedef phuff_entropy_encoder * phuff_entropy_ptr;

/* MAX_CORR_BITS is the number of bits the AC refinement correction-bit
 * buffer can hold.  Larger sizes may slightly improve compression, but
 * 1000 is already well into the realm of overkill.
 * The minimum safe size is 64 bits.
 */

#define MAX_CORR_BITS  1000 /* Max # of correction bits I can buffer */

/* IRIGHT_SHIFT is like RIGHT_SHIFT, but works on int rather than IJG_INT32.
 * We assume that int right shift is unsigned if IJG_INT32 right shift is,
 * which should be safe.
 */

#ifdef RIGHT_SHIFT_IS_UNSIGNED
#define ISHIFT_TEMPS    int ishift_temp;
#define IRIGHT_SHIFT(x,shft)  \
    ((ishift_temp = (x)) < 0 ? \
     (ishift_temp >> (shft)) | ((~0) << (16-(shft))) : \
     (ishift_temp >> (shft)))
#else
#define ISHIFT_TEMPS
#define IRIGHT_SHIFT(x,shft)    ((x) >> (shft))
#endif

/* Forward declarations */
J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t) encode_mcu_DC_first JPP((j_compress_ptr cinfo,
                        JBLOCKROW *MCU_data));
J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t) encode_mcu_AC_first JPP((j_compress_ptr cinfo,
                        JBLOCKROW *MCU_data));
J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t) encode_mcu_DC_refine JPP((j_compress_ptr cinfo,
                         JBLOCKROW *MCU_data));
J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t) encode_mcu_AC_refine JPP((j_compress_ptr cinfo,
                         JBLOCKROW *MCU_data));
J_WARN_UNUSED_RESULT METHODDEF(void_result_t) finish_pass_phuff JPP((j_compress_ptr cinfo));
J_WARN_UNUSED_RESULT METHODDEF(void_result_t) finish_pass_gather_phuff JPP((j_compress_ptr cinfo));


/*
 * Initialize for a Huffman-compressed scan using progressive JPEG.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
start_pass_phuff (j_compress_ptr cinfo, boolean gather_statistics)
{  
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  phuff_entropy_ptr entropy = (phuff_entropy_ptr) lossyc->entropy_private;
  boolean is_DC_band;
  int ci, tbl;
  jpeg_component_info * compptr;

  entropy->cinfo = cinfo;
  entropy->gather_statistics = gather_statistics;

  is_DC_band = (cinfo->Ss == 0);

  /* We assume jcmaster.c already validated the scan parameters. */

  /* Select execution routines */
  if (cinfo->Ah == 0) {
    if (is_DC_band)
      lossyc->entropy_encode_mcu = encode_mcu_DC_first;
    else
      lossyc->entropy_encode_mcu = encode_mcu_AC_first;
  } else {
    if (is_DC_band)
      lossyc->entropy_encode_mcu = encode_mcu_DC_refine;
    else {
      lossyc->entropy_encode_mcu = encode_mcu_AC_refine;
      /* AC refinement needs a correction bit buffer */
      if (entropy->bit_buffer == NULL) {
        void_ptr_result_t alloc_small_result =
            (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
                      MAX_CORR_BITS * SIZEOF(char));
        if (alloc_small_result.is_err) {
          return ERR_VOID(alloc_small_result.err_code);
        }
        entropy->bit_buffer = (char *) alloc_small_result.value;
      }
    }
  }
  if (gather_statistics)
    lossyc->pub.entropy_finish_pass = finish_pass_gather_phuff;
  else
    lossyc->pub.entropy_finish_pass = finish_pass_phuff;

  /* Only DC coefficients may be interleaved, so cinfo->comps_in_scan = 1
   * for AC coefficients.
   */
  for (ci = 0; ci < cinfo->comps_in_scan; ci++) {
    compptr = cinfo->cur_comp_info[ci];
    /* Initialize DC predictions to 0 */
    entropy->last_dc_val[ci] = 0;
    /* Get table index */
    if (is_DC_band) {
      if (cinfo->Ah != 0)   /* DC refinement needs no table */
    continue;
      tbl = compptr->dc_tbl_no;
    } else {
      entropy->ac_tbl_no = tbl = compptr->ac_tbl_no;
    }
    if (gather_statistics) {
      /* Check for invalid table index */
      /* (make_c_derived_tbl does this in the other path) */
      if (tbl < 0 || tbl >= NUM_HUFF_TBLS)
        ERREXIT1(cinfo, JERR_NO_HUFF_TABLE, tbl, ERR_VOID);
      /* Allocate and zero the statistics tables */
      /* Note that jpeg_gen_optimal_table expects 257 entries in each table! */
      if (entropy->count_ptrs[tbl] == NULL) {
        void_ptr_result_t alloc_small_result =
          (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
                          257 * SIZEOF(long));
        if (alloc_small_result.is_err) {
          return ERR_VOID(alloc_small_result.err_code);
        }
        entropy->count_ptrs[tbl] = (long *) alloc_small_result.value;
      }
      MEMZERO(entropy->count_ptrs[tbl], 257 * SIZEOF(long));
    } else {
      /* Compute derived values for Huffman table */
      /* We may do this more than once for a table, but it's not expensive */
      void_result_t jpeg_make_c_derived_tbl_result = jpeg_make_c_derived_tbl(cinfo, is_DC_band, tbl,
                  & entropy->derived_tbls[tbl]);
      if (jpeg_make_c_derived_tbl_result.is_err) {
        return jpeg_make_c_derived_tbl_result;
      }
    }
  }

  /* Initialize AC stuff */
  entropy->EOBRUN = 0;
  entropy->BE = 0;

  /* Initialize bit buffer to empty */
  entropy->put_buffer = 0;
  entropy->put_bits = 0;

  /* Initialize restart stuff */
  entropy->restarts_to_go = cinfo->restart_interval;
  entropy->next_restart_num = 0;

  return OK_VOID;
}


/* Outputting bytes to the file.
 * NB: these must be called only when actually outputting,
 * that is, entropy->gather_statistics == FALSE.
 */

/* Emit a byte */
#define emit_byte(entropy,val)  \
    { *(entropy)->next_output_byte++ = (JOCTET) (val);  \
      if (--(entropy)->free_in_buffer == 0)  \
        { \
          void_result_t dump_buffer_result = dump_buffer(entropy); \
          if (dump_buffer_result.is_err) { \
            return dump_buffer_result; \
          } \
        } }


J_WARN_UNUSED_RESULT LOCAL(void_result_t)
dump_buffer (phuff_entropy_ptr entropy)
/* Empty the output buffer; we do not support suspension in this module. */
{
  struct jpeg_destination_mgr * dest = entropy->cinfo->dest;

  boolean_result_t empty_output_buffer_result = (*dest->empty_output_buffer) (entropy->cinfo);
  if (empty_output_buffer_result.is_err) {
    return ERR_VOID(empty_output_buffer_result.err_code);
  }
  if (! empty_output_buffer_result.value)
    ERREXIT(entropy->cinfo, JERR_CANT_SUSPEND, ERR_VOID);
  /* After a successful buffer dump, must reset buffer pointers */
  entropy->next_output_byte = dest->next_output_byte;
  entropy->free_in_buffer = dest->free_in_buffer;

  return OK_VOID;
}


/* Outputting bits to the file */

/* Only the right 24 bits of put_buffer are used; the valid bits are
 * left-justified in this part.  At most 16 bits can be passed to emit_bits
 * in one call, and we never retain more than 7 bits in put_buffer
 * between calls, so 24 bits are sufficient.
 */

INLINE
J_WARN_UNUSED_RESULT LOCAL(void_result_t)
emit_bits (phuff_entropy_ptr entropy, unsigned int code, int size)
/* Emit some bits, unless we are in gather mode */
{
  /* This routine is heavily used, so it's worth coding tightly. */
  register IJG_INT32 put_buffer = (IJG_INT32) code;
  register int put_bits = entropy->put_bits;

  /* if size is 0, caller used an invalid Huffman table entry */
  if (size == 0)
    ERREXIT(entropy->cinfo, JERR_HUFF_MISSING_CODE, ERR_VOID);

  if (entropy->gather_statistics)
    return OK_VOID;         /* do nothing if we're only getting stats */

  put_buffer &= (((IJG_INT32) 1)<<size) - 1; /* mask off any extra bits in code */
  
  put_bits += size;     /* new number of bits in buffer */
  
  put_buffer <<= 24 - put_bits; /* align incoming bits */

  put_buffer |= entropy->put_buffer; /* and merge with old buffer contents */

  while (put_bits >= 8) {
    int c = (int) ((put_buffer >> 16) & 0xFF);
    
    emit_byte(entropy, c);
    if (c == 0xFF) {        /* need to stuff a zero byte? */
      emit_byte(entropy, 0);
    }
    put_buffer <<= 8;
    put_bits -= 8;
  }

  entropy->put_buffer = put_buffer; /* update variables */
  entropy->put_bits = put_bits;

  return OK_VOID;
}


J_WARN_UNUSED_RESULT LOCAL(void_result_t)
flush_bits (phuff_entropy_ptr entropy)
{
  void_result_t emit_bits_result = emit_bits(entropy, 0x7F, 7); /* fill any partial byte with ones */
  if (emit_bits_result.is_err) {
    return emit_bits_result;
  }

  entropy->put_buffer = 0;     /* and reset bit-buffer to empty */
  entropy->put_bits = 0;

  return OK_VOID;
}


/*
 * Emit (or just count) a Huffman symbol.
 */

INLINE
J_WARN_UNUSED_RESULT LOCAL(void_result_t)
emit_symbol (phuff_entropy_ptr entropy, int tbl_no, int symbol)
{
  if (entropy->gather_statistics)
    entropy->count_ptrs[tbl_no][symbol]++;
  else {
    c_derived_tbl * tbl = entropy->derived_tbls[tbl_no];
    return emit_bits(entropy, tbl->ehufco[symbol], tbl->ehufsi[symbol]);
  }

  return OK_VOID;
}


/*
 * Emit bits from a correction bit buffer.
 */

J_WARN_UNUSED_RESULT LOCAL(void_result_t)
emit_buffered_bits (phuff_entropy_ptr entropy, char * bufstart,
            unsigned int nbits)
{
  if (entropy->gather_statistics)
    return OK_VOID;         /* no real work */

  while (nbits > 0) {
    void_result_t emit_bits_result = emit_bits(entropy, (unsigned int) (*bufstart), 1);
    if (emit_bits_result.is_err) {
      return emit_bits_result;
    }
    bufstart++;
    nbits--;
  }

  return OK_VOID;
}


/*
 * Emit any pending EOBRUN symbol.
 */

J_WARN_UNUSED_RESULT LOCAL(void_result_t)
emit_eobrun (phuff_entropy_ptr entropy)
{
  register int temp, nbits;

  if (entropy->EOBRUN > 0) {    /* if there is any pending EOBRUN */
    temp = (int)entropy->EOBRUN;
    nbits = 0;
    while ((temp >>= 1))
      nbits++;
    /* safety check: shouldn't happen given limited correction-bit buffer */
    if (nbits > 14)
      ERREXIT(entropy->cinfo, JERR_HUFF_MISSING_CODE, ERR_VOID);

    void_result_t emit_symbol_result = emit_symbol(entropy, entropy->ac_tbl_no, nbits << 4);
    if (emit_symbol_result.is_err) {
      return emit_symbol_result;
    }
    if (nbits) {
      void_result_t emit_bits_result = emit_bits(entropy, entropy->EOBRUN, nbits);
      if (emit_bits_result.is_err) {
        return emit_bits_result;
      }
    }

    entropy->EOBRUN = 0;

    /* Emit any buffered correction bits */
    void_result_t emit_buffered_bits_result = emit_buffered_bits(entropy, entropy->bit_buffer, entropy->BE);
    if (emit_buffered_bits_result.is_err) {
      return emit_buffered_bits_result;
    }
    entropy->BE = 0;
  }

  return OK_VOID;
}


/*
 * Emit a restart marker & resynchronize predictions.
 */

J_WARN_UNUSED_RESULT LOCAL(void_result_t)
emit_restart (phuff_entropy_ptr entropy, int restart_num)
{
  int ci;

  void_result_t emit_eobrun_result = emit_eobrun(entropy);
  if (emit_eobrun_result.is_err) {
    return emit_eobrun_result;
  }

  if (! entropy->gather_statistics) {
    void_result_t flush_bits_result = flush_bits(entropy);
    if (flush_bits_result.is_err) {
      return flush_bits_result;
    }
    emit_byte(entropy, 0xFF);
    emit_byte(entropy, JPEG_RST0 + restart_num);
  }

  if (entropy->cinfo->Ss == 0) {
    /* Re-initialize DC predictions to 0 */
    for (ci = 0; ci < entropy->cinfo->comps_in_scan; ci++)
      entropy->last_dc_val[ci] = 0;
  } else {
    /* Re-initialize all AC-related fields to 0 */
    entropy->EOBRUN = 0;
    entropy->BE = 0;
  }

  return OK_VOID;
}


/*
 * MCU encoding for DC initial scan (either spectral selection,
 * or first pass of successive approximation).
 */

J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t)
encode_mcu_DC_first (j_compress_ptr cinfo, JBLOCKROW *MCU_data)
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  phuff_entropy_ptr entropy = (phuff_entropy_ptr) lossyc->entropy_private;
  register int temp, temp2;
  register int nbits;
  int blkn, ci;
  int Al = cinfo->Al;
  JBLOCKROW block;
  jpeg_component_info * compptr;
  ISHIFT_TEMPS

  entropy->next_output_byte = cinfo->dest->next_output_byte;
  entropy->free_in_buffer = cinfo->dest->free_in_buffer;

  /* Emit restart marker if needed */
  if (cinfo->restart_interval)
    if (entropy->restarts_to_go == 0) {
      void_result_t emit_restart_result = emit_restart(entropy, entropy->next_restart_num);
      if (emit_restart_result.is_err) {
        return RESULT_ERR(boolean, emit_restart_result.err_code);
      }
    }

  /* Encode the MCU data blocks */
  for (blkn = 0; blkn < cinfo->data_units_in_MCU; blkn++) {
    block = MCU_data[blkn];
    ci = cinfo->MCU_membership[blkn];
    compptr = cinfo->cur_comp_info[ci];

    /* Compute the DC value after the required point transform by Al.
     * This is simply an arithmetic right shift.
     */
    temp2 = IRIGHT_SHIFT((int) ((*block)[0]), Al);

    /* DC differences are figured on the point-transformed values. */
    temp = temp2 - entropy->last_dc_val[ci];
    entropy->last_dc_val[ci] = temp2;

    /* Encode the DC coefficient difference per section G.1.2.1 */
    temp2 = temp;
    if (temp < 0) {
      temp = -temp;     /* temp is abs value of input */
      /* For a negative input, want temp2 = bitwise complement of abs(input) */
      /* This code assumes we are on a two's complement machine */
      temp2--;
    }
    
    /* Find the number of bits needed for the magnitude of the coefficient */
    nbits = 0;
    while (temp) {
      nbits++;
      temp >>= 1;
    }
    /* Check for out-of-range coefficient values.
     * Since we're encoding a difference, the range limit is twice as much.
     */
    if (nbits > MAX_COEF_BITS+1)
      ERREXIT(cinfo, JERR_BAD_DCT_COEF, ERR_BOOL);
    
    /* Count/emit the Huffman-coded symbol for the number of bits */
    void_result_t emit_symbol_result = emit_symbol(entropy, compptr->dc_tbl_no, nbits);
    if (emit_symbol_result.is_err) {
      return RESULT_ERR(boolean, emit_symbol_result.err_code);
    }
    
    /* Emit that number of bits of the value, if positive, */
    /* or the complement of its magnitude, if negative. */
    if (nbits)          /* emit_bits rejects calls with size 0 */
    {
      void_result_t emit_bits_result = emit_bits(entropy, (unsigned int) temp2, nbits);
      if (emit_bits_result.is_err) {
        return RESULT_ERR(boolean, emit_bits_result.err_code);
      }
    }
  }

  cinfo->dest->next_output_byte = entropy->next_output_byte;
  cinfo->dest->free_in_buffer = entropy->free_in_buffer;

  /* Update restart-interval state too */
  if (cinfo->restart_interval) {
    if (entropy->restarts_to_go == 0) {
      entropy->restarts_to_go = cinfo->restart_interval;
      entropy->next_restart_num++;
      entropy->next_restart_num &= 7;
    }
    entropy->restarts_to_go--;
  }

  return RESULT_OK(boolean, TRUE);
}


/*
 * MCU encoding for AC initial scan (either spectral selection,
 * or first pass of successive approximation).
 */

J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t)
encode_mcu_AC_first (j_compress_ptr cinfo, JBLOCKROW *MCU_data)
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  phuff_entropy_ptr entropy = (phuff_entropy_ptr) lossyc->entropy_private;
  register int temp, temp2;
  register int nbits;
  register int r, k;
  int Se = cinfo->Se;
  int Al = cinfo->Al;
  JBLOCKROW block;

  entropy->next_output_byte = cinfo->dest->next_output_byte;
  entropy->free_in_buffer = cinfo->dest->free_in_buffer;

  /* Emit restart marker if needed */
  if (cinfo->restart_interval)
    if (entropy->restarts_to_go == 0) {
      void_result_t emit_restart_result = emit_restart(entropy, entropy->next_restart_num);
      if (emit_restart_result.is_err) {
        return RESULT_ERR(boolean, emit_restart_result.err_code);
      }
    }

  /* Encode the MCU data block */
  block = MCU_data[0];

  /* Encode the AC coefficients per section G.1.2.2, fig. G.3 */
  
  r = 0;            /* r = run length of zeros */
   
  for (k = cinfo->Ss; k <= Se; k++) {
    if ((temp = (*block)[jpeg_natural_order[k]]) == 0) {
      r++;
      continue;
    }
    /* We must apply the point transform by Al.  For AC coefficients this
     * is an integer division with rounding towards 0.  To do this portably
     * in C, we shift after obtaining the absolute value; so the code is
     * interwoven with finding the abs value (temp) and output bits (temp2).
     */
    if (temp < 0) {
      temp = -temp;     /* temp is abs value of input */
      temp >>= Al;      /* apply the point transform */
      /* For a negative coef, want temp2 = bitwise complement of abs(coef) */
      temp2 = ~temp;
    } else {
      temp >>= Al;      /* apply the point transform */
      temp2 = temp;
    }
    /* Watch out for case that nonzero coef is zero after point transform */
    if (temp == 0) {
      r++;
      continue;
    }

    /* Emit any pending EOBRUN */
    if (entropy->EOBRUN > 0) {
      void_result_t emit_eobrun_result = emit_eobrun(entropy);
      if (emit_eobrun_result.is_err) {
        return RESULT_ERR(boolean, emit_eobrun_result.err_code);
      }
    }
    /* if run length > 15, must emit special run-length-16 codes (0xF0) */
    while (r > 15) {
      void_result_t emit_symbol_result = emit_symbol(entropy, entropy->ac_tbl_no, 0xF0);
      if (emit_symbol_result.is_err) {
        return RESULT_ERR(boolean, emit_symbol_result.err_code);
      }
      r -= 16;
    }

    /* Find the number of bits needed for the magnitude of the coefficient */
    nbits = 1;          /* there must be at least one 1 bit */
    while ((temp >>= 1))
      nbits++;
    /* Check for out-of-range coefficient values */
    if (nbits > MAX_COEF_BITS)
      ERREXIT(cinfo, JERR_BAD_DCT_COEF, ERR_BOOL);

    /* Count/emit Huffman symbol for run length / number of bits */
    void_result_t emit_symbol_result = emit_symbol(entropy, entropy->ac_tbl_no, (r << 4) + nbits);
    if (emit_symbol_result.is_err) {
      return RESULT_ERR(boolean, emit_symbol_result.err_code);
    }
    /* Emit that number of bits of the value, if positive, */
    /* or the complement of its magnitude, if negative. */
    void_result_t emit_bits_result = emit_bits(entropy, (unsigned int) temp2, nbits);
    if (emit_bits_result.is_err) {
      return RESULT_ERR(boolean, emit_bits_result.err_code);
    }

    r = 0;          /* reset zero run length */
  }

  if (r > 0) {          /* If there are trailing zeroes, */
    entropy->EOBRUN++;      /* count an EOB */
    if (entropy->EOBRUN == 0x7FFF) {
      void_result_t emit_eobrun_result = emit_eobrun(entropy); /* force it out to avoid overflow */
      if (emit_eobrun_result.is_err) {
        return RESULT_ERR(boolean, emit_eobrun_result.err_code);
      }
    }
  }

  cinfo->dest->next_output_byte = entropy->next_output_byte;
  cinfo->dest->free_in_buffer = entropy->free_in_buffer;

  /* Update restart-interval state too */
  if (cinfo->restart_interval) {
    if (entropy->restarts_to_go == 0) {
      entropy->restarts_to_go = cinfo->restart_interval;
      entropy->next_restart_num++;
      entropy->next_restart_num &= 7;
    }
    entropy->restarts_to_go--;
  }

  return RESULT_OK(boolean, TRUE);
}


/*
 * MCU encoding for DC successive approximation refinement scan.
 * Note: we assume such scans can be multi-component, although the spec
 * is not very clear on the point.
 */

J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t)
encode_mcu_DC_refine (j_compress_ptr cinfo, JBLOCKROW *MCU_data)
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  phuff_entropy_ptr entropy = (phuff_entropy_ptr) lossyc->entropy_private;
  register int temp;
  int blkn;
  int Al = cinfo->Al;
  JBLOCKROW block;

  entropy->next_output_byte = cinfo->dest->next_output_byte;
  entropy->free_in_buffer = cinfo->dest->free_in_buffer;

  /* Emit restart marker if needed */
  if (cinfo->restart_interval)
    if (entropy->restarts_to_go == 0) {
      void_result_t emit_restart_result = emit_restart(entropy, entropy->next_restart_num);
      if (emit_restart_result.is_err) {
        return RESULT_ERR(boolean, emit_restart_result.err_code);
      }
    }

  /* Encode the MCU data blocks */
  for (blkn = 0; blkn < cinfo->data_units_in_MCU; blkn++) {
    block = MCU_data[blkn];

    /* We simply emit the Al'th bit of the DC coefficient value. */
    temp = (*block)[0];
    void_result_t emit_bits_result = emit_bits(entropy, (unsigned int) (temp >> Al), 1);
    if (emit_bits_result.is_err) {
      return RESULT_ERR(boolean, emit_bits_result.err_code);
    }
  }

  cinfo->dest->next_output_byte = entropy->next_output_byte;
  cinfo->dest->free_in_buffer = entropy->free_in_buffer;

  /* Update restart-interval state too */
  if (cinfo->restart_interval) {
    if (entropy->restarts_to_go == 0) {
      entropy->restarts_to_go = cinfo->restart_interval;
      entropy->next_restart_num++;
      entropy->next_restart_num &= 7;
    }
    entropy->restarts_to_go--;
  }

  return RESULT_OK(boolean, TRUE);
}


/*
 * MCU encoding for AC successive approximation refinement scan.
 */

J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t)
encode_mcu_AC_refine (j_compress_ptr cinfo, JBLOCKROW *MCU_data)
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  phuff_entropy_ptr entropy = (phuff_entropy_ptr) lossyc->entropy_private;
  register int temp;
  register int r, k;
  int EOB;
  char *BR_buffer;
  unsigned int BR;
  int Se = cinfo->Se;
  int Al = cinfo->Al;
  JBLOCKROW block;
  int absvalues[DCTSIZE2];

  entropy->next_output_byte = cinfo->dest->next_output_byte;
  entropy->free_in_buffer = cinfo->dest->free_in_buffer;

  /* Emit restart marker if needed */
  if (cinfo->restart_interval)
    if (entropy->restarts_to_go == 0) {
      void_result_t emit_restart_result = emit_restart(entropy, entropy->next_restart_num);
      if (emit_restart_result.is_err) {
        return RESULT_ERR(boolean, emit_restart_result.err_code);
      }
    }

  /* Encode the MCU data block */
  block = MCU_data[0];

  /* It is convenient to make a pre-pass to determine the transformed
   * coefficients' absolute values and the EOB position.
   */
  EOB = 0;
  for (k = cinfo->Ss; k <= Se; k++) {
    temp = (*block)[jpeg_natural_order[k]];
    /* We must apply the point transform by Al.  For AC coefficients this
     * is an integer division with rounding towards 0.  To do this portably
     * in C, we shift after obtaining the absolute value.
     */
    if (temp < 0)
      temp = -temp;     /* temp is abs value of input */
    temp >>= Al;        /* apply the point transform */
    absvalues[k] = temp;    /* save abs value for main pass */
    if (temp == 1)
      EOB = k;          /* EOB = index of last newly-nonzero coef */
  }

  /* Encode the AC coefficients per section G.1.2.3, fig. G.7 */
  
  r = 0;            /* r = run length of zeros */
  BR = 0;           /* BR = count of buffered bits added now */
  BR_buffer = entropy->bit_buffer + entropy->BE; /* Append bits to buffer */

  for (k = cinfo->Ss; k <= Se; k++) {
    if ((temp = absvalues[k]) == 0) {
      r++;
      continue;
    }

    /* Emit any required ZRLs, but not if they can be folded into EOB */
    while (r > 15 && k <= EOB) {
      /* emit any pending EOBRUN and the BE correction bits */
      void_result_t emit_eobrun_result = emit_eobrun(entropy);
      if (emit_eobrun_result.is_err) {
        return RESULT_ERR(boolean, emit_eobrun_result.err_code);
      }
      /* Emit ZRL */
      void_result_t emit_symbol_result = emit_symbol(entropy, entropy->ac_tbl_no, 0xF0);
      if (emit_symbol_result.is_err) {
        return RESULT_ERR(boolean, emit_symbol_result.err_code);
      }
      r -= 16;
      /* Emit buffered correction bits that must be associated with ZRL */
      void_result_t emit_buffered_bits_result = emit_buffered_bits(entropy, BR_buffer, BR);
      if (emit_buffered_bits_result.is_err) {
        return RESULT_ERR(boolean, emit_buffered_bits_result.err_code);
      }
      BR_buffer = entropy->bit_buffer; /* BE bits are gone now */
      BR = 0;
    }

    /* If the coef was previously nonzero, it only needs a correction bit.
     * NOTE: a straight translation of the spec's figure G.7 would suggest
     * that we also need to test r > 15.  But if r > 15, we can only get here
     * if k > EOB, which implies that this coefficient is not 1.
     */
    if (temp > 1) {
      /* The correction bit is the next bit of the absolute value. */
      BR_buffer[BR++] = (char) (temp & 1);
      continue;
    }

    /* Emit any pending EOBRUN and the BE correction bits */
    void_result_t emit_eobrun_result = emit_eobrun(entropy);
    if (emit_eobrun_result.is_err) {
      return RESULT_ERR(boolean, emit_eobrun_result.err_code);
    }

    /* Count/emit Huffman symbol for run length / number of bits */
    void_result_t emit_symbol_result = emit_symbol(entropy, entropy->ac_tbl_no, (r << 4) + 1);
    if (emit_symbol_result.is_err) {
      return RESULT_ERR(boolean, emit_symbol_result.err_code);
    }

    /* Emit output bit for newly-nonzero coef */
    temp = ((*block)[jpeg_natural_order[k]] < 0) ? 0 : 1;
    void_result_t emit_bits_result = emit_bits(entropy, (unsigned int) temp, 1);
    if (emit_bits_result.is_err) {
      return RESULT_ERR(boolean, emit_bits_result.err_code);
    }

    /* Emit buffered correction bits that must be associated with this code */
    void_result_t emit_buffered_bits_result = emit_buffered_bits(entropy, BR_buffer, BR);
    if (emit_buffered_bits_result.is_err) {
      return RESULT_ERR(boolean, emit_buffered_bits_result.err_code);
    }
    BR_buffer = entropy->bit_buffer; /* BE bits are gone now */
    BR = 0;
    r = 0;          /* reset zero run length */
  }

  if (r > 0 || BR > 0) {    /* If there are trailing zeroes, */
    entropy->EOBRUN++;      /* count an EOB */
    entropy->BE += BR;      /* concat my correction bits to older ones */
    /* We force out the EOB if we risk either:
     * 1. overflow of the EOB counter;
     * 2. overflow of the correction bit buffer during the next MCU.
     */
    if (entropy->EOBRUN == 0x7FFF || entropy->BE > (MAX_CORR_BITS-DCTSIZE2+1)) {
      void_result_t emit_eobrun_result = emit_eobrun(entropy);
      if (emit_eobrun_result.is_err) {
        return RESULT_ERR(boolean, emit_eobrun_result.err_code);
      }
    }
  }

  cinfo->dest->next_output_byte = entropy->next_output_byte;
  cinfo->dest->free_in_buffer = entropy->free_in_buffer;

  /* Update restart-interval state too */
  if (cinfo->restart_interval) {
    if (entropy->restarts_to_go == 0) {
      entropy->restarts_to_go = cinfo->restart_interval;
      entropy->next_restart_num++;
      entropy->next_restart_num &= 7;
    }
    entropy->restarts_to_go--;
  }

  return RESULT_OK(boolean, TRUE);
}


/*
 * Finish up at the end of a Huffman-compressed progressive scan.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
finish_pass_phuff (j_compress_ptr cinfo)
{   
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  phuff_entropy_ptr entropy = (phuff_entropy_ptr) lossyc->entropy_private;

  entropy->next_output_byte = cinfo->dest->next_output_byte;
  entropy->free_in_buffer = cinfo->dest->free_in_buffer;

  /* Flush out any buffered data */
  void_result_t emit_eobrun_result = emit_eobrun(entropy);
  if (emit_eobrun_result.is_err) {
    return emit_eobrun_result;
  }
  void_result_t flush_bits_result = flush_bits(entropy);
  if (flush_bits_result.is_err) {
    return flush_bits_result;
  }

  cinfo->dest->next_output_byte = entropy->next_output_byte;
  cinfo->dest->free_in_buffer = entropy->free_in_buffer;

  return OK_VOID;
}


/*
 * Finish up a statistics-gathering pass and create the new Huffman tables.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
finish_pass_gather_phuff (j_compress_ptr cinfo)
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  phuff_entropy_ptr entropy = (phuff_entropy_ptr) lossyc->entropy_private;
  boolean is_DC_band;
  int ci, tbl;
  jpeg_component_info * compptr;
  JHUFF_TBL **htblptr;
  boolean did[NUM_HUFF_TBLS];

  /* Flush out buffered data (all we care about is counting the EOB symbol) */
  void_result_t emit_eobrun_result = emit_eobrun(entropy);
  if (emit_eobrun_result.is_err) {
    return emit_eobrun_result;
  }

  is_DC_band = (cinfo->Ss == 0);

  /* It's important not to apply jpeg_gen_optimal_table more than once
   * per table, because it clobbers the input frequency counts!
   */
  MEMZERO(did, SIZEOF(did));

  for (ci = 0; ci < cinfo->comps_in_scan; ci++) {
    compptr = cinfo->cur_comp_info[ci];
    if (is_DC_band) {
      if (cinfo->Ah != 0)   /* DC refinement needs no table */
    continue;
      tbl = compptr->dc_tbl_no;
    } else {
      tbl = compptr->ac_tbl_no;
    }
    if (! did[tbl]) {
      if (is_DC_band)
        htblptr = & cinfo->dc_huff_tbl_ptrs[tbl];
      else
        htblptr = & cinfo->ac_huff_tbl_ptrs[tbl];
      if (*htblptr == NULL) {
        jhuff_tbl_ptr_result_t jpeg_alloc_huff_table_result = jpeg_alloc_huff_table((j_common_ptr) cinfo);
        if (jpeg_alloc_huff_table_result.is_err) {
          return ERR_VOID(jpeg_alloc_huff_table_result.err_code);
        }
        *htblptr = jpeg_alloc_huff_table_result.value;
      }
      void_result_t jpeg_gen_optimal_table_result = jpeg_gen_optimal_table(cinfo, *htblptr, entropy->count_ptrs[tbl]);
      if (jpeg_gen_optimal_table_result.is_err) {
        return jpeg_gen_optimal_table_result;
      }
      did[tbl] = TRUE;
    }
  }

  return OK_VOID;
}


J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t)
need_optimization_pass (j_compress_ptr cinfo)
{
  return RESULT_OK(boolean, (cinfo->Ss != 0 || cinfo->Ah == 0));
}


/*
 * Module initialization routine for progressive Huffman entropy encoding.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_phuff_encoder (j_compress_ptr cinfo)
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  phuff_entropy_ptr entropy;
  int i;

  void_ptr_result_t alloc_small_result =
    (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
                SIZEOF(phuff_entropy_encoder));
  if (alloc_small_result.is_err) {
    return ERR_VOID(alloc_small_result.err_code);
  }
  entropy = (phuff_entropy_ptr) alloc_small_result.value;
  lossyc->entropy_private = (void *) entropy;
  lossyc->pub.entropy_start_pass = start_pass_phuff;
  lossyc->pub.need_optimization_pass = need_optimization_pass;

  /* Mark tables unallocated */
  for (i = 0; i < NUM_HUFF_TBLS; i++) {
    entropy->derived_tbls[i] = NULL;
    entropy->count_ptrs[i] = NULL;
  }
  entropy->bit_buffer = NULL;   /* needed only in AC refinement scan */

  return OK_VOID;
}

#endif /* C_PROGRESSIVE_SUPPORTED */
