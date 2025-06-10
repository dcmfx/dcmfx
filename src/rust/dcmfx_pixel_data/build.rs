fn main() {
  build_libjpeg_12bit();
  build_openjpeg();

  if !std::env::var("TARGET").unwrap().contains("wasm") {
    build_charls();
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
  println!("cargo::rustc-link-search=native={}", out_dir);
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
    &["vendor/charls_2.4.2/charls_interface.c"],
    &["vendor/charls_2.4.2/include"],
    &[("CHARLS_STATIC", "1")],
    &[],
    "dcmfx_pixel_data_charls_c",
  );

  compile(
    &[
      "vendor/charls_2.4.2/src/charls_jpegls_decoder.cpp",
      "vendor/charls_2.4.2/src/charls_jpegls_encoder.cpp",
      "vendor/charls_2.4.2/src/jpeg_stream_reader.cpp",
      "vendor/charls_2.4.2/src/jpeg_stream_writer.cpp",
      "vendor/charls_2.4.2/src/jpegls.cpp",
      "vendor/charls_2.4.2/src/jpegls_error.cpp",
      "vendor/charls_2.4.2/src/validate_spiff_header.cpp",
      "vendor/charls_2.4.2/src/version.cpp",
    ],
    &["vendor/charls_2.4.2/include"],
    &[("CHARLS_STATIC", "1")],
    &[],
    "dcmfx_pixel_data_charls",
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
    .any(|f| f.ends_with(".cpp") || f.ends_with(".cxx"))
  {
    build.cpp(true);
    build.static_crt(true);

    // Target C++14
    if std::env::var("TARGET").unwrap().contains("msvc") {
      build.flag("/std:c++14");
    } else {
      build.flag("-std=c++14");
    }
  }

  // Optimize builds
  if std::env::var("PROFILE").unwrap() == "release" {
    build.define("NDEBUG", "1");
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

  // Silence build warnings on GCC/Clang
  if !std::env::var("TARGET").unwrap().contains("msvc") {
    build.flag("-Wno-unused-but-set-variable");
    build.flag("-Wno-unused-parameter");
    build.flag("-Wno-implicit-fallthrough");
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
    println!("cargo:rerun-if-changed={}", src_file);
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
    if std::env::var("TARGET").unwrap().contains("msvc") {
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
