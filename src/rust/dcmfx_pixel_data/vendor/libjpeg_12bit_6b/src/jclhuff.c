/*
 * jclhuff.c
 *
 * Copyright (C) 1991-1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains Huffman entropy encoding routines for lossless JPEG.
 *
 * Much of the complexity here has to do with supporting output suspension.
 * If the data destination module demands suspension, we want to be able to
 * back up to the start of the current MCU.  To do this, we copy state
 * variables into local working storage, and update them back to the
 * permanent JPEG objects only upon successful completion of an MCU.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"
#include "jlossls12.h"      /* Private declarations for lossless codec */
#include "jchuff12.h"       /* Declarations shared with jc*huff.c */


/* Expanded entropy encoder object for Huffman encoding.
 *
 * The savable_state subrecord contains fields that change within an MCU,
 * but must not be updated permanently until we complete the MCU.
 */

typedef struct {
  IJG_INT32 put_buffer;     /* current bit-accumulation buffer */
  int put_bits;         /* # of bits now in it */
} savable_state;

/* This macro is to work around compilers with missing or broken
 * structure assignment.  You'll need to fix this code if you have
 * such a compiler and you change MAX_COMPS_IN_SCAN.
 */

#ifndef NO_STRUCT_ASSIGN
#define ASSIGN_STATE(dest,src)  ((dest) = (src))
#else
#define ASSIGN_STATE(dest,src)  \
    ((dest).put_buffer = (src).put_buffer, \
     (dest).put_bits = (src).put_bits)
#endif


typedef struct {
  int ci, yoffset, MCU_width;
} lhe_input_ptr_info;


typedef struct {
  savable_state saved;      /* Bit buffer at start of MCU */

  /* These fields are NOT loaded into local working state. */
  unsigned int restarts_to_go;  /* MCUs left in this restart interval */
  int next_restart_num;     /* next restart number to write (0-7) */

  /* Pointers to derived tables (these workspaces have image lifespan) */
  c_derived_tbl * derived_tbls[NUM_HUFF_TBLS];

  /* Pointers to derived tables to be used for each data unit within an MCU */
  c_derived_tbl * cur_tbls[C_MAX_DATA_UNITS_IN_MCU];

#ifdef ENTROPY_OPT_SUPPORTED    /* Statistics tables for optimization */
  long * count_ptrs[NUM_HUFF_TBLS];

  /* Pointers to stats tables to be used for each data unit within an MCU */
  long * cur_counts[C_MAX_DATA_UNITS_IN_MCU];
#endif

  /* Pointers to the proper input difference row for each group of data units
   * within an MCU.  For each component, there are Vi groups of Hi data units.
   */
  JDIFFROW input_ptr[C_MAX_DATA_UNITS_IN_MCU];

  /* Number of input pointers in use for the current MCU.  This is the sum
   * of all Vi in the MCU.
   */
  int num_input_ptrs;

  /* Information used for positioning the input pointers within the input
   * difference rows.
   */
  lhe_input_ptr_info input_ptr_info[C_MAX_DATA_UNITS_IN_MCU];

  /* Index of the proper input pointer for each data unit within an MCU */
  int input_ptr_index[C_MAX_DATA_UNITS_IN_MCU];

} lhuff_entropy_encoder;

typedef lhuff_entropy_encoder * lhuff_entropy_ptr;

/* Working state while writing an MCU.
 * This struct contains all the fields that are needed by subroutines.
 */

typedef struct {
  JOCTET * next_output_byte;    /* => next byte to write in buffer */
  size_t free_in_buffer;    /* # of byte spaces remaining in buffer */
  savable_state cur;        /* Current bit buffer & DC state */
  j_compress_ptr cinfo;     /* dump_buffer needs access to this */
} working_state;


/* Forward declarations */
J_WARN_UNUSED_RESULT METHODDEF(jdimension_result_t) encode_mcus_huff (j_compress_ptr cinfo,
                    JDIFFIMAGE diff_buf,
                    JDIMENSION MCU_row_num,
                    JDIMENSION MCU_col_num,
                    JDIMENSION nMCU);
J_WARN_UNUSED_RESULT METHODDEF(void_result_t) finish_pass_huff JPP((j_compress_ptr cinfo));
#ifdef ENTROPY_OPT_SUPPORTED
J_WARN_UNUSED_RESULT METHODDEF(jdimension_result_t) encode_mcus_gather (j_compress_ptr cinfo,
                      JDIFFIMAGE diff_buf,
                      JDIMENSION MCU_row_num,
                      JDIMENSION MCU_col_num,
                      JDIMENSION nMCU);
