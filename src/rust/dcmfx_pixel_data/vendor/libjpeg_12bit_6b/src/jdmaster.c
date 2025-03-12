/*
 * jdmaster.c
 *
 * Copyright (C) 1991-1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains master control logic for the JPEG decompressor.
 * These routines are concerned with selecting the modules to be executed
 * and with determining the number of passes and the work to be done in each
 * pass.
 */

#define JPEG_INTERNALS
#include "jinclude12.h"
#include "jpeglib12.h"


/* Private state */

typedef struct {
  struct jpeg_decomp_master pub; /* public fields */

  int pass_number;		/* # of passes completed */

  boolean using_merged_upsample; /* TRUE if using merged upsample/cconvert */

  /* Saved references to initialized quantizer modules,
   * in case we need to switch modes.
   */
  struct jpeg_color_quantizer * quantizer_1pass;
  struct jpeg_color_quantizer * quantizer_2pass;
} my_decomp_master;

typedef my_decomp_master * my_master_ptr;


/*
 * Determine whether merged upsample/color conversion should be used.
 * CRUCIAL: this must match the actual capabilities of jdmerge.c!
 */

LOCAL(boolean)
use_merged_upsample (j_decompress_ptr cinfo)
{
#ifdef UPSAMPLE_MERGING_SUPPORTED
  /* Merging is the equivalent of plain box-filter upsampling */
  if (cinfo->do_fancy_upsampling || cinfo->CCIR601_sampling)
    return FALSE;
  /* jdmerge.c only supports YCC=>RGB color conversion */
  if (cinfo->jpeg_color_space != JCS_YCbCr || cinfo->num_components != 3 ||
      cinfo->out_color_space != JCS_RGB ||
      cinfo->out_color_components != RGB_PIXELSIZE)
    return FALSE;
  /* and it only handles 2h1v or 2h2v sampling ratios */
  if (cinfo->comp_info[0].h_samp_factor != 2 ||
      cinfo->comp_info[1].h_samp_factor != 1 ||
      cinfo->comp_info[2].h_samp_factor != 1 ||
      cinfo->comp_info[0].v_samp_factor >  2 ||
      cinfo->comp_info[1].v_samp_factor != 1 ||
      cinfo->comp_info[2].v_samp_factor != 1)
    return FALSE;
  /* furthermore, it doesn't work if each component has been
     processed differently */
  if (cinfo->comp_info[0].codec_data_unit != cinfo->min_codec_data_unit ||
      cinfo->comp_info[1].codec_data_unit != cinfo->min_codec_data_unit ||
      cinfo->comp_info[2].codec_data_unit != cinfo->min_codec_data_unit)
    return FALSE;
  /* ??? also need to test for upsample-time rescaling, when & if supported */
  return TRUE;			/* by golly, it'll work... */
#else
  return FALSE;
#endif
}


/*
 * Compute output image dimensions and related values.
 * NOTE: this is exported for possible use by application.
 * Hence it mustn't do anything that can't be done twice.
 * Also note that it may be called before the master module is initialized!
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_calc_output_dimensions (j_decompress_ptr cinfo)
/* Do computations that are needed before master selection phase */
{
  /* Prevent application from calling me at wrong times */
  if (cinfo->global_state != DSTATE_READY)
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_VOID);

  (*cinfo->codec->calc_output_dimensions) (cinfo);

  /* Report number of components in selected colorspace. */
  /* Probably this should be in the color conversion module... */
  switch (cinfo->out_color_space) {
  case JCS_GRAYSCALE:
    cinfo->out_color_components = 1;
    break;
  case JCS_RGB:
#if RGB_PIXELSIZE != 3
    cinfo->out_color_components = RGB_PIXELSIZE;
    break;
#endif /* else share code with YCbCr */
  case JCS_YCbCr:
    cinfo->out_color_components = 3;
    break;
  case JCS_CMYK:
  case JCS_YCCK:
    cinfo->out_color_components = 4;
    break;
  default:			/* else must be same colorspace as in file */
    cinfo->out_color_components = cinfo->num_components;
    break;
  }
  cinfo->output_components = (cinfo->quantize_colors ? 1 :
			      cinfo->out_color_components);

  /* See if upsampler will want to emit more than one row at a time */
  if (use_merged_upsample(cinfo))
    cinfo->rec_outbuf_height = cinfo->max_v_samp_factor;
  else
    cinfo->rec_outbuf_height = 1;

  return OK_VOID;
}


