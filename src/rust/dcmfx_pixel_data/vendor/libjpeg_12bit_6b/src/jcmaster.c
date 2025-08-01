/*
 * jcmaster.c
 *
 * Copyright (C) 1991-1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains master control logic for the JPEG compressor.
 * These routines are concerned with parameter validation, initial setup,
 * and inter-pass control (determining the number of passes and the work 
 * to be done in each pass).
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"
#include "jlossy12.h"       /* Private declarations for lossy codec */


/* Private state */

typedef enum {
    main_pass,      /* input data, also do first output step */
    huff_opt_pass,      /* Huffman code optimization pass */
    output_pass     /* data output pass */
} c_pass_type;

typedef struct {
  struct jpeg_comp_master pub;  /* public fields */

  c_pass_type pass_type;    /* the type of the current pass */

  int pass_number;      /* # of passes completed */
  int total_passes;     /* total # of passes needed */

  int scan_number;      /* current index in scan_info[] */
} my_comp_master;

typedef my_comp_master * my_master_ptr;


/*
 * Support routines that do various essential calculations.
 */

J_WARN_UNUSED_RESULT LOCAL(void_result_t)
initial_setup (j_compress_ptr cinfo)
/* Do computations that are needed before master selection phase */
{
  int ci;
  jpeg_component_info *compptr;
  long samplesperrow;
  JDIMENSION jd_samplesperrow;
  int data_unit = cinfo->data_unit;

  /* Sanity check on image dimensions */
  if (cinfo->image_height <= 0 || cinfo->image_width <= 0
      || cinfo->num_components <= 0 || cinfo->input_components <= 0)
    ERREXIT(cinfo, JERR_EMPTY_IMAGE, ERR_VOID);

  /* Make sure image isn't bigger than I can handle */
  if ((long) cinfo->image_height > (long) JPEG_MAX_DIMENSION ||
      (long) cinfo->image_width > (long) JPEG_MAX_DIMENSION)
    ERREXIT1(cinfo, JERR_IMAGE_TOO_BIG, (unsigned int) JPEG_MAX_DIMENSION, ERR_VOID);

  /* Width of an input scanline must be representable as JDIMENSION. */
  samplesperrow = (long) cinfo->image_width * (long) cinfo->input_components;
  jd_samplesperrow = (JDIMENSION) samplesperrow;
  if ((long) jd_samplesperrow != samplesperrow)
    ERREXIT(cinfo, JERR_WIDTH_OVERFLOW, ERR_VOID);

  /* For now, precision must match compiled-in value... */
  if (cinfo->data_precision != BITS_IN_JSAMPLE)
    ERREXIT1(cinfo, JERR_BAD_PRECISION, cinfo->data_precision, ERR_VOID);

  /* Check that number of components won't exceed internal array sizes */
  if (cinfo->num_components > MAX_COMPONENTS)
    ERREXIT2(cinfo, JERR_COMPONENT_COUNT, cinfo->num_components,
         MAX_COMPONENTS, ERR_VOID);

  /* Compute maximum sampling factors; check factor validity */
  cinfo->max_h_samp_factor = 1;
  cinfo->max_v_samp_factor = 1;
  for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    if (compptr->h_samp_factor<=0 || compptr->h_samp_factor>MAX_SAMP_FACTOR ||
    compptr->v_samp_factor<=0 || compptr->v_samp_factor>MAX_SAMP_FACTOR)
      ERREXIT(cinfo, JERR_BAD_SAMPLING, ERR_VOID);
    cinfo->max_h_samp_factor = MAX(cinfo->max_h_samp_factor,
                   compptr->h_samp_factor);
    cinfo->max_v_samp_factor = MAX(cinfo->max_v_samp_factor,
                   compptr->v_samp_factor);
  }

  /* Compute dimensions of components */
  for (ci = 0, compptr = cinfo->comp_info; ci < cinfo->num_components;
       ci++, compptr++) {
    /* Fill in the correct component_index value; don't rely on application */
    compptr->component_index = ci;
    /* For compression, we never do any codec-based processing. */
    compptr->codec_data_unit = data_unit;
    /* Size in data units */
    compptr->width_in_data_units = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_width * (long) compptr->h_samp_factor,
            (long) (cinfo->max_h_samp_factor * data_unit));
    compptr->height_in_data_units = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_height * (long) compptr->v_samp_factor,
            (long) (cinfo->max_v_samp_factor * data_unit));
    /* Size in samples */
    compptr->downsampled_width = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_width * (long) compptr->h_samp_factor,
            (long) cinfo->max_h_samp_factor);
    compptr->downsampled_height = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_height * (long) compptr->v_samp_factor,
            (long) cinfo->max_v_samp_factor);
    /* Mark component needed (this flag isn't actually used for compression) */
    compptr->component_needed = TRUE;
  }

  /* Compute number of fully interleaved MCU rows (number of times that
   * main controller will call coefficient controller).
   */
  cinfo->total_iMCU_rows = (JDIMENSION)
    jdiv_round_up((long) cinfo->image_height,
          (long) (cinfo->max_v_samp_factor*data_unit));

  return OK_VOID;
}