J_WARN_UNUSED_RESULT METHODDEF(void_result_t) finish_pass_gather JPP((j_compress_ptr cinfo));
#endif


/*
 * Initialize for a Huffman-compressed scan.
 * If gather_statistics is TRUE, we do not output anything during the scan,
 * just count the Huffman symbols used and generate Huffman code tables.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
start_pass_huff (j_compress_ptr cinfo, boolean gather_statistics)
{
  j_lossless_c_ptr losslsc = (j_lossless_c_ptr) cinfo->codec;
  lhuff_entropy_ptr entropy = (lhuff_entropy_ptr) losslsc->entropy_private;
  int ci, dctbl, sampn, ptrn, yoffset, xoffset;
  jpeg_component_info * compptr;

  if (gather_statistics) {
#ifdef ENTROPY_OPT_SUPPORTED
    losslsc->entropy_encode_mcus = encode_mcus_gather;
    losslsc->pub.entropy_finish_pass = finish_pass_gather;
#else
    ERREXIT(cinfo, JERR_NOT_COMPILED);
#endif
  } else {
    losslsc->entropy_encode_mcus = encode_mcus_huff;
    losslsc->pub.entropy_finish_pass = finish_pass_huff;
  }

  for (ci = 0; ci < cinfo->comps_in_scan; ci++) {
    compptr = cinfo->cur_comp_info[ci];
    dctbl = compptr->dc_tbl_no;
    if (gather_statistics) {
#ifdef ENTROPY_OPT_SUPPORTED
      /* Check for invalid table indexes */
      /* (make_c_derived_tbl does this in the other path) */
      if (dctbl < 0 || dctbl >= NUM_HUFF_TBLS)
    ERREXIT1(cinfo, JERR_NO_HUFF_TABLE, dctbl, ERR_VOID);
      /* Allocate and zero the statistics tables */
      /* Note that jpeg_gen_optimal_table expects 257 entries in each table! */
      if (entropy->count_ptrs[dctbl] == NULL) {
        void_ptr_result_t alloc_small_result = 
          (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
                      257 * SIZEOF(long));
        if (alloc_small_result.is_err) {
          return ERR_VOID(alloc_small_result.err_code);
        }
        entropy->count_ptrs[dctbl] = (long *) alloc_small_result.value;
      }
      MEMZERO(entropy->count_ptrs[dctbl], 257 * SIZEOF(long));
#endif
    } else {
      /* Compute derived values for Huffman tables */
      /* We may do this more than once for a table, but it's not expensive */
      void_result_t jpeg_make_c_derived_tbl_result = jpeg_make_c_derived_tbl(cinfo, TRUE, dctbl,
                  & entropy->derived_tbls[dctbl]);
      if (jpeg_make_c_derived_tbl_result.is_err) {
        return jpeg_make_c_derived_tbl_result;
      }
    }
  }

  /* Precalculate encoding info for each sample in an MCU of this scan */
  for (sampn = 0, ptrn = 0; sampn < cinfo->data_units_in_MCU;) {
    compptr = cinfo->cur_comp_info[cinfo->MCU_membership[sampn]];
    ci = compptr->component_index;
    /*    ci = cinfo->MCU_membership[sampn];
    compptr = cinfo->cur_comp_info[ci];*/
    for (yoffset = 0; yoffset < compptr->MCU_height; yoffset++, ptrn++) {
      /* Precalculate the setup info for each input pointer */
      entropy->input_ptr_info[ptrn].ci = ci;
      entropy->input_ptr_info[ptrn].yoffset = yoffset;
      entropy->input_ptr_info[ptrn].MCU_width = compptr->MCU_width;
      for (xoffset = 0; xoffset < compptr->MCU_width; xoffset++, sampn++) {
    /* Precalculate the input pointer index for each sample */
    entropy->input_ptr_index[sampn] = ptrn;
    /* Precalculate which tables to use for each sample */
    entropy->cur_tbls[sampn] = entropy->derived_tbls[compptr->dc_tbl_no];
    entropy->cur_counts[sampn] = entropy->count_ptrs[compptr->dc_tbl_no];
      }
    }
  }
  entropy->num_input_ptrs = ptrn;

  /* Initialize bit buffer to empty */
  entropy->saved.put_buffer = 0;
  entropy->saved.put_bits = 0;

  /* Initialize restart stuff */
  entropy->restarts_to_go = cinfo->restart_interval;
  entropy->next_restart_num = 0;

  return OK_VOID;
}


