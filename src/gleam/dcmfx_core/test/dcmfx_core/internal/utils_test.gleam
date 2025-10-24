import dcmfx_core/internal/utils

pub fn trim_codepoints_test() {
  assert utils.trim_ascii("  \n234 ", 0x20) == "\n234"
}

pub fn trim_end_codepoints_test() {
  assert utils.trim_ascii_end("\n\n\n 234 \n\n", 0x0A) == "\n\n\n 234 "
}