#ifdef C_MULTISCAN_FILES_SUPPORTED
#define NEED_SCAN_SCRIPT
#else
#ifdef C_LOSSLESS_SUPPORTED
#define NEED_SCAN_SCRIPT
#endif
#endif

#ifdef NEED_SCAN_SCRIPT

J_WARN_UNUSED_RESULT LOCAL(void_result_t)
validate_script (j_compress_ptr cinfo)
/* Verify that the scan script in cinfo->scan_info[] is valid; also
 * determine whether it uses progressive JPEG, and set cinfo->process.
 */
{
  const jpeg_scan_info * scanptr;
  int scanno, ncomps, ci, coefi, thisi;
  int Ss, Se, Ah, Al;
  boolean component_sent[MAX_COMPONENTS];
#ifdef C_PROGRESSIVE_SUPPORTED
  int * last_bitpos_ptr;
  int last_bitpos[MAX_COMPONENTS][DCTSIZE2];
  /* -1 until that coefficient has been seen; then last Al for it */
#endif

  if (cinfo->num_scans <= 0)
    ERREXIT1(cinfo, JERR_BAD_SCAN_SCRIPT, 0, ERR_VOID);

#ifndef C_MULTISCAN_FILES_SUPPORTED
  if (cinfo->num_scans > 1)
    ERREXIT(cinfo, JERR_NOT_COMPILED);
#endif

  scanptr = cinfo->scan_info;
  if (cinfo->lossless) {
#ifdef C_LOSSLESS_SUPPORTED
    cinfo->process = JPROC_LOSSLESS;
    for (ci = 0; ci < cinfo->num_components; ci++) 
      component_sent[ci] = FALSE;
#else
    ERREXIT(cinfo, JERR_NOT_COMPILED);
#endif
  }
  /* For sequential JPEG, all scans must have Ss=0, Se=DCTSIZE2-1;
   * for progressive JPEG, no scan can have this.
   */
  else if (scanptr->Ss != 0 || scanptr->Se != DCTSIZE2-1) {
#ifdef C_PROGRESSIVE_SUPPORTED
    cinfo->process = JPROC_PROGRESSIVE;
    last_bitpos_ptr = & last_bitpos[0][0];
    for (ci = 0; ci < cinfo->num_components; ci++) 
      for (coefi = 0; coefi < DCTSIZE2; coefi++)
    *last_bitpos_ptr++ = -1;
#else
    ERREXIT(cinfo, JERR_NOT_COMPILED);
#endif
  } else {
    cinfo->process = JPROC_SEQUENTIAL;
    for (ci = 0; ci < cinfo->num_components; ci++) 
      component_sent[ci] = FALSE;
  }

  for (scanno = 1; scanno <= cinfo->num_scans; scanptr++, scanno++) {
    /* Validate component indexes */
    ncomps = scanptr->comps_in_scan;
    if (ncomps <= 0 || ncomps > MAX_COMPS_IN_SCAN)
      ERREXIT2(cinfo, JERR_COMPONENT_COUNT, ncomps, MAX_COMPS_IN_SCAN, ERR_VOID);
    for (ci = 0; ci < ncomps; ci++) {
      thisi = scanptr->component_index[ci];
      if (thisi < 0 || thisi >= cinfo->num_components)
    ERREXIT1(cinfo, JERR_BAD_SCAN_SCRIPT, scanno, ERR_VOID);
      /* Components must appear in SOF order within each scan */
      if (ci > 0 && thisi <= scanptr->component_index[ci-1])
    ERREXIT1(cinfo, JERR_BAD_SCAN_SCRIPT, scanno, ERR_VOID);
    }
    /* Validate progression parameters */
    Ss = scanptr->Ss;
    Se = scanptr->Se;
    Ah = scanptr->Ah;
    Al = scanptr->Al;
    if (cinfo->process == JPROC_LOSSLESS) {
#ifdef C_LOSSLESS_SUPPORTED
      /* The JPEG spec simply gives the range 0..15 for Al (Pt), but that
       * seems wrong: the upper bound ought to depend on data precision.
       * Perhaps they really meant 0..N-1 for N-bit precision, which is what
       * we allow here.
       */
      if (Ss < 1 || Ss > 7 ||           /* predictor selector */
      Se != 0 || Ah != 0 ||
      Al < 0 || Al >= cinfo->data_precision) /* point transform */
    ERREXIT1(cinfo, JERR_BAD_LOSSLESS_SCRIPT, scanno, ERR_VOID);
      /* Make sure components are not sent twice */
      for (ci = 0; ci < ncomps; ci++) {
    thisi = scanptr->component_index[ci];
    if (component_sent[thisi])
      ERREXIT1(cinfo, JERR_BAD_SCAN_SCRIPT, scanno, ERR_VOID);
    component_sent[thisi] = TRUE;
      }
#endif
    } else if (cinfo->process == JPROC_PROGRESSIVE) {
#ifdef C_PROGRESSIVE_SUPPORTED
      /* The JPEG spec simply gives the ranges 0..13 for Ah and Al, but that
       * seems wrong: the upper bound ought to depend on data precision.
       * Perhaps they really meant 0..N+1 for N-bit precision.
       * Here we allow 0..10 for 8-bit data; Al larger than 10 results in
       * out-of-range reconstructed DC values during the first DC scan,
       * which might cause problems for some decoders.
       */
#if BITS_IN_JSAMPLE == 8
#define MAX_AH_AL 10
#else
#define MAX_AH_AL 13
#endif
      if (Ss < 0 || Ss >= DCTSIZE2 || Se < Ss || Se >= DCTSIZE2 ||
      Ah < 0 || Ah > MAX_AH_AL || Al < 0 || Al > MAX_AH_AL)
    ERREXIT1(cinfo, JERR_BAD_PROG_SCRIPT, scanno, ERR_VOID);
      if (Ss == 0) {
    if (Se != 0)        /* DC and AC together not OK */
      ERREXIT1(cinfo, JERR_BAD_PROG_SCRIPT, scanno, ERR_VOID);
      } else {
    if (ncomps != 1)    /* AC scans must be for only one component */
      ERREXIT1(cinfo, JERR_BAD_PROG_SCRIPT, scanno, ERR_VOID);
      }
      for (ci = 0; ci < ncomps; ci++) {
    last_bitpos_ptr = & last_bitpos[scanptr->component_index[ci]][0];
    if (Ss != 0 && last_bitpos_ptr[0] < 0) /* AC without prior DC scan */
      ERREXIT1(cinfo, JERR_BAD_PROG_SCRIPT, scanno, ERR_VOID);
    for (coefi = Ss; coefi <= Se; coefi++) {
      if (last_bitpos_ptr[coefi] < 0) {
        /* first scan of this coefficient */
        if (Ah != 0)
          ERREXIT1(cinfo, JERR_BAD_PROG_SCRIPT, scanno, ERR_VOID);
      } else {
        /* not first scan */
        if (Ah != last_bitpos_ptr[coefi] || Al != Ah-1)
          ERREXIT1(cinfo, JERR_BAD_PROG_SCRIPT, scanno, ERR_VOID);
      }
      last_bitpos_ptr[coefi] = Al;
    }
      }
#endif
    } else {
      /* For sequential JPEG, all progression parameters must be these: */
      if (Ss != 0 || Se != DCTSIZE2-1 || Ah != 0 || Al != 0)
    ERREXIT1(cinfo, JERR_BAD_PROG_SCRIPT, scanno, ERR_VOID);
      /* Make sure components are not sent twice */
      for (ci = 0; ci < ncomps; ci++) {
    thisi = scanptr->component_index[ci];
    if (component_sent[thisi])
      ERREXIT1(cinfo, JERR_BAD_SCAN_SCRIPT, scanno, ERR_VOID);
    component_sent[thisi] = TRUE;
      }
    }
  }

  /* Now verify that everything got sent. */
  if (cinfo->process == JPROC_PROGRESSIVE) {
#ifdef C_PROGRESSIVE_SUPPORTED
    /* For progressive mode, we only check that at least some DC data
     * got sent for each component; the spec does not require that all bits
     * of all coefficients be transmitted.  Would it be wiser to enforce
     * transmission of all coefficient bits??
     */
    for (ci = 0; ci < cinfo->num_components; ci++) {
      if (last_bitpos[ci][0] < 0)
    ERREXIT(cinfo, JERR_MISSING_DATA, ERR_VOID);
    }
#endif
  } else {
    for (ci = 0; ci < cinfo->num_components; ci++) {
      if (! component_sent[ci])
    ERREXIT(cinfo, JERR_MISSING_DATA, ERR_VOID);
    }
  }

  return OK_VOID;
}