/* Outputting bytes to the file */

/* Emit a byte, taking 'action' if must suspend. */
#define emit_byte(state,val,action)  \
    { *(state)->next_output_byte++ = (JOCTET) (val);  \
      if (--(state)->free_in_buffer == 0)  \
      { \
        boolean_result_t dump_buffer_result = dump_buffer(state); \
        if (dump_buffer_result.is_err) \
          return dump_buffer_result; \
        if (! dump_buffer_result.value)  \
          { action; } \
      } }


J_WARN_UNUSED_RESULT LOCAL(boolean_result_t)
dump_buffer (working_state * state)
/* Empty the output buffer; return TRUE if successful, FALSE if must suspend */
{
  struct jpeg_destination_mgr * dest = state->cinfo->dest;

  boolean_result_t empty_output_buffer_result = (*dest->empty_output_buffer) (state->cinfo);
  if (empty_output_buffer_result.is_err) {
    return empty_output_buffer_result;
  }
  if (! empty_output_buffer_result.value)
    return RESULT_OK(boolean, FALSE);
  /* After a successful buffer dump, must reset buffer pointers */
  state->next_output_byte = dest->next_output_byte;
  state->free_in_buffer = dest->free_in_buffer;
  return RESULT_OK(boolean, TRUE);
}


/* Outputting bits to the file */

/* Only the right 24 bits of put_buffer are used; the valid bits are
 * left-justified in this part.  At most 16 bits can be passed to emit_bits
 * in one call, and we never retain more than 7 bits in put_buffer
 * between calls, so 24 bits are sufficient.
 */

J_WARN_UNUSED_RESULT INLINE
LOCAL(boolean_result_t)
emit_bits (working_state * state, unsigned int code, int size)
/* Emit some bits; return TRUE if successful, FALSE if must suspend */
{
  /* This routine is heavily used, so it's worth coding tightly. */
  register IJG_INT32 put_buffer = (IJG_INT32) code;
  register int put_bits = state->cur.put_bits;

  /* if size is 0, caller used an invalid Huffman table entry */
  if (size == 0)
    ERREXIT(state->cinfo, JERR_HUFF_MISSING_CODE, ERR_BOOL);

  put_buffer &= (((IJG_INT32) 1)<<size) - 1; /* mask off any extra bits in code */
  
  put_bits += size;     /* new number of bits in buffer */
  
  put_buffer <<= 24 - put_bits; /* align incoming bits */

  put_buffer |= state->cur.put_buffer; /* and merge with old buffer contents */
  
  while (put_bits >= 8) {
    int c = (int) ((put_buffer >> 16) & 0xFF);
    
    emit_byte(state, c, return RESULT_OK(boolean, FALSE));
    if (c == 0xFF) {        /* need to stuff a zero byte? */
      emit_byte(state, 0, return RESULT_OK(boolean, FALSE));
    }
    put_buffer <<= 8;
    put_bits -= 8;
  }

  state->cur.put_buffer = put_buffer; /* update state variables */
  state->cur.put_bits = put_bits;

  return RESULT_OK(boolean, TRUE);
}


J_WARN_UNUSED_RESULT LOCAL(boolean_result_t)
flush_bits (working_state * state)
{
  boolean_result_t emit_bits_result = emit_bits(state, 0x7F, 7);
  if (emit_bits_result.is_err) {
    return emit_bits_result;
  }
  if (! emit_bits_result.value) /* fill any partial byte with ones */
    return RESULT_OK(boolean, FALSE);
  state->cur.put_buffer = 0;    /* and reset bit-buffer to empty */
  state->cur.put_bits = 0;
  return RESULT_OK(boolean, TRUE);
}


/*
 * Emit a restart marker & resynchronize predictions.
 */