/*
 * Several decompression processes need to range-limit values to the range
 * 0..MAXJSAMPLE; the input value may fall somewhat outside this range
 * due to noise introduced by quantization, roundoff error, etc.  These
 * processes are inner loops and need to be as fast as possible.  On most
 * machines, particularly CPUs with pipelines or instruction prefetch,
 * a (subscript-check-less) C table lookup
 *		x = sample_range_limit[x];
 * is faster than explicit tests
 *		if (x < 0)  x = 0;
 *		else if (x > MAXJSAMPLE)  x = MAXJSAMPLE;
 * These processes all use a common table prepared by the routine below.
 *
 * For most steps we can mathematically guarantee that the initial value
 * of x is within MAXJSAMPLE+1 of the legal range, so a table running from
 * -(MAXJSAMPLE+1) to 2*MAXJSAMPLE+1 is sufficient.  But for the initial
 * limiting step (just after the IDCT), a wildly out-of-range value is 
 * possible if the input data is corrupt.  To avoid any chance of indexing
 * off the end of memory and getting a bad-pointer trap, we perform the
 * post-IDCT limiting thus:
 *		x = range_limit[x & MASK];
 * where MASK is 2 bits wider than legal sample data, ie 10 bits for 8-bit
 * samples.  Under normal circumstances this is more than enough range and
 * a correct output will be generated; with bogus input data the mask will
 * cause wraparound, and we will safely generate a bogus-but-in-range output.
 * For the post-IDCT step, we want to convert the data from signed to unsigned
 * representation by adding CENTERJSAMPLE at the same time that we limit it.
 * So the post-IDCT limiting table ends up looking like this:
 *   CENTERJSAMPLE,CENTERJSAMPLE+1,...,MAXJSAMPLE,
 *   MAXJSAMPLE (repeat 2*(MAXJSAMPLE+1)-CENTERJSAMPLE times),
 *   0          (repeat 2*(MAXJSAMPLE+1)-CENTERJSAMPLE times),
 *   0,1,...,CENTERJSAMPLE-1
 * Negative inputs select values from the upper half of the table after
 * masking.
 *
 * We can save some space by overlapping the start of the post-IDCT table
 * with the simpler range limiting table.  The post-IDCT table begins at
 * sample_range_limit + CENTERJSAMPLE.
 *
 * Note that the table is allocated in near data space on PCs; it's small
 * enough and used often enough to justify this.
 */

J_WARN_UNUSED_RESULT LOCAL(void_result_t)
prepare_range_limit_table (j_decompress_ptr cinfo)
/* Allocate and fill in the sample_range_limit table */
{
  JSAMPLE * table;
  int i;

  void_ptr_result_t alloc_small_result =
    (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
		  (5 * (MAXJSAMPLE+1) + CENTERJSAMPLE) * SIZEOF(JSAMPLE));
  if (alloc_small_result.is_err)
    return ERR_VOID(alloc_small_result.err_code);
  table = (JSAMPLE *) alloc_small_result.value;
  table += (MAXJSAMPLE+1);	/* allow negative subscripts of simple table */
  cinfo->sample_range_limit = table;
  /* First segment of "simple" table: limit[x] = 0 for x < 0 */
  MEMZERO(table - (MAXJSAMPLE+1), (MAXJSAMPLE+1) * SIZEOF(JSAMPLE));
  /* Main part of "simple" table: limit[x] = x */
  for (i = 0; i <= MAXJSAMPLE; i++)
    table[i] = (JSAMPLE) i;
  table += CENTERJSAMPLE;	/* Point to where post-IDCT table starts */
  /* End of simple table, rest of first half of post-IDCT table */
  for (i = CENTERJSAMPLE; i < 2*(MAXJSAMPLE+1); i++)
    table[i] = MAXJSAMPLE;
  /* Second half of post-IDCT table */
  MEMZERO(table + (2 * (MAXJSAMPLE+1)),
	  (2 * (MAXJSAMPLE+1) - CENTERJSAMPLE) * SIZEOF(JSAMPLE));
  MEMCOPY(table + (4 * (MAXJSAMPLE+1) - CENTERJSAMPLE),
	  cinfo->sample_range_limit, CENTERJSAMPLE * SIZEOF(JSAMPLE));

  return OK_VOID;
}