#endif /* NEED_SCAN_SCRIPT */


J_WARN_UNUSED_RESULT LOCAL(void_result_t)
select_scan_parameters (j_compress_ptr cinfo)
/* Set up the scan parameters for the current scan */
{
  int ci;

#ifdef NEED_SCAN_SCRIPT
  if (cinfo->scan_info != NULL) {
    /* Prepare for current scan --- the script is already validated */
    my_master_ptr master = (my_master_ptr) cinfo->master;
    const jpeg_scan_info * scanptr = cinfo->scan_info + master->scan_number;

    cinfo->comps_in_scan = scanptr->comps_in_scan;
    for (ci = 0; ci < scanptr->comps_in_scan; ci++) {
      cinfo->cur_comp_info[ci] =
    &cinfo->comp_info[scanptr->component_index[ci]];
    }
    cinfo->Ss = scanptr->Ss;
    cinfo->Se = scanptr->Se;
    cinfo->Ah = scanptr->Ah;
    cinfo->Al = scanptr->Al;
  } else
#endif
  {
    /* Prepare for single sequential-JPEG scan containing all components */
    if (cinfo->num_components > MAX_COMPS_IN_SCAN)
      ERREXIT2(cinfo, JERR_COMPONENT_COUNT, cinfo->num_components,
           MAX_COMPS_IN_SCAN, ERR_VOID);
    cinfo->comps_in_scan = cinfo->num_components;
    for (ci = 0; ci < cinfo->num_components; ci++) {
      cinfo->cur_comp_info[ci] = &cinfo->comp_info[ci];
    }
    if (cinfo->lossless) {
#ifdef C_LOSSLESS_SUPPORTED
    /* If we fall through to here, the user specified lossless, but did not
     * provide a scan script.
     */
      ERREXIT(cinfo, JERR_NO_LOSSLESS_SCRIPT, ERR_VOID);
#endif
    } else {
      cinfo->process = JPROC_SEQUENTIAL;
      cinfo->Ss = 0;
      cinfo->Se = DCTSIZE2-1;
      cinfo->Ah = 0;
      cinfo->Al = 0;
    }
  }

  return OK_VOID;
}