J_WARN_UNUSED_RESULT LOCAL(boolean_result_t)
emit_restart (working_state * state, int restart_num)
{
  /* int ci; */

  boolean_result_t flush_bits_result = flush_bits(state);
  if (flush_bits_result.is_err) {
    return RESULT_OK(boolean, flush_bits_result.err_code);
  }
  if (! flush_bits_result.value)
    return RESULT_OK(boolean, FALSE);

  emit_byte(state, 0xFF, return RESULT_OK(boolean, FALSE));
  emit_byte(state, JPEG_RST0 + restart_num, return RESULT_OK(boolean, FALSE));

  /* The restart counter is not updated until we successfully write the MCU. */

  return RESULT_OK(boolean, TRUE);
}


/*
 * Encode and output one nMCU's worth of Huffman-compressed differences.
 */

J_WARN_UNUSED_RESULT METHODDEF(jdimension_result_t)
encode_mcus_huff (j_compress_ptr cinfo, JDIFFIMAGE diff_buf,
          JDIMENSION MCU_row_num, JDIMENSION MCU_col_num,
          JDIMENSION nMCU)
{
  j_lossless_c_ptr losslsc = (j_lossless_c_ptr) cinfo->codec;
  lhuff_entropy_ptr entropy = (lhuff_entropy_ptr) losslsc->entropy_private;
  working_state state;
  unsigned int mcu_num;
  int sampn, ci, yoffset, MCU_width, ptrn;
  /* jpeg_component_info * compptr; */

  /* Load up working state */
  state.next_output_byte = cinfo->dest->next_output_byte;
  state.free_in_buffer = cinfo->dest->free_in_buffer;
  ASSIGN_STATE(state.cur, entropy->saved);
  state.cinfo = cinfo;

  /* Emit restart marker if needed */
  if (cinfo->restart_interval) {
    if (entropy->restarts_to_go == 0) {
      boolean_result_t emit_restart_result = emit_restart(&state, entropy->next_restart_num);
      if (emit_restart_result.is_err) {
        return RESULT_ERR(jdimension, emit_restart_result.err_code);
      }
      if (!emit_restart_result.value)
        return RESULT_OK(jdimension, 0);
    }
  }

  /* Set input pointer locations based on MCU_col_num */
  for (ptrn = 0; ptrn < entropy->num_input_ptrs; ptrn++) {
    ci = entropy->input_ptr_info[ptrn].ci;
    yoffset = entropy->input_ptr_info[ptrn].yoffset;
    MCU_width = entropy->input_ptr_info[ptrn].MCU_width;
    entropy->input_ptr[ptrn] =
      diff_buf[ci][MCU_row_num + (size_t)yoffset] + MCU_col_num * (size_t)MCU_width;
  }

  for (mcu_num = 0; mcu_num < nMCU; mcu_num++) {

    /* Inner loop handles the samples in the MCU */
    for (sampn = 0; sampn < cinfo->data_units_in_MCU; sampn++) {
      register int temp, temp2 /* , temp3 */ ;
      register int nbits;
      c_derived_tbl *dctbl = entropy->cur_tbls[sampn];
  
      /* Encode the difference per section H.1.2.2 */
  
      /* Input the sample difference */
      temp = *entropy->input_ptr[entropy->input_ptr_index[sampn]]++;

      if (temp & 0x8000) {  /* instead of temp < 0 */
    temp = (-temp) & 0x7FFF; /* absolute value, mod 2^16 */
    if (temp == 0)      /* special case: magnitude = 32768 */
      temp2 = temp = 0x8000;
    temp2 = ~ temp;     /* one's complement of magnitude */
      } else {
    temp &= 0x7FFF;     /* abs value mod 2^16 */
    temp2 = temp;       /* magnitude */
      }

      /* Find the number of bits needed for the magnitude of the difference */
      nbits = 0;
      while (temp) {
    nbits++;
    temp >>= 1;
      }
      /* Check for out-of-range difference values.
       */
      if (nbits > MAX_DIFF_BITS)
    ERREXIT(cinfo, JERR_BAD_DIFF, ERR_JDIMENSION);
  
      /* Emit the Huffman-coded symbol for the number of bits */
      boolean_result_t emit_bits_result = emit_bits(&state, dctbl->ehufco[nbits], dctbl->ehufsi[nbits]);
      if (emit_bits_result.is_err) {
        return RESULT_ERR(jdimension, emit_bits_result.err_code);
      }
      if (! emit_bits_result.value)
    return RESULT_OK(jdimension, mcu_num);

      /* Emit that number of bits of the value, if positive, */
      /* or the complement of its magnitude, if negative. */
      if (nbits &&      /* emit_bits rejects calls with size 0 */
      nbits != 16)      /* special case: no bits should be emitted */
      {
        emit_bits_result = emit_bits(&state, (unsigned int) temp2, nbits);
        if (emit_bits_result.is_err) {
          return RESULT_ERR(jdimension, emit_bits_result.err_code);
        }
    if (! emit_bits_result.value)
      return RESULT_OK(jdimension, mcu_num);
      }
    }

    /* Completed MCU, so update state */
    cinfo->dest->next_output_byte = state.next_output_byte;
    cinfo->dest->free_in_buffer = state.free_in_buffer;
    ASSIGN_STATE(entropy->saved, state.cur);

    /* Update restart-interval state too */
    if (cinfo->restart_interval) {
      if (entropy->restarts_to_go == 0) {
    entropy->restarts_to_go = cinfo->restart_interval;
    entropy->next_restart_num++;
    entropy->next_restart_num &= 7;
      }
      entropy->restarts_to_go--;
    }

  }

  return RESULT_OK(jdimension, nMCU);
}