/*
 * Master selection of decompression modules.
 * This is done once at jpeg_start_decompress time.  We determine
 * which modules will be used and give them appropriate initialization calls.
 * We also initialize the decompressor input side to begin consuming data.
 *
 * Since jpeg_read_header has finished, we know what is in the SOF
 * and (first) SOS markers.  We also have all the application parameter
 * settings.
 */

J_WARN_UNUSED_RESULT LOCAL(void_result_t)
master_selection (j_decompress_ptr cinfo)
{
  my_master_ptr master = (my_master_ptr) cinfo->master;
  long samplesperrow;
  JDIMENSION jd_samplesperrow;

  /* Initialize dimensions and other stuff */
  void_result_t jpeg_calc_output_dimensions_result = jpeg_calc_output_dimensions(cinfo);
  if (jpeg_calc_output_dimensions_result.is_err)
    return jpeg_calc_output_dimensions_result;
  void_result_t prepare_range_limit_table_result = prepare_range_limit_table(cinfo);
  if (prepare_range_limit_table_result.is_err)
    return prepare_range_limit_table_result;

  /* Width of an output scanline must be representable as JDIMENSION. */
  samplesperrow = (long) cinfo->output_width * (long) cinfo->out_color_components;
  jd_samplesperrow = (JDIMENSION) samplesperrow;
  if ((long) jd_samplesperrow != samplesperrow)
    ERREXIT(cinfo, JERR_WIDTH_OVERFLOW, ERR_VOID);

  /* Initialize my private state */
  master->pass_number = 0;
  master->using_merged_upsample = use_merged_upsample(cinfo);

  /* Color quantizer selection */
  master->quantizer_1pass = NULL;
  master->quantizer_2pass = NULL;
  /* No mode changes if not using buffered-image mode. */
  if (! cinfo->quantize_colors || ! cinfo->buffered_image) {
    cinfo->enable_1pass_quant = FALSE;
    cinfo->enable_external_quant = FALSE;
    cinfo->enable_2pass_quant = FALSE;
  }
  if (cinfo->quantize_colors) {
    if (cinfo->raw_data_out)
      ERREXIT(cinfo, JERR_NOTIMPL, ERR_VOID);
    /* 2-pass quantizer only works in 3-component color space. */
    if (cinfo->out_color_components != 3) {
      cinfo->enable_1pass_quant = TRUE;
      cinfo->enable_external_quant = FALSE;
      cinfo->enable_2pass_quant = FALSE;
      cinfo->colormap = NULL;
    } else if (cinfo->colormap != NULL) {
      cinfo->enable_external_quant = TRUE;
    } else if (cinfo->two_pass_quantize) {
      cinfo->enable_2pass_quant = TRUE;
    } else {
      cinfo->enable_1pass_quant = TRUE;
    }

    if (cinfo->enable_1pass_quant) {
#ifdef QUANT_1PASS_SUPPORTED
      void_result_t jinit_1pass_quantizer_result = jinit_1pass_quantizer(cinfo);
      if (jinit_1pass_quantizer_result.is_err)
        return jinit_1pass_quantizer_result;
      master->quantizer_1pass = cinfo->cquantize;
#else
      ERREXIT(cinfo, JERR_NOT_COMPILED, ERR_VOID);
#endif
    }

    /* We use the 2-pass code to map to external colormaps. */
    if (cinfo->enable_2pass_quant || cinfo->enable_external_quant) {
#ifdef QUANT_2PASS_SUPPORTED
      void_result_t jinit_2pass_quantizer_result = jinit_2pass_quantizer(cinfo);
      if (jinit_2pass_quantizer_result.is_err)
        return jinit_2pass_quantizer_result;
      master->quantizer_2pass = cinfo->cquantize;
#else
      ERREXIT(cinfo, JERR_NOT_COMPILED, ERR_VOID);
#endif
    }
    /* If both quantizers are initialized, the 2-pass one is left active;
     * this is necessary for starting with quantization to an external map.
     */
  }

  /* Post-processing: in particular, color conversion first */
  if (! cinfo->raw_data_out) {
    if (master->using_merged_upsample) {
#ifdef UPSAMPLE_MERGING_SUPPORTED
      void_result_t jinit_merged_upsampler_result = jinit_merged_upsampler(cinfo);
      if (jinit_merged_upsampler_result.is_err) /* does color conversion too */
        return jinit_merged_upsampler_result;
#else
      ERREXIT(cinfo, JERR_NOT_COMPILED, ERR_VOID);
#endif
    } else {
      void_result_t jinit_color_deconverter_result = jinit_color_deconverter(cinfo);
      if (jinit_color_deconverter_result.is_err)
        return jinit_color_deconverter_result;
      void_result_t jinit_upsampler_result = jinit_upsampler(cinfo);
      if (jinit_upsampler_result.is_err)
        return jinit_upsampler_result;
    }
    void_result_t jinit_d_post_controller_result = jinit_d_post_controller(cinfo, cinfo->enable_2pass_quant);
    if (jinit_d_post_controller_result.is_err)
      return jinit_d_post_controller_result;
  }

  /* Initialize principal buffer controllers. */
  if (! cinfo->raw_data_out) {
    void_result_t jinit_d_main_controller_result = jinit_d_main_controller(cinfo, FALSE /* never need full buffer here */);
    if (jinit_d_main_controller_result.is_err)
      return jinit_d_main_controller_result;
  }

  /* We can now tell the memory manager to allocate virtual arrays. */
  void_result_t realize_virt_arrays_result = ((*cinfo->mem->realize_virt_arrays) ((j_common_ptr) cinfo));
  if (realize_virt_arrays_result.is_err)
    return realize_virt_arrays_result;

  /* Initialize input side of decompressor to consume first scan. */
  void_result_t start_input_pass_result = ((*cinfo->inputctl->start_input_pass) (cinfo));
  if (start_input_pass_result.is_err)
    return start_input_pass_result;

#ifdef D_MULTISCAN_FILES_SUPPORTED
  /* If jpeg_start_decompress will read the whole file, initialize
   * progress monitoring appropriately.  The input step is counted
   * as one pass.
   */
  if (cinfo->progress != NULL && ! cinfo->buffered_image &&
      cinfo->inputctl->has_multiple_scans) {
    int nscans;
    /* Estimate number of scans to set pass_limit. */
    if (cinfo->process == JPROC_PROGRESSIVE) {
      /* Arbitrarily estimate 2 interleaved DC scans + 3 AC scans/component. */
      nscans = 2 + 3 * cinfo->num_components;
    } else {
      /* For a nonprogressive multiscan file, estimate 1 scan per component. */
      nscans = cinfo->num_components;
    }
    cinfo->progress->pass_counter = 0L;
    cinfo->progress->pass_limit = (long) cinfo->total_iMCU_rows * nscans;
    cinfo->progress->completed_passes = 0;
    cinfo->progress->total_passes = (cinfo->enable_2pass_quant ? 3 : 2);
    /* Count the input pass as done */
    master->pass_number++;
  }
#endif /* D_MULTISCAN_FILES_SUPPORTED */

  return OK_VOID;
}


