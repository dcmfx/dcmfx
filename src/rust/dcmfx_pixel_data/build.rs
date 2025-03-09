fn main() {
  // Glob for all the .c files
  let c_files: Vec<_> = glob::glob("vendor/**/*.c")
    .unwrap()
    .filter_map(Result::ok)
    .collect();

  // Re-run build if any of the .c files change
  for file in c_files.iter() {
    println!("cargo:rerun-if-changed={}", file.to_string_lossy());
  }

  // Determine the compilation flag to hide build warnings
  let disable_warnings_flag =
    if std::env::var("TARGET").unwrap().contains("msvc") {
      "/w"
    } else {
      "-w"
    };

  // Prepare build
  let mut build = cc::Build::new();
  build
    .files(c_files)
    .flag(disable_warnings_flag)
    .opt_level(2)
    .flag("-DNDEBUG")
    .opt_level(2);

  // When targeting WASM, add OpenBSD libc include path
  if let Some(libc) =
    std::env::var_os("DEP_WASM32_UNKNOWN_UNKNOWN_OPENBSD_LIBC_INCLUDE")
  {
    build.include(libc);
    println!("cargo::rustc-link-lib=wasm32-unknown-unknown-openbsd-libc");
  }

  build.include("vendor/libjpeg_12bit_6b");
  build.include("vendor/openjpeg_2.5.3/src");

  build.compile("dcmfx_pixel_data_c_libs");

  // Add output directory to the linker's search path
  let out_dir = std::env::var("OUT_DIR").unwrap();
  println!("cargo::rustc-link-search=native={}", out_dir);
}