J_WARN_UNUSED_RESULT LOCAL(void_result_t)
per_scan_setup (j_compress_ptr cinfo)
/* Do computations that are needed before processing a JPEG scan */
/* cinfo->comps_in_scan and cinfo->cur_comp_info[] are already set */
{
  int ci, mcublks, tmp;
  jpeg_component_info *compptr;
  int data_unit = cinfo->data_unit;
  
  if (cinfo->comps_in_scan == 1) {
    
    /* Noninterleaved (single-component) scan */
    compptr = cinfo->cur_comp_info[0];
    
    /* Overall image size in MCUs */
    cinfo->MCUs_per_row = compptr->width_in_data_units;
    cinfo->MCU_rows_in_scan = compptr->height_in_data_units;
    
    /* For noninterleaved scan, always one block per MCU */
    compptr->MCU_width = 1;
    compptr->MCU_height = 1;
    compptr->MCU_data_units = 1;
    compptr->MCU_sample_width = data_unit;
    compptr->last_col_width = 1;
    /* For noninterleaved scans, it is convenient to define last_row_height
     * as the number of block rows present in the last iMCU row.
     */
    tmp = (int)compptr->height_in_data_units % compptr->v_samp_factor;
    if (tmp == 0) tmp = compptr->v_samp_factor;
    compptr->last_row_height = tmp;
    
    /* Prepare array describing MCU composition */
    cinfo->data_units_in_MCU = 1;
    cinfo->MCU_membership[0] = 0;
    
  } else {
    
    /* Interleaved (multi-component) scan */
    if (cinfo->comps_in_scan <= 0 || cinfo->comps_in_scan > MAX_COMPS_IN_SCAN)
      ERREXIT2(cinfo, JERR_COMPONENT_COUNT, cinfo->comps_in_scan,
           MAX_COMPS_IN_SCAN, ERR_VOID);
    
    /* Overall image size in MCUs */
    cinfo->MCUs_per_row = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_width,
            (long) (cinfo->max_h_samp_factor*data_unit));
    cinfo->MCU_rows_in_scan = (JDIMENSION)
      jdiv_round_up((long) cinfo->image_height,
            (long) (cinfo->max_v_samp_factor*data_unit));
    
    cinfo->data_units_in_MCU = 0;
    
    for (ci = 0; ci < cinfo->comps_in_scan; ci++) {
      compptr = cinfo->cur_comp_info[ci];
      /* Sampling factors give # of blocks of component in each MCU */
      compptr->MCU_width = compptr->h_samp_factor;
      compptr->MCU_height = compptr->v_samp_factor;
      compptr->MCU_data_units = compptr->MCU_width * compptr->MCU_height;
      compptr->MCU_sample_width = compptr->MCU_width * data_unit;
      /* Figure number of non-dummy blocks in last MCU column & row */
      tmp = (int)compptr->width_in_data_units % compptr->MCU_width;
      if (tmp == 0) tmp = compptr->MCU_width;
      compptr->last_col_width = tmp;
      tmp = (int)compptr->height_in_data_units % compptr->MCU_height;
      if (tmp == 0) tmp = compptr->MCU_height;
      compptr->last_row_height = tmp;
      /* Prepare array describing MCU composition */
      mcublks = compptr->MCU_data_units;
      if (cinfo->data_units_in_MCU + mcublks > C_MAX_DATA_UNITS_IN_MCU)
    ERREXIT(cinfo, JERR_BAD_MCU_SIZE, ERR_VOID);
      while (mcublks-- > 0) {
    cinfo->MCU_membership[cinfo->data_units_in_MCU++] = ci;
      }
    }
    
  }

  /* Convert restart specified in rows to actual MCU count. */
  /* Note that count must fit in 16 bits, so we provide limiting. */
  if (cinfo->restart_in_rows > 0) {
    long nominal = (long) cinfo->restart_in_rows * (long) cinfo->MCUs_per_row;
    cinfo->restart_interval = (unsigned int) MIN(nominal, 65535L);
  }
 
  return OK_VOID;
}