/*
 * Per-pass setup.
 * This is called at the beginning of each output pass.  We determine which
 * modules will be active during this pass and give them appropriate
 * start_pass calls.  We also set is_dummy_pass to indicate whether this
 * is a "real" output pass or a dummy pass for color quantization.
 * (In the latter case, jdapistd.c will crank the pass to completion.)
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
prepare_for_output_pass (j_decompress_ptr cinfo)
{
  my_master_ptr master = (my_master_ptr) cinfo->master;

  if (master->pub.is_dummy_pass) {
#ifdef QUANT_2PASS_SUPPORTED
    /* Final pass of 2-pass quantization */
    master->pub.is_dummy_pass = FALSE;
    void_result_t start_pass_result = ((*cinfo->cquantize->start_pass) (cinfo, FALSE));
    if (start_pass_result.is_err)
      return start_pass_result;
    start_pass_result = ((*cinfo->post->start_pass) (cinfo, JBUF_CRANK_DEST));
    if (start_pass_result.is_err)
      return start_pass_result;
    start_pass_result = ((*cinfo->main->start_pass) (cinfo, JBUF_CRANK_DEST));
    if (start_pass_result.is_err)
      return start_pass_result;
#else
    ERREXIT(cinfo, JERR_NOT_COMPILED, ERR_VOID);
