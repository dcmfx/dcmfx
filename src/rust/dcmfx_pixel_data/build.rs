fn main() {
  if !cfg!(feature = "native") {
    return;
  }

  build_libjpeg_12bit();
  build_openjpeg();

  if !std::env::var("TARGET").unwrap().contains("wasm") {
    build_charls();
    build_libjxl();
    build_openjph();

    // Link the C++ standard library statically on windows-gnu targets
    if std::env::var("TARGET").unwrap().contains("windows-gnu") {
      println!("cargo:rustc-link-search=native=C:/msys64/mingw64/lib");
      println!("cargo:rustc-link-lib=static=stdc++");
    }
  }

  // Re-run build if any of the header files change
  let header_files: Vec<_> = glob::glob("vendor/**/*.h*")
    .unwrap()
    .filter_map(Result::ok)
    .collect();
  for file in header_files {
    println!("cargo:rerun-if-changed={}", file.to_string_lossy());
  }

  // Add output directory to the linker's search path
  let out_dir = std::env::var("OUT_DIR").unwrap();
  println!("cargo::rustc-link-search=native={out_dir}");
}

fn build_libjpeg_12bit() {
  compile(
    &[
      "vendor/libjpeg_12bit_6b/libjpeg_12bit_interface.c",
      "vendor/libjpeg_12bit_6b/src/jaricom.c",
      "vendor/libjpeg_12bit_6b/src/jcapimin.c",
      "vendor/libjpeg_12bit_6b/src/jcapistd.c",
      "vendor/libjpeg_12bit_6b/src/jcarith.c",
      "vendor/libjpeg_12bit_6b/src/jccoefct.c",
      "vendor/libjpeg_12bit_6b/src/jccolor.c",
      "vendor/libjpeg_12bit_6b/src/jcdctmgr.c",
      "vendor/libjpeg_12bit_6b/src/jcdiffct.c",
      "vendor/libjpeg_12bit_6b/src/jchuff.c",
      "vendor/libjpeg_12bit_6b/src/jcinit.c",
      "vendor/libjpeg_12bit_6b/src/jclhuff.c",
      "vendor/libjpeg_12bit_6b/src/jclossls.c",
      "vendor/libjpeg_12bit_6b/src/jclossy.c",
      "vendor/libjpeg_12bit_6b/src/jcmainct.c",
      "vendor/libjpeg_12bit_6b/src/jcmarker.c",
      "vendor/libjpeg_12bit_6b/src/jcmaster.c",
      "vendor/libjpeg_12bit_6b/src/jcodec.c",
      "vendor/libjpeg_12bit_6b/src/jcomapi.c",
      "vendor/libjpeg_12bit_6b/src/jcparam.c",
      "vendor/libjpeg_12bit_6b/src/jcphuff.c",
      "vendor/libjpeg_12bit_6b/src/jcpred.c",
      "vendor/libjpeg_12bit_6b/src/jcprepct.c",
      "vendor/libjpeg_12bit_6b/src/jcsample.c",
      "vendor/libjpeg_12bit_6b/src/jcscale.c",
      "vendor/libjpeg_12bit_6b/src/jcshuff.c",
      "vendor/libjpeg_12bit_6b/src/jctrans.c",
      "vendor/libjpeg_12bit_6b/src/jdapimin.c",
      "vendor/libjpeg_12bit_6b/src/jdapistd.c",
      "vendor/libjpeg_12bit_6b/src/jdarith.c",
      "vendor/libjpeg_12bit_6b/src/jdcoefct.c",
      "vendor/libjpeg_12bit_6b/src/jdcolor.c",
      "vendor/libjpeg_12bit_6b/src/jddctmgr.c",
      "vendor/libjpeg_12bit_6b/src/jddiffct.c",
      "vendor/libjpeg_12bit_6b/src/jdhuff.c",
      "vendor/libjpeg_12bit_6b/src/jdinput.c",
      "vendor/libjpeg_12bit_6b/src/jdlhuff.c",
      "vendor/libjpeg_12bit_6b/src/jdlossls.c",
      "vendor/libjpeg_12bit_6b/src/jdlossy.c",
      "vendor/libjpeg_12bit_6b/src/jdmainct.c",
      "vendor/libjpeg_12bit_6b/src/jdmarker.c",
      "vendor/libjpeg_12bit_6b/src/jdmaster.c",
      "vendor/libjpeg_12bit_6b/src/jdmerge.c",
      "vendor/libjpeg_12bit_6b/src/jdphuff.c",
      "vendor/libjpeg_12bit_6b/src/jdpostct.c",
      "vendor/libjpeg_12bit_6b/src/jdpred.c",
      "vendor/libjpeg_12bit_6b/src/jdsample.c",
      "vendor/libjpeg_12bit_6b/src/jdscale.c",
      "vendor/libjpeg_12bit_6b/src/jdshuff.c",
      "vendor/libjpeg_12bit_6b/src/jerror.c",
      "vendor/libjpeg_12bit_6b/src/jfdctflt.c",
      "vendor/libjpeg_12bit_6b/src/jfdctfst.c",
      "vendor/libjpeg_12bit_6b/src/jfdctint.c",
      "vendor/libjpeg_12bit_6b/src/jidctflt.c",
      "vendor/libjpeg_12bit_6b/src/jidctfst.c",
      "vendor/libjpeg_12bit_6b/src/jidctint.c",
      "vendor/libjpeg_12bit_6b/src/jidctred.c",
      "vendor/libjpeg_12bit_6b/src/jmemmgr.c",
      "vendor/libjpeg_12bit_6b/src/jmemnobs.c",
      "vendor/libjpeg_12bit_6b/src/jquant1.c",
      "vendor/libjpeg_12bit_6b/src/jquant2.c",
      "vendor/libjpeg_12bit_6b/src/jutils.c",
    ],
    &["vendor/libjpeg_12bit_6b"],
    &[],
    &[],
    "dcmfx_pixel_data_libjpeg_12bit",
  );
}

