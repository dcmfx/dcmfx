/*
 * jlossls.h
 *
 * Copyright (C) 1998, Thomas G. Lane.
 * This file is part of the Independent JPEG Group's software.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This include file contains common declarations for the lossless JPEG
 * codec modules.
 */

#ifndef JLOSSLS_H
#define JLOSSLS_H


/*
 * Table H.1: Predictors for lossless coding.
 */

#define PREDICTOR1	Ra
#define PREDICTOR2	Rb
#define PREDICTOR3	Rc
#define PREDICTOR4	(int) ((IJG_INT32) Ra + (IJG_INT32) Rb - (IJG_INT32) Rc)
#define PREDICTOR5	(int) ((IJG_INT32) Ra + RIGHT_SHIFT((IJG_INT32) Rb - (IJG_INT32) Rc, 1))
#define PREDICTOR6	(int) ((IJG_INT32) Rb + RIGHT_SHIFT((IJG_INT32) Ra - (IJG_INT32) Rc, 1))
#define PREDICTOR7	(int) RIGHT_SHIFT((IJG_INT32) Ra + (IJG_INT32) Rb, 1)


typedef JMETHOD(void, predict_difference_method_ptr,
		(j_compress_ptr cinfo, int ci,
		 JSAMPROW input_buf, JSAMPROW prev_row,
		 JDIFFROW diff_buf, JDIMENSION width));

typedef JMETHOD(void, scaler_method_ptr,
		(j_compress_ptr cinfo, int ci,
		 JSAMPROW input_buf, JSAMPROW output_buf,
		 JDIMENSION width));

/* Lossless-specific compression codec (compressor proper) */
typedef struct {
  struct jpeg_c_codec pub; /* public fields */


  /* Difference buffer control */
  J_WARN_UNUSED_RESULT JMETHOD(void_result_t, diff_start_pass, (j_compress_ptr cinfo,
				  J_BUF_MODE pass_mode));

  /* Pointer to data which is private to diff controller */
  void *diff_private;


  /* Entropy encoding */
  J_WARN_UNUSED_RESULT JMETHOD(jdimension_result_t, entropy_encode_mcus, (j_compress_ptr cinfo,
					    JDIFFIMAGE diff_buf,
					    JDIMENSION MCU_row_num,
					    JDIMENSION MCU_col_num,
					    JDIMENSION nMCU));

  /* Pointer to data which is private to entropy module */
  void *entropy_private;


  /* Prediction, differencing */
  J_WARN_UNUSED_RESULT JMETHOD(void_result_t, predict_start_pass, (j_compress_ptr cinfo));

  /* It is useful to allow each component to have a separate diff method. */
  predict_difference_method_ptr predict_difference[MAX_COMPONENTS];

  /* Pointer to data which is private to predictor module */
  void *pred_private;

  /* Sample scaling */
  JMETHOD(void, scaler_start_pass, (j_compress_ptr cinfo));
  JMETHOD(void, scaler_scale, (j_compress_ptr cinfo,
			       JSAMPROW input_buf, JSAMPROW output_buf,
			       JDIMENSION width));

  /* Pointer to data which is private to scaler module */
  void *scaler_private;

} jpeg_lossless_c_codec;

typedef jpeg_lossless_c_codec * j_lossless_c_ptr;


typedef JMETHOD(void, predict_undifference_method_ptr,
		(j_decompress_ptr cinfo, int comp_index,
		 JDIFFROW diff_buf, JDIFFROW prev_row,
		 JDIFFROW undiff_buf, JDIMENSION width));

/* Lossless-specific decompression codec (decompressor proper) */
typedef struct {
  struct jpeg_d_codec pub; /* public fields */


  /* Difference buffer control */
  J_WARN_UNUSED_RESULT JMETHOD(void_result_t, diff_start_input_pass, (j_decompress_ptr cinfo));

  /* Pointer to data which is private to diff controller */
  void *diff_private;


  /* Entropy decoding */
  J_WARN_UNUSED_RESULT JMETHOD(void_result_t, entropy_start_pass, (j_decompress_ptr cinfo));
  J_WARN_UNUSED_RESULT JMETHOD(boolean_result_t, entropy_process_restart, (j_decompress_ptr cinfo));
  J_WARN_UNUSED_RESULT JMETHOD(jdimension_result_t, entropy_decode_mcus, (j_decompress_ptr cinfo,
					    JDIFFIMAGE diff_buf,
					    JDIMENSION MCU_row_num,
					    JDIMENSION MCU_col_num,
					    JDIMENSION nMCU));

  /* Pointer to data which is private to entropy module */
  void *entropy_private;


  /* Prediction, undifferencing */
  J_WARN_UNUSED_RESULT JMETHOD(void_result_t, predict_start_pass, (j_decompress_ptr cinfo));
  J_WARN_UNUSED_RESULT JMETHOD(void_result_t, predict_process_restart, (j_decompress_ptr cinfo));

  /* It is useful to allow each component to have a separate undiff method. */
  predict_undifference_method_ptr predict_undifference[MAX_COMPONENTS];

  /* Pointer to data which is private to predictor module */
  void *pred_private;

  /* Sample scaling */
  JMETHOD(void, scaler_start_pass, (j_decompress_ptr cinfo));
  JMETHOD(void, scaler_scale, (j_decompress_ptr cinfo,
			       JDIFFROW diff_buf, JSAMPROW output_buf,
			       JDIMENSION width));

  /* Pointer to data which is private to scaler module */
  void *scaler_private;

} jpeg_lossless_d_codec;

typedef jpeg_lossless_d_codec * j_lossless_d_ptr;


/* Compression module initialization routines */
J_WARN_UNUSED_RESULT EXTERN(void_result_t) jinit_lossless_c_codec JPP((j_compress_ptr cinfo));
J_WARN_UNUSED_RESULT EXTERN(void_result_t) jinit_lhuff_encoder JPP((j_compress_ptr cinfo));
J_WARN_UNUSED_RESULT EXTERN(void_result_t) jinit_differencer JPP((j_compress_ptr cinfo));
EXTERN(void) jinit_c_scaler JPP((j_compress_ptr cinfo));
/* Decompression module initialization routines */
J_WARN_UNUSED_RESULT EXTERN(void_result_t) jinit_lossless_d_codec JPP((j_decompress_ptr cinfo));
J_WARN_UNUSED_RESULT EXTERN(void_result_t) jinit_lhuff_decoder JPP((j_decompress_ptr cinfo));
EXTERN(void) jinit_undifferencer JPP((j_decompress_ptr cinfo));
J_WARN_UNUSED_RESULT EXTERN(void_result_t) jinit_d_scaler JPP((j_decompress_ptr cinfo));

#endif /* JLOSSLS_H */