/*
 * Per-pass setup.
 * This is called at the beginning of each pass.  We determine which modules
 * will be active during this pass and give them appropriate start_pass calls.
 * We also set is_last_pass to indicate whether any more passes will be
 * required.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
prepare_for_pass (j_compress_ptr cinfo)
{
  /* j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec; */
  my_master_ptr master = (my_master_ptr) cinfo->master;

  switch (master->pass_type) {
  case main_pass: {
    /* Initial pass: will collect input data, and do either Huffman
     * optimization or data output for the first scan.
     */
    void_result_t select_scan_parameters_result = select_scan_parameters(cinfo);
    if (select_scan_parameters_result.is_err) {
      return select_scan_parameters_result;
    }
    void_result_t per_scan_setup_result = per_scan_setup(cinfo);
    if (per_scan_setup_result.is_err) {
      return per_scan_setup_result;
    }
    if (! cinfo->raw_data_in) {
      void_result_t start_pass_result = (*cinfo->cconvert->start_pass) (cinfo);
      if (start_pass_result.is_err) {
        return start_pass_result;
      }
      (*cinfo->downsample->start_pass) (cinfo);
      start_pass_result = (*cinfo->prep->start_pass) (cinfo, JBUF_PASS_THRU);
      if (start_pass_result.is_err) {
        return start_pass_result;
      }
    }
    void_result_t entropy_start_pass_result = (*cinfo->codec->entropy_start_pass) (cinfo, cinfo->optimize_coding);
    if (entropy_start_pass_result.is_err) {
      return entropy_start_pass_result;
    }
    void_result_t start_pass_result = (*cinfo->codec->start_pass) (cinfo,
                 (master->total_passes > 1 ?
                  JBUF_SAVE_AND_PASS : JBUF_PASS_THRU));
    if (start_pass_result.is_err) {
      return start_pass_result;
    }
    start_pass_result = (*cinfo->main->start_pass) (cinfo, JBUF_PASS_THRU);
    if (start_pass_result.is_err) {
      return start_pass_result;
    }
    if (cinfo->optimize_coding) {
      /* No immediate data output; postpone writing frame/scan headers */
      master->pub.call_pass_startup = FALSE;
    } else {
      /* Will write frame/scan headers at first jpeg_write_scanlines call */
      master->pub.call_pass_startup = TRUE;
    }
    break;
  }
#ifdef ENTROPY_OPT_SUPPORTED
  case huff_opt_pass:
    /* Do Huffman optimization for a scan after the first one. */
    {
      void_result_t select_scan_parameters_result = select_scan_parameters(cinfo);
      if (select_scan_parameters_result.is_err) {
        return select_scan_parameters_result;
      }
      void_result_t per_scan_setup_result = per_scan_setup(cinfo);
      if (per_scan_setup_result.is_err) {
        return per_scan_setup_result;
      }
    }
#ifdef WITH_ARITHMETIC_PATCH
    if ((*cinfo->codec->need_optimization_pass) (cinfo)) {
#else
    boolean_result_t need_optimization_pass_result = (*cinfo->codec->need_optimization_pass) (cinfo);
    if (need_optimization_pass_result.is_err) {
      return ERR_VOID(need_optimization_pass_result.err_code);
    }
    if (need_optimization_pass_result.value || cinfo->arith_code) {
#endif
      void_result_t entropy_start_pass_result = (*cinfo->codec->entropy_start_pass) (cinfo, TRUE);
      if (entropy_start_pass_result.is_err) {
        return entropy_start_pass_result;
      }
      void_result_t start_pass_result = (*cinfo->codec->start_pass) (cinfo, JBUF_CRANK_DEST);
      if (start_pass_result.is_err) {
        return start_pass_result;
      }
      master->pub.call_pass_startup = FALSE;
      break;
    }
    /* Special case: Huffman DC refinement scans need no Huffman table
     * and therefore we can skip the optimization pass for them.
     */
    master->pass_type = output_pass;
    master->pass_number++;
    /*FALLTHROUGH*/
#endif
  case output_pass: {
    /* Do a data-output pass. */
    /* We need not repeat per-scan setup if prior optimization pass did it. */
    if (! cinfo->optimize_coding) {
      void_result_t select_scan_parameters_result = select_scan_parameters(cinfo);
      if (select_scan_parameters_result.is_err) {
        return select_scan_parameters_result;
      }
      void_result_t per_scan_setup_result = per_scan_setup(cinfo);
      if (per_scan_setup_result.is_err) {
        return per_scan_setup_result;
      }
    }
    void_result_t entropy_start_pass_result = (*cinfo->codec->entropy_start_pass) (cinfo, FALSE);
    if (entropy_start_pass_result.is_err) {
      return entropy_start_pass_result;
    }
    void_result_t start_pass_result = (*cinfo->codec->start_pass) (cinfo, JBUF_CRANK_DEST);
    if (start_pass_result.is_err) {
      return start_pass_result;
    }
    /* We emit frame/scan headers now */
    if (master->scan_number == 0) {
      void_result_t write_frame_header_result = (*cinfo->marker->write_frame_header) (cinfo);
      if (write_frame_header_result.is_err) {
        return write_frame_header_result;
      }
    }
    void_result_t write_scan_header_result = (*cinfo->marker->write_scan_header) (cinfo);
    if (write_scan_header_result.is_err) {
      return write_scan_header_result;
    }
    master->pub.call_pass_startup = FALSE;
    break;
  }
  default:
    ERREXIT(cinfo, JERR_NOT_COMPILED, ERR_VOID);
  }

  master->pub.is_last_pass = (master->pass_number == master->total_passes-1);

  /* Set up progress monitor's pass info if present */
  if (cinfo->progress != NULL) {
    cinfo->progress->completed_passes = master->pass_number;
    cinfo->progress->total_passes = master->total_passes;
  }

  return OK_VOID;
}