fn build_openjpeg() {
  compile(
    &[
      "vendor/openjpeg_2.5.3/openjpeg_interface.c",
      "vendor/openjpeg_2.5.3/src/bio.c",
      "vendor/openjpeg_2.5.3/src/cio.c",
      "vendor/openjpeg_2.5.3/src/dwt.c",
      "vendor/openjpeg_2.5.3/src/event.c",
      "vendor/openjpeg_2.5.3/src/function_list.c",
      "vendor/openjpeg_2.5.3/src/ht_dec.c",
      "vendor/openjpeg_2.5.3/src/image.c",
      "vendor/openjpeg_2.5.3/src/invert.c",
      "vendor/openjpeg_2.5.3/src/j2k.c",
      "vendor/openjpeg_2.5.3/src/jp2.c",
      "vendor/openjpeg_2.5.3/src/mct.c",
      "vendor/openjpeg_2.5.3/src/mqc.c",
      "vendor/openjpeg_2.5.3/src/openjpeg.c",
      "vendor/openjpeg_2.5.3/src/opj_clock.c",
      "vendor/openjpeg_2.5.3/src/opj_malloc.c",
      "vendor/openjpeg_2.5.3/src/pi.c",
      "vendor/openjpeg_2.5.3/src/sparse_array.c",
      "vendor/openjpeg_2.5.3/src/t1.c",
      "vendor/openjpeg_2.5.3/src/t1_ht_generate_luts.c",
      "vendor/openjpeg_2.5.3/src/t2.c",
      "vendor/openjpeg_2.5.3/src/tcd.c",
      "vendor/openjpeg_2.5.3/src/tgt.c",
      "vendor/openjpeg_2.5.3/src/thread.c",
    ],
    &["vendor/openjpeg_2.5.3/src"],
    &[("OPJ_STATIC", "1")],
    &[],
    "dcmfx_pixel_data_openjpeg",
  );
}

