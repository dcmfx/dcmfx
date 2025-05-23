fn main() {
  build_c_code();

  if !std::env::var("TARGET").unwrap().contains("wasm") {
    build_cpp_code();
  }

  // Add output directory to the linker's search path
  let out_dir = std::env::var("OUT_DIR").unwrap();
  println!("cargo::rustc-link-search=native={}", out_dir);
}

fn build_c_code() {
  let mut build = cc::Build::new();

  shared_build_config(&mut build, "vendor/**/*.c", "vendor/**/*.h");
  build.include("vendor/charls_2.4.2/include");
  build.include("vendor/libjpeg_12bit_6b");
  build.include("vendor/openjpeg_2.5.3/src");
  build.define("CHARLS_STATIC", "1");
  build.define("OPJ_STATIC", "1");

  build.compile("dcmfx_pixel_data_c_libs");
}

fn build_cpp_code() {
  let mut build = cc::Build::new();

  build.cpp(true);

  shared_build_config(&mut build, "vendor/**/*.cpp", "vendor/**/*.hpp");
  build.include("vendor/charls_2.4.2/include");
  build.define("CHARLS_STATIC", "1");

  // Explicitly specify C++14 as this is what CharLS 2.x targets
  if !std::env::var("TARGET").unwrap().contains("msvc") {
    build.flag("-std=c++14");
  }

  build.compile("dcmfx_pixel_data_cpp_libs");

  // Link the C++ standard library statically on windows-gnu targets
  if std::env::var("TARGET").unwrap().contains("windows-gnu") {
    println!("cargo:rustc-link-search=native=C:/msys64/mingw64/lib");
    println!("cargo:rustc-link-lib=static=stdc++");
  }
}

fn shared_build_config(
  build: &mut cc::Build,
  glob_path: &str,
  header_glob_path: &str,
) {
  // Silence build warnings
  if !std::env::var("TARGET").unwrap().contains("msvc") {
    build.flag("-Wno-unused-but-set-variable");
    build.flag("-Wno-unused-parameter");
    build.flag("-Wno-implicit-fallthrough");
  }

  // Optimize builds
  build.define("NDEBUG", "1");
  build.opt_level(2);

  // When targeting WASM, add OpenBSD libc include path
  if let Some(libc) =
    std::env::var_os("DEP_WASM32_UNKNOWN_UNKNOWN_OPENBSD_LIBC_INCLUDE")
  {
    build.include(libc);
    println!("cargo::rustc-link-lib=wasm32-unknown-unknown-openbsd-libc");
  }

  // Glob for all the source files
  let src_files: Vec<_> = glob::glob(glob_path)
    .unwrap()
    .filter_map(Result::ok)
    .collect();

  // Re-run build if any of the source files change
  for file in src_files.iter() {
    println!("cargo:rerun-if-changed={}", file.to_string_lossy());
  }

  // Glob for all the header files
  let header_files: Vec<_> = glob::glob(header_glob_path)
    .unwrap()
    .filter_map(Result::ok)
    .collect();

  // Re-run build if any of the header files change
  for file in header_files {
    println!("cargo:rerun-if-changed={}", file.to_string_lossy());
  }

  build.files(src_files);
}