#endif /* QUANT_2PASS_SUPPORTED */
  } else {
    if (cinfo->quantize_colors && cinfo->colormap == NULL) {
      /* Select new quantization method */
      if (cinfo->two_pass_quantize && cinfo->enable_2pass_quant) {
	cinfo->cquantize = master->quantizer_2pass;
	master->pub.is_dummy_pass = TRUE;
      } else if (cinfo->enable_1pass_quant) {
	cinfo->cquantize = master->quantizer_1pass;
      } else {
	ERREXIT(cinfo, JERR_MODE_CHANGE, ERR_VOID);
      }
    }
    void_result_t start_output_pass_result = ((*cinfo->codec->start_output_pass) (cinfo));
    if (start_output_pass_result.is_err)
      return start_output_pass_result;
    if (! cinfo->raw_data_out) {
      if (! master->using_merged_upsample)
	(*cinfo->cconvert->start_pass) (cinfo);
      (*cinfo->upsample->start_pass) (cinfo);
      if (cinfo->quantize_colors) {
        void_result_t start_pass_result = ((*cinfo->cquantize->start_pass) (cinfo, master->pub.is_dummy_pass));
	      if (start_pass_result.is_err)
          return start_pass_result;
      }
      void_result_t start_pass_result = ((*cinfo->post->start_pass) (cinfo,
        (master->pub.is_dummy_pass ? JBUF_SAVE_AND_PASS : JBUF_PASS_THRU)));
      if (start_pass_result.is_err)
        return start_pass_result;
      start_pass_result = ((*cinfo->main->start_pass) (cinfo, JBUF_PASS_THRU));
      if (start_pass_result.is_err)
        return ERR_VOID(start_pass_result.err_code);
    }
  }

  /* Set up progress monitor's pass info if present */
  if (cinfo->progress != NULL) {
    cinfo->progress->completed_passes = master->pass_number;
    cinfo->progress->total_passes = master->pass_number +
				    (master->pub.is_dummy_pass ? 2 : 1);
    /* In buffered-image mode, we assume one more output pass if EOI not
     * yet reached, but no more passes if EOI has been reached.
     */
    if (cinfo->buffered_image && ! cinfo->inputctl->eoi_reached) {
      cinfo->progress->total_passes += (cinfo->enable_2pass_quant ? 2 : 1);
    }
  }

  return OK_VOID;
}


/*
 * Finish up at end of an output pass.
 */

J_WARN_UNUSED_RESULT METHODDEF(void_result_t)
finish_output_pass (j_decompress_ptr cinfo)
{
  my_master_ptr master = (my_master_ptr) cinfo->master;

  if (cinfo->quantize_colors) {
    void_result_t finish_pass_result = ((*cinfo->cquantize->finish_pass) (cinfo));
    if (finish_pass_result.is_err)
      return finish_pass_result;
  }
  master->pass_number++;

  return OK_VOID;
}


#ifdef D_MULTISCAN_FILES_SUPPORTED

/*
 * Switch to a new external colormap between output passes.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jpeg_new_colormap (j_decompress_ptr cinfo)
{
  my_master_ptr master = (my_master_ptr) cinfo->master;

  /* Prevent application from calling me at wrong times */
  if (cinfo->global_state != DSTATE_BUFIMAGE)
    ERREXIT1(cinfo, JERR_BAD_STATE, cinfo->global_state, ERR_VOID);

  if (cinfo->quantize_colors && cinfo->enable_external_quant &&
      cinfo->colormap != NULL) {
    /* Select 2-pass quantizer for external colormap use */
    cinfo->cquantize = master->quantizer_2pass;
    /* Notify quantizer of colormap change */
    void_result_t new_color_map_result = ((*cinfo->cquantize->new_color_map) (cinfo));
    if (new_color_map_result.is_err)
      return new_color_map_result;
    master->pub.is_dummy_pass = FALSE; /* just in case */
  } else
    ERREXIT(cinfo, JERR_MODE_CHANGE, ERR_VOID);

  return OK_VOID;
}

#endif /* D_MULTISCAN_FILES_SUPPORTED */


/*
 * Initialize master decompression control and select active modules.
 * This is performed at the start of jpeg_start_decompress.
 */

J_WARN_UNUSED_RESULT GLOBAL(void_result_t)
jinit_master_decompress (j_decompress_ptr cinfo)
{
  my_master_ptr master;

  void_ptr_result_t alloc_small_result =
      (*cinfo->mem->alloc_small) ((j_common_ptr) cinfo, JPOOL_IMAGE,
				  SIZEOF(my_decomp_master));
  if (alloc_small_result.is_err)
    return ERR_VOID(alloc_small_result.err_code);
  master = (my_master_ptr) alloc_small_result.value;
  cinfo->master = (struct jpeg_decomp_master *) master;
  master->pub.prepare_for_output_pass = prepare_for_output_pass;
  master->pub.finish_output_pass = finish_output_pass;

  master->pub.is_dummy_pass = FALSE;

  return master_selection(cinfo);
}