fn build_charls() {
  compile(
    &[
      "vendor/charls_2.4.2/charls_interface.cpp",
      "vendor/charls_2.4.2/src/charls_jpegls_decoder.cpp",
      "vendor/charls_2.4.2/src/charls_jpegls_encoder.cpp",
      "vendor/charls_2.4.2/src/jpeg_stream_reader.cpp",
      "vendor/charls_2.4.2/src/jpeg_stream_writer.cpp",
      "vendor/charls_2.4.2/src/jpegls_error.cpp",
      "vendor/charls_2.4.2/src/jpegls.cpp",
      "vendor/charls_2.4.2/src/validate_spiff_header.cpp",
      "vendor/charls_2.4.2/src/version.cpp",
    ],
    &["vendor/charls_2.4.2/include"],
    &[("CHARLS_STATIC", "1")],
    &[],
    "dcmfx_pixel_data_charls",
  );
}

fn build_libjxl() {
  compile(
    &[
      "vendor/libjxl_0.11.1/lib/jxl/ac_strategy.cc",
      "vendor/libjxl_0.11.1/lib/jxl/alpha.cc",
      "vendor/libjxl_0.11.1/lib/jxl/ans_common.cc",
      "vendor/libjxl_0.11.1/lib/jxl/blending.cc",
      "vendor/libjxl_0.11.1/lib/jxl/box_content_decoder.cc",
      "vendor/libjxl_0.11.1/lib/jxl/butteraugli/butteraugli.cc",
      "vendor/libjxl_0.11.1/lib/jxl/chroma_from_luma.cc",
      "vendor/libjxl_0.11.1/lib/jxl/cms/jxl_cms.cc",
      "vendor/libjxl_0.11.1/lib/jxl/coeff_order.cc",
      "vendor/libjxl_0.11.1/lib/jxl/color_encoding_internal.cc",
      "vendor/libjxl_0.11.1/lib/jxl/compressed_dc.cc",
      "vendor/libjxl_0.11.1/lib/jxl/convolve_separable5.cc",
      "vendor/libjxl_0.11.1/lib/jxl/convolve_slow.cc",
      "vendor/libjxl_0.11.1/lib/jxl/convolve_symmetric3.cc",
      "vendor/libjxl_0.11.1/lib/jxl/convolve_symmetric5.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_ans.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_cache.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_context_map.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_external_image.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_frame.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_group_border.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_group.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_huffman.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_modular.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_noise.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_patch_dictionary.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_transforms_testonly.cc",
      "vendor/libjxl_0.11.1/lib/jxl/dec_xyb.cc",
      "vendor/libjxl_0.11.1/lib/jxl/decode_to_jpeg.cc",
      "vendor/libjxl_0.11.1/lib/jxl/decode.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_ac_strategy.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_adaptive_quantization.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_ans.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_aux_out.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_bit_writer.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_butteraugli_comparator.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_cache.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_chroma_from_luma.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_cluster.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_coeff_order.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_comparator.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_context_map.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_debug_image.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_detect_dots.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_dot_dictionary.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_entropy_coder.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_external_image.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_fast_lossless.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_fields.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_frame.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_gaborish.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_group.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_heuristics.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_huffman_tree.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_huffman.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_icc_codec.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_image_bundle.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_linalg.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_modular.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_noise.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_patch_dictionary.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_photon_noise.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_progressive_split.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_quant_weights.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_splines.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_toc.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_transforms.cc",
      "vendor/libjxl_0.11.1/lib/jxl/enc_xyb.cc",
      "vendor/libjxl_0.11.1/lib/jxl/encode.cc",
      "vendor/libjxl_0.11.1/lib/jxl/entropy_coder.cc",
      "vendor/libjxl_0.11.1/lib/jxl/epf.cc",
      "vendor/libjxl_0.11.1/lib/jxl/fields.cc",
      "vendor/libjxl_0.11.1/lib/jxl/frame_header.cc",
      "vendor/libjxl_0.11.1/lib/jxl/headers.cc",
      "vendor/libjxl_0.11.1/lib/jxl/huffman_table.cc",
      "vendor/libjxl_0.11.1/lib/jxl/icc_codec_common.cc",
      "vendor/libjxl_0.11.1/lib/jxl/icc_codec.cc",
      "vendor/libjxl_0.11.1/lib/jxl/image_bundle.cc",
      "vendor/libjxl_0.11.1/lib/jxl/image_metadata.cc",
      "vendor/libjxl_0.11.1/lib/jxl/image_ops.cc",
      "vendor/libjxl_0.11.1/lib/jxl/image.cc",
      "vendor/libjxl_0.11.1/lib/jxl/jpeg/dec_jpeg_data_writer.cc",
      "vendor/libjxl_0.11.1/lib/jxl/jpeg/dec_jpeg_data.cc",
      "vendor/libjxl_0.11.1/lib/jxl/jpeg/enc_jpeg_data_reader.cc",
      "vendor/libjxl_0.11.1/lib/jxl/jpeg/enc_jpeg_data.cc",
      "vendor/libjxl_0.11.1/lib/jxl/jpeg/enc_jpeg_huffman_decode.cc",
      "vendor/libjxl_0.11.1/lib/jxl/jpeg/jpeg_data.cc",
      "vendor/libjxl_0.11.1/lib/jxl/loop_filter.cc",
      "vendor/libjxl_0.11.1/lib/jxl/luminance.cc",
      "vendor/libjxl_0.11.1/lib/jxl/memory_manager_internal.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/encoding/dec_ma.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/encoding/enc_debug_tree.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/encoding/enc_encoding.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/encoding/enc_ma.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/encoding/encoding.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/modular_image.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/transform/enc_palette.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/transform/enc_rct.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/transform/enc_squeeze.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/transform/enc_transform.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/transform/palette.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/transform/rct.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/transform/squeeze.cc",
      "vendor/libjxl_0.11.1/lib/jxl/modular/transform/transform.cc",
      "vendor/libjxl_0.11.1/lib/jxl/opsin_params.cc",
      "vendor/libjxl_0.11.1/lib/jxl/passes_state.cc",
      "vendor/libjxl_0.11.1/lib/jxl/quant_weights.cc",
      "vendor/libjxl_0.11.1/lib/jxl/quantizer.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/low_memory_render_pipeline.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/render_pipeline.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/simple_render_pipeline.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_blending.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_chroma_upsampling.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_cms.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_epf.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_from_linear.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_gaborish.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_noise.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_patches.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_splines.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_spot.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_to_linear.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_tone_mapping.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_upsampling.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_write.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_xyb.cc",
      "vendor/libjxl_0.11.1/lib/jxl/render_pipeline/stage_ycbcr.cc",
      "vendor/libjxl_0.11.1/lib/jxl/simd_util.cc",
      "vendor/libjxl_0.11.1/lib/jxl/splines.cc",
      "vendor/libjxl_0.11.1/lib/jxl/toc.cc",
      "vendor/libjxl_0.11.1/lib/threads/resizable_parallel_runner.cc",
      "vendor/libjxl_0.11.1/lib/threads/thread_parallel_runner_internal.cc",
      "vendor/libjxl_0.11.1/lib/threads/thread_parallel_runner.cc",
      "vendor/libjxl_0.11.1/libjxl_interface.cpp",
    ],
    &[
      "vendor/libjxl_0.11.1",
      "vendor/libjxl_0.11.1/build/lib/include",
      "vendor/libjxl_0.11.1/lib/include",
      "vendor/libjxl_0.11.1/third_party/brotli/c/include",
      "vendor/libjxl_0.11.1/third_party/highway",
      "vendor/libjxl_0.11.1/third_party/lcms/include",
    ],
    &[
      ("JXL_STATIC_DEFINE", "1"),
      ("JXL_THREADS_STATIC_DEFINE", "1"),
      ("JXL_CMS_STATIC_DEFINE", "1"),
      ("CMS_NO_REGISTER_KEYWORD", "1"),
    ],
    &[],
    "dcmfx_pixel_data_libjxl",
  );

  compile(
    &[
      "vendor/libjxl_0.11.1/third_party/brotli/c/common/constants.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/common/context.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/common/dictionary.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/common/platform.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/common/shared_dictionary.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/common/transform.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/dec/bit_reader.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/dec/decode.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/dec/huffman.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/dec/state.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/backward_references_hq.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/backward_references.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/bit_cost.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/block_splitter.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/brotli_bit_stream.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/cluster.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/command.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/compound_dictionary.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/compress_fragment_two_pass.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/compress_fragment.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/dictionary_hash.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/encode.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/encoder_dict.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/entropy_encode.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/fast_log.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/histogram.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/literal_cost.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/memory.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/metablock.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/static_dict.c",
      "vendor/libjxl_0.11.1/third_party/brotli/c/enc/utf8_util.c",
    ],
    &["vendor/libjxl_0.11.1/third_party/brotli/c/include"],
    &[],
    &[],
    "dcmfx_pixel_data_libjxl_brotli",
  );

  compile(
    &[
      "vendor/libjxl_0.11.1/third_party/highway/hwy/abort.cc",
      "vendor/libjxl_0.11.1/third_party/highway/hwy/aligned_allocator.cc",
      "vendor/libjxl_0.11.1/third_party/highway/hwy/nanobenchmark.cc",
      "vendor/libjxl_0.11.1/third_party/highway/hwy/per_target.cc",
      "vendor/libjxl_0.11.1/third_party/highway/hwy/print.cc",
      "vendor/libjxl_0.11.1/third_party/highway/hwy/stats.cc",
      "vendor/libjxl_0.11.1/third_party/highway/hwy/targets.cc",
      "vendor/libjxl_0.11.1/third_party/highway/hwy/timer.cc",
    ],
    &["vendor/libjxl_0.11.1/third_party/highway"],
    &[],
    &[],
    "dcmfx_pixel_data_libjxl_highway",
  );

  compile(
    &[
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsalpha.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmscam02.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmscgats.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmscnvrt.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmserr.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsgamma.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsgmt.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmshalf.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsintrp.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsio0.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsio1.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmslut.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsmd5.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsmtrx.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsnamed.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsopt.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmspack.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmspcs.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsplugin.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsps2.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmssamp.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmssm.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmstypes.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsvirt.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmswtpnt.c",
      "vendor/libjxl_0.11.1/third_party/lcms/src/cmsxform.c",
    ],
    &["vendor/libjxl_0.11.1/third_party/lcms/include"],
    &[],
    &[],
    "dcmfx_pixel_data_libjxl_lcms",
  );
}