/*
 * Special start-of-pass hook.
 * This is called by jpeg_write_scanlines if call_pass_startup is TRUE.
 * In single-pass processing, we need this hook because we don't want to
 * write frame/scan headers during jpeg_start_compress; we want to let the
 * application write COM markers etc. between jpeg_start_compress and the
 * jpeg_write_scanlines loop.
 * In multi-pass processing, this routine is not used.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
pass_startup (j_compress_ptr cinfo)
{
  cinfo->master->call_pass_startup = FALSE; /* reset flag so call only once */

  void_result_t write_frame_header_result = (*cinfo->marker->write_frame_header) (cinfo);
  if (write_frame_header_result.is_err) {
    return write_frame_header_result;
  }
  void_result_t write_scan_header_result = (*cinfo->marker->write_scan_header) (cinfo);
  if (write_scan_header_result.is_err) {
    return write_scan_header_result;
  }

  return OK_VOID;
}


/*
 * Finish up at end of pass.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
finish_pass_master (j_compress_ptr cinfo)
{
  j_lossy_c_ptr lossyc = (j_lossy_c_ptr) cinfo->codec;
  my_master_ptr master = (my_master_ptr) cinfo->master;

  /* The entropy coder always needs an end-of-pass call,
   * either to analyze statistics or to flush its output buffer.
   */
  void_result_t entropy_finish_pass_result = (*lossyc->pub.entropy_finish_pass) (cinfo);
  if (entropy_finish_pass_result.is_err) {
    return entropy_finish_pass_result;
  }

  /* Update state for next pass */
  switch (master->pass_type) {
  case main_pass:
    /* next pass is either output of scan 0 (after optimization)
     * or output of scan 1 (if no optimization).
     */
    master->pass_type = output_pass;
    if (! cinfo->optimize_coding)
      master->scan_number++;
    break;
  case huff_opt_pass:
    /* next pass is always output of current scan */
    master->pass_type = output_pass;
    break;
  case output_pass:
    /* next pass is either optimization or output of next scan */
    if (cinfo->optimize_coding)
      master->pass_type = huff_opt_pass;
    master->scan_number++;
    break;
  }

  master->pass_number++;

  return OK_VOID;
}


