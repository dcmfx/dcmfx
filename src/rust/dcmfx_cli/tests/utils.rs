use rand::Rng;

pub fn to_native_path(path: &str) -> String {
  #[cfg(windows)]
  return path.replace("/", "\\");

  #[cfg(not(windows))]
  return path.to_string();
}

pub fn generate_temp_filename() -> std::path::PathBuf {
  let temp_dir = std::env::temp_dir();

  let mut rng = rand::rng();
  let random_suffix: String = (0..16)
    .map(|_| char::from(rng.sample(rand::distr::Alphanumeric)))
    .collect();

  let file_name = format!("dcmfx_{}", random_suffix);
  temp_dir.join(file_name)
}
