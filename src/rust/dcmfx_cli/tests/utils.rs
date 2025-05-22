use assert_cmd::assert::Assert;
use rand::Rng;

#[allow(dead_code)]
pub fn to_native_path(path: &str) -> String {
  #[cfg(windows)]
  return path.replace("/", "\\");

  #[cfg(not(windows))]
  return path.to_string();
}

#[allow(dead_code)]
pub fn generate_temp_filename() -> std::path::PathBuf {
  let temp_dir = std::env::temp_dir();

  let mut rng = rand::rng();
  let random_suffix: String = (0..16)
    .map(|_| char::from(rng.sample(rand::distr::Alphanumeric)))
    .collect();

  let file_name = format!("dcmfx_{}", random_suffix);
  temp_dir.join(file_name)
}

#[allow(dead_code)]
pub fn get_stdout(assert: Assert) -> String {
  String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

#[allow(dead_code)]
pub fn get_stderr(assert: Assert) -> String {
  String::from_utf8(assert.get_output().stderr.clone()).unwrap()
}