fn build_openjph() {
  compile(
    &[
      "vendor/openjph_0.21.3/openjph_interface.cpp",
      "vendor/openjph_0.21.3/src/codestream/ojph_codeblock_fun.cpp",
      "vendor/openjph_0.21.3/src/codestream/ojph_codeblock.cpp",
      "vendor/openjph_0.21.3/src/codestream/ojph_codestream_gen.cpp",
      "vendor/openjph_0.21.3/src/codestream/ojph_codestream_local.cpp",
      "vendor/openjph_0.21.3/src/codestream/ojph_codestream.cpp",
      "vendor/openjph_0.21.3/src/codestream/ojph_params.cpp",
      "vendor/openjph_0.21.3/src/codestream/ojph_precinct.cpp",
      "vendor/openjph_0.21.3/src/codestream/ojph_resolution.cpp",
      "vendor/openjph_0.21.3/src/codestream/ojph_subband.cpp",
      "vendor/openjph_0.21.3/src/codestream/ojph_tile_comp.cpp",
      "vendor/openjph_0.21.3/src/codestream/ojph_tile.cpp",
      "vendor/openjph_0.21.3/src/coding/ojph_block_common.cpp",
      "vendor/openjph_0.21.3/src/coding/ojph_block_decoder32.cpp",
      "vendor/openjph_0.21.3/src/coding/ojph_block_decoder64.cpp",
      "vendor/openjph_0.21.3/src/coding/ojph_block_encoder.cpp",
      "vendor/openjph_0.21.3/src/others/ojph_arch.cpp",
      "vendor/openjph_0.21.3/src/others/ojph_file.cpp",
      "vendor/openjph_0.21.3/src/others/ojph_mem.cpp",
      "vendor/openjph_0.21.3/src/others/ojph_message.cpp",
      "vendor/openjph_0.21.3/src/transform/ojph_colour.cpp",
      "vendor/openjph_0.21.3/src/transform/ojph_transform.cpp",
    ],
    &["vendor/openjph_0.21.3/src/common"],
    &[],
    &[],
    "dcmfx_pixel_data_openjph",
  );

  if std::env::var("CARGO_CFG_TARGET_ARCH").unwrap() == "x86_64" {
    compile(
      &[
        "vendor/openjph_0.21.3/src/codestream/ojph_codestream_avx.cpp",
        "vendor/openjph_0.21.3/src/transform/ojph_colour_avx.cpp",
        "vendor/openjph_0.21.3/src/transform/ojph_transform_avx.cpp",
      ],
      &["vendor/openjph_0.21.3/src/common"],
      &[],
      &[BuildFlag::ArchitectureAVX],
      "dcmfx_pixel_data_openjph_avx",
    );

    compile(
      &[
        "vendor/openjph_0.21.3/src/codestream/ojph_codestream_avx2.cpp",
        "vendor/openjph_0.21.3/src/coding/ojph_block_encoder_avx2.cpp",
        "vendor/openjph_0.21.3/src/coding/ojph_block_decoder_avx2.cpp",
        "vendor/openjph_0.21.3/src/transform/ojph_colour_avx2.cpp",
        "vendor/openjph_0.21.3/src/transform/ojph_transform_avx2.cpp",
      ],
      &["vendor/openjph_0.21.3/src/common"],
      &[],
      &[BuildFlag::ArchitectureAVX2],
      "dcmfx_pixel_data_openjph_avx2",
    );

    compile(
      &[
        "vendor/openjph_0.21.3/src/coding/ojph_block_encoder_avx512.cpp",
        "vendor/openjph_0.21.3/src/transform/ojph_transform_avx512.cpp",
      ],
      &["vendor/openjph_0.21.3/src/common"],
      &[],
      &[BuildFlag::ArchitectureAVX512],
      "dcmfx_pixel_data_openjph_avx512",
    );

    compile(
      &[
        "vendor/openjph_0.21.3/src/codestream/ojph_codestream_sse.cpp",
        "vendor/openjph_0.21.3/src/transform/ojph_colour_sse.cpp",
        "vendor/openjph_0.21.3/src/transform/ojph_transform_sse.cpp",
      ],
      &["vendor/openjph_0.21.3/src/common"],
      &[],
      &[BuildFlag::ArchitectureSSE],
      "dcmfx_pixel_data_openjph_sse",
    );

    compile(
      &[
        "vendor/openjph_0.21.3/src/codestream/ojph_codestream_sse2.cpp",
        "vendor/openjph_0.21.3/src/transform/ojph_colour_sse2.cpp",
        "vendor/openjph_0.21.3/src/transform/ojph_transform_sse2.cpp",
      ],
      &["vendor/openjph_0.21.3/src/common"],
      &[],
      &[BuildFlag::ArchitectureSSE2],
      "dcmfx_pixel_data_openjph_sse2",
    );

    compile(
      &["vendor/openjph_0.21.3/src/coding/ojph_block_decoder_ssse3.cpp"],
      &["vendor/openjph_0.21.3/src/common"],
      &[],
      &[BuildFlag::ArchitectureSSSE3],
      "dcmfx_pixel_data_openjph_ssse3",
    );
  }
}

