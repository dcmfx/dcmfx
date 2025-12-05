use std::path::PathBuf;

use assert_cmd::{
  cargo::cargo_bin_cmd,
  {Command, assert::Assert},
};
use tempfile::{NamedTempFile, TempDir};

#[allow(dead_code)]
pub fn dcmfx_cli() -> Command {
  let mut cmd = cargo_bin_cmd!("dcmfx_cli");

  // Set AWS environment variables for LocalStack S3 emulation
  cmd
    .env("AWS_ACCESS_KEY_ID", "test")
    .env("AWS_SECRET_ACCESS_KEY", "test")
    .env("AWS_REGION", "us-east-1")
    .env("AWS_ENDPOINT_URL", "http://localhost:4566");

  cmd
}

#[allow(dead_code)]
pub fn to_native_path(path: &str) -> String {
  #[cfg(windows)]
  return path.replace("/", "\\");

  #[cfg(not(windows))]
  return path.to_string();
}

fn temp_dir() -> PathBuf {
  if let Ok(t) = std::env::var("RUNNER_TEMP") {
    PathBuf::from(t)
  } else {
    std::env::temp_dir()
  }
}

#[allow(dead_code)]
pub fn create_temp_dir() -> TempDir {
  TempDir::new_in(temp_dir()).unwrap()
}

#[allow(dead_code)]
pub fn create_temp_file() -> NamedTempFile {
  NamedTempFile::new_in(temp_dir()).unwrap()
}

#[allow(dead_code)]
pub fn get_stdout(assert: Assert) -> String {
  String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

#[allow(dead_code)]
pub fn get_stderr(assert: Assert) -> String {
  String::from_utf8(assert.get_output().stderr.clone()).unwrap()
}

#[allow(dead_code)]
pub fn get_stdout_and_stderr(assert: Assert) -> (String, String) {
  (
    String::from_utf8(assert.get_output().stdout.clone()).unwrap(),
    String::from_utf8(assert.get_output().stderr.clone()).unwrap(),
  )
}

#[allow(dead_code)]
pub async fn s3_get_object(key: &str) -> NamedTempFile {
  use object_store::{ObjectStore, path::Path as ObjectPath};

  let store = amazon_s3_store();

  let path = ObjectPath::from(key);
  let bytes = store.get(&path).await.unwrap().bytes().await.unwrap();

  let extension = PathBuf::from(key).extension().unwrap().to_os_string();
  let suffix = format!(".{}", extension.to_string_lossy());
  let temp_file = NamedTempFile::with_suffix_in(&suffix, temp_dir()).unwrap();
  std::fs::write(&temp_file.path(), &bytes).unwrap();

  temp_file
}

#[allow(dead_code)]
pub async fn s3_copy_object(src: &str, dst: &str) {
  use object_store::{ObjectStore, path::Path as ObjectPath};

  let store = amazon_s3_store();

  let src = ObjectPath::from(src);
  let dst = ObjectPath::from(dst);

  store.copy(&src, &dst).await.unwrap();
}

fn amazon_s3_store() -> object_store::aws::AmazonS3 {
  object_store::aws::AmazonS3Builder::new()
    .with_bucket_name("dcmfx-test")
    .with_access_key_id("test")
    .with_secret_access_key("test")
    .with_endpoint("http://localhost:4566")
    .with_allow_http(true)
    .build()
    .unwrap()
}