/*
 * Initialize master compression control.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_c_master_control (j_compress_ptr cinfo, boolean transcode_only)
{
  my_master_ptr master;

  void_ptr_result_t alloc_small_result =
      (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
                  SIZEOF(my_comp_master));
  if (alloc_small_result.is_err) {
    return ERR_VOID(alloc_small_result.err_code);
  }
  master = (my_master_ptr) alloc_small_result.value;
  cinfo->master = (struct jpeg_comp_master *) master;
  master->pub.prepare_for_pass = prepare_for_pass;
  master->pub.pass_startup = pass_startup;
  master->pub.finish_pass = finish_pass_master;
  master->pub.is_last_pass = FALSE;

  cinfo->data_unit = cinfo->lossless ? 1 : DCTSIZE;

  /* Validate parameters, determine derived values */
  void_result_t initial_setup_result = initial_setup(cinfo);
  if (initial_setup_result.is_err) {
    return initial_setup_result;
  }

  if (cinfo->scan_info != NULL) {
#ifdef NEED_SCAN_SCRIPT
    void_result_t validate_script_result = validate_script(cinfo);
    if (validate_script_result.is_err) {
      return validate_script_result;
    }
#else
    ERREXIT(cinfo, JERR_NOT_COMPILED);
#endif
  } else {
    cinfo->process = JPROC_SEQUENTIAL;
    cinfo->num_scans = 1;
  }

#ifdef WITH_ARITHMETIC_PATCH
  if ((cinfo->arith_code == 0) &&
      (cinfo->process == JPROC_PROGRESSIVE ||   /*  TEMPORARY HACK ??? */
       cinfo->process == JPROC_LOSSLESS))
#else
  if (cinfo->process == JPROC_PROGRESSIVE ||    /*  TEMPORARY HACK ??? */
      cinfo->process == JPROC_LOSSLESS)
#endif
    cinfo->optimize_coding = TRUE; /* assume default tables no good for
                    * progressive mode or lossless mode */

  /* Initialize my private state */
  if (transcode_only) {
    /* no main pass in transcoding */
    if (cinfo->optimize_coding)
      master->pass_type = huff_opt_pass;
    else
      master->pass_type = output_pass;
  } else {
    /* for normal compression, first pass is always this type: */
    master->pass_type = main_pass;
  }
  master->scan_number = 0;
  master->pass_number = 0;
  if (cinfo->optimize_coding)
    master->total_passes = cinfo->num_scans * 2;
  else
    master->total_passes = cinfo->num_scans;

  return OK_VOID;
}