/*
 * Finish up at the end of a Huffman-compressed scan.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
finish_pass_huff (j_compress_ptr cinfo)
{
  j_lossless_c_ptr losslsc = (j_lossless_c_ptr) cinfo->codec;
  lhuff_entropy_ptr entropy = (lhuff_entropy_ptr) losslsc->entropy_private;
  working_state state;

  /* Load up working state ... flush_bits needs it */
  state.next_output_byte = cinfo->dest->next_output_byte;
  state.free_in_buffer = cinfo->dest->free_in_buffer;
  ASSIGN_STATE(state.cur, entropy->saved);
  state.cinfo = cinfo;

  /* Flush out the last data */
  boolean_result_t flush_bits_result = flush_bits(&state);
  if (flush_bits_result.is_err) {
    return ERR_VOID(flush_bits_result.err_code);
  }
  if (! flush_bits_result.value)
    ERREXIT(cinfo, JERR_CANT_SUSPEND, ERR_VOID);

  /* Update state */
  cinfo->dest->next_output_byte = state.next_output_byte;
  cinfo->dest->free_in_buffer = state.free_in_buffer;
  ASSIGN_STATE(entropy->saved, state.cur);

  return OK_VOID;
}


/*
 * Huffman coding optimization.
 *
 * We first scan the supplied data and count the number of uses of each symbol
 * that is to be Huffman-coded. (This process MUST agree with the code above.)
 * Then we build a Huffman coding tree for the observed counts.
 * Symbols which are not needed at all for the particular image are not
 * assigned any code, which saves space in the DHT marker as well as in
 * the compressed data.
 */

#ifdef ENTROPY_OPT_SUPPORTED

/*
 * Trial-encode one nMCU's worth of Huffman-compressed differences.
 * No data is actually output, so no suspension return is possible.
 */

J_WARN_UNUSED_RESULT METHODDEF(jdimension_result_t)
encode_mcus_gather (j_compress_ptr cinfo, JDIFFIMAGE diff_buf,
            JDIMENSION MCU_row_num, JDIMENSION MCU_col_num,
            JDIMENSION nMCU)
{
  j_lossless_c_ptr losslsc = (j_lossless_c_ptr) cinfo->codec;
  lhuff_entropy_ptr entropy = (lhuff_entropy_ptr) losslsc->entropy_private;
  unsigned int mcu_num; 
  int sampn, ci, yoffset, MCU_width, ptrn;
  /* jpeg_component_info * compptr; */

  /* Take care of restart intervals if needed */
  if (cinfo->restart_interval) {
    if (entropy->restarts_to_go == 0) {
      /* Update restart state */
      entropy->restarts_to_go = cinfo->restart_interval;
    }
    entropy->restarts_to_go--;
  }

  /* Set input pointer locations based on MCU_col_num */
  for (ptrn = 0; ptrn < entropy->num_input_ptrs; ptrn++) {
    ci = entropy->input_ptr_info[ptrn].ci;
    yoffset = entropy->input_ptr_info[ptrn].yoffset;
    MCU_width = entropy->input_ptr_info[ptrn].MCU_width;
    entropy->input_ptr[ptrn] =
      diff_buf[ci][MCU_row_num + (size_t)yoffset] + MCU_col_num * (size_t)MCU_width;
  }

  for (mcu_num = 0; mcu_num < nMCU; mcu_num++) {

    /* Inner loop handles the samples in the MCU */
    for (sampn = 0; sampn < cinfo->data_units_in_MCU; sampn++) {
      register int temp;
      register int nbits;
      /* c_derived_tbl *dctbl = entropy->cur_tbls[sampn]; */
      long * counts = entropy->cur_counts[sampn];
  
      /* Encode the difference per section H.1.2.2 */
  
      /* Input the sample difference */
      temp = *entropy->input_ptr[entropy->input_ptr_index[sampn]]++;

      if (temp & 0x8000) {  /* instead of temp < 0 */
    temp = (-temp) & 0x7FFF; /* absolute value, mod 2^16 */
    if (temp == 0)      /* special case: magnitude = 32768 */
      temp = 0x8000;
      } else
    temp &= 0x7FFF;     /* abs value mod 2^16 */

      /* Find the number of bits needed for the magnitude of the difference */
      nbits = 0;
      while (temp) {
    nbits++;
    temp >>= 1;
      }
      /* Check for out-of-range difference values.
       */
      if (nbits > MAX_DIFF_BITS)
    ERREXIT(cinfo, JERR_BAD_DIFF, ERR_JDIMENSION);
  
      /* Count the Huffman symbol for the number of bits */
      counts[nbits]++;
    }
  }

  return RESULT_OK(jdimension, nMCU);
}