fn compile(
  src_files: &[&str],
  include_paths: &[&str],
  defines: &[(&str, &str)],
  build_flags: &[BuildFlag],
  output: &str,
) {
  let mut build = cc::Build::new();

  // Enable C++ support if there are any C++ source files
  if src_files
    .iter()
    .any(|f| f.ends_with(".cpp") || f.ends_with(".cxx") || f.ends_with(".cc"))
  {
    build.cpp(true);
    build.static_crt(true);

    // Target C++17
    if is_msvc() {
      build.flag("/std:c++17");
    } else {
      build.flag("-std=c++17");
    }

    // Enable exception handling on MSVC
    if is_msvc() {
      build.flag("/EHsc");
    }
  }

  // Disable warnings
  build.warnings(false);
  if is_msvc() {
    build.define("_CRT_SECURE_NO_WARNINGS", "1");
  }

  let is_release = std::env::var("PROFILE").unwrap() == "release";

  // Remove asserts and debug code from release builds, and also from WASM
  // builds because assert() doesn't work on that target
  if is_release || std::env::var("TARGET").unwrap().contains("wasm") {
    build.define("NDEBUG", "1");
  }

  // Optimize release builds
  if is_release {
    build.opt_level(3);
  }

  // Add build flags
  for build_flag in build_flags {
    for flag in build_flag.compiler_flags() {
      build.flag(flag);
    }
  }

  // Add include paths
  for include_path in include_paths {
    build.include(include_path);
  }

  // Add preprocessor defines
  for define in defines {
    build.define(define.0, define.1);
  }

  // When targeting WASM, add OpenBSD libc include path
  if let Some(libc) =
    std::env::var_os("DEP_WASM32_UNKNOWN_UNKNOWN_OPENBSD_LIBC_INCLUDE")
  {
    build.include(libc);
    println!("cargo::rustc-link-lib=wasm32-unknown-unknown-openbsd-libc");
  }

  // Re-run if any source file changes
  for src_file in src_files {
    println!("cargo:rerun-if-changed={src_file}");
  }

  build.files(src_files);
  build.compile(output);
}

enum BuildFlag {
  ArchitectureAVX,
  ArchitectureAVX2,
  ArchitectureAVX512,
  ArchitectureSSE,
  ArchitectureSSE2,
  ArchitectureSSSE3,
}

impl BuildFlag {
  fn compiler_flags(&self) -> &[&str] {
    if is_msvc() {
      match self {
        Self::ArchitectureAVX => &["/arch:AVX"],
        Self::ArchitectureAVX2 => &["/arch:AVX2"],
        Self::ArchitectureAVX512 => &["/arch:AVX512"],
        Self::ArchitectureSSE => &[],
        Self::ArchitectureSSE2 => &[],
        Self::ArchitectureSSSE3 => &["/arch:AVX"],
      }
    } else {
      match self {
        Self::ArchitectureAVX => &["-mavx"],
        Self::ArchitectureAVX2 => &["-mavx2"],
        Self::ArchitectureAVX512 => &["-mavx512f", "-mavx512cd"],
        Self::ArchitectureSSE => &["-msse"],
        Self::ArchitectureSSE2 => &["-msse2"],
        Self::ArchitectureSSSE3 => &["-mssse3"],
      }
    }
  }
}

fn is_msvc() -> bool {
  std::env::var("TARGET").unwrap().contains("msvc")
}