/*
 * Finish up a statistics-gathering pass and create the new Huffman tables.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
finish_pass_gather (j_compress_ptr cinfo)
{
  j_lossless_c_ptr losslsc = (j_lossless_c_ptr) cinfo->codec;
  lhuff_entropy_ptr entropy = (lhuff_entropy_ptr) losslsc->entropy_private;
  int ci, dctbl;
  jpeg_component_info * compptr;
  JHUFF_TBL **htblptr;
  boolean did_dc[NUM_HUFF_TBLS];

  /* It's important not to apply jpeg_gen_optimal_table more than once
   * per table, because it clobbers the input frequency counts!
   */
  MEMZERO(did_dc, SIZEOF(did_dc));

  for (ci = 0; ci < cinfo->comps_in_scan; ci++) {
    compptr = cinfo->cur_comp_info[ci];
    dctbl = compptr->dc_tbl_no;
    if (! did_dc[dctbl]) {
      htblptr = & cinfo->dc_huff_tbl_ptrs[dctbl];
      if (*htblptr == NULL) {
        jhuff_tbl_ptr_result_t htblptr_result = jpeg_alloc_huff_table((j_common_ptr) cinfo);
        if (htblptr_result.is_err) {
          return ERR_VOID(htblptr_result.err_code);
        }
        *htblptr = htblptr_result.value;
      }
      void_result_t jpeg_gen_optimal_table_result = jpeg_gen_optimal_table(cinfo, *htblptr, entropy->count_ptrs[dctbl]);
      if (jpeg_gen_optimal_table_result.is_err) {
        return jpeg_gen_optimal_table_result;
      }
      did_dc[dctbl] = TRUE;
    }
  }

  return OK_VOID;
}


#endif /* ENTROPY_OPT_SUPPORTED */


J_WARN_UNUSED_RESULT METHODDEF(boolean_result_t)
need_optimization_pass (j_compress_ptr cinfo)
{
  (void)cinfo;
  return RESULT_OK(boolean, TRUE);
}


/*
 * Module initialization routine for Huffman entropy encoding.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_lhuff_encoder (j_compress_ptr cinfo)
{
  j_lossless_c_ptr losslsc = (j_lossless_c_ptr) cinfo->codec;
  lhuff_entropy_ptr entropy;
  int i;

  void_ptr_result_t alloc_small_result =
    (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
                SIZEOF(lhuff_entropy_encoder));
  if (alloc_small_result.is_err) {
    return ERR_VOID(alloc_small_result.err_code);
  }
  entropy = (lhuff_entropy_ptr) alloc_small_result.value;
  losslsc->entropy_private = (void *) entropy;
  losslsc->pub.entropy_start_pass = start_pass_huff;
  losslsc->pub.need_optimization_pass = need_optimization_pass;

  /* Mark tables unallocated */
  for (i = 0; i < NUM_HUFF_TBLS; i++) {
    entropy->derived_tbls[i] = NULL;
#ifdef ENTROPY_OPT_SUPPORTED
    entropy->count_ptrs[i] = NULL;
#endif
  }

  return OK_VOID;
}
