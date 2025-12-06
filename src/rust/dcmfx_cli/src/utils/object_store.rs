use object_store::{ObjectStore, aws::AmazonS3Builder};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;

/// Parses a URL that uses one of the supported output schemes: `file://``,
/// `s3://`, `gs://`, or `az://`.
///
/// On success, returns a store and a path to the object in that store. The
/// stores are reused wherever possible.
///
/// Returns an error if no recognized scheme is detected in the URL.
///
#[allow(clippy::result_unit_err)]
pub async fn object_url_to_store_and_path(
  url: &str,
) -> Result<(Arc<dyn object_store::ObjectStore>, String), ()> {
  if let Some(path) = url.strip_prefix("file://") {
    let store = get_cached_store(ObjectStoreScheme::File, "").await;
    return Ok((store, path.strip_prefix("/").unwrap_or(path).into()));
  }

  let scheme = match &url[..5] {
    "s3://" => ObjectStoreScheme::AmazonS3,
    "gs://" => ObjectStoreScheme::GoogleCloudStorage,
    "az://" => ObjectStoreScheme::AzureBlobStorage,
    _ => return Err(()),
  };

  let url = &url[5..];

  let Some(idx) = url.find("/") else {
    return Err(());
  };

  if idx == 0 {
    return Err(());
  }

  let host = &url[..idx];

  let store = get_cached_store(scheme, host).await;
  let path = &url[idx..];
  let path = path.strip_prefix("/").unwrap_or(path);

  Ok((store, path.into()))
}

/// Converts. a relative or absoluate path on the local filesystem to an object
/// store and path pair. Internally this will use the file:// schema.
///
pub async fn local_path_to_store_and_path<P: AsRef<std::path::Path>>(
  path: P,
) -> (Arc<dyn object_store::ObjectStore>, String) {
  let normalized_path = format!(
    "file://{}",
    crate::utils::normalize_path(path.as_ref()).display()
  );

  let normalized_path = normalized_path.replace('\\', "/");

  object_url_to_store_and_path(&normalized_path)
    .await
    .unwrap()
}

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
enum ObjectStoreScheme {
  File,
  AmazonS3,
  GoogleCloudStorage,
  AzureBlobStorage,
}

type StoreCacheHash =
  HashMap<(ObjectStoreScheme, String), Arc<dyn ObjectStore>>;

static STORE_CACHE: LazyLock<Arc<Mutex<StoreCacheHash>>> =
  LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

async fn get_cached_store(
  scheme: ObjectStoreScheme,
  host: &str,
) -> Arc<dyn ObjectStore> {
  let mut store_cache = STORE_CACHE.lock().await;

  let key = (scheme, host.to_string());

  if let Some(store) = store_cache.get(&key) {
    return store.clone();
  }

  let store = match scheme {
    ObjectStoreScheme::File => {
      Arc::new(object_store::local::LocalFileSystem::new())
        as Arc<dyn ObjectStore>
    }

    ObjectStoreScheme::AmazonS3 => {
      let mut builder = AmazonS3Builder::from_env().with_bucket_name(host);

      // If any of the following three environment variables are missing then
      // do a full AWS credential evaluation to determine them
      if std::env::var("AWS_REGION").is_err()
        || std::env::var("AWS_ACCESS_KEY_ID").is_err()
        || std::env::var("AWS_SECRET_ACCESS_KEY").is_err()
      {
        let (credentials, region) = aws::get_credentials_and_region().await;

        builder = builder
          .with_access_key_id(credentials.access_key_id())
          .with_secret_access_key(credentials.secret_access_key())
          .with_region(region.to_string());

        if let Some(token) = credentials.session_token() {
          builder = builder.with_token(token);
        }
      }

      // If the endpoint is being set to an http:// URL then explicitly allow
      // the use of HTTP. This is necessary for endpoints such as LocalStack,
      // which don't use HTTPS.
      if let Some(endpoint) = builder
        .get_config_value(&object_store::aws::AmazonS3ConfigKey::Endpoint)
        && endpoint.starts_with("http://")
      {
        builder = builder.with_allow_http(true);
      }

      Arc::new(builder.build().unwrap())
    }

    ObjectStoreScheme::GoogleCloudStorage => Arc::new(
      object_store::gcp::GoogleCloudStorageBuilder::from_env()
        .with_bucket_name(host)
        .build()
        .unwrap(),
    ),

    ObjectStoreScheme::AzureBlobStorage => Arc::new(
      object_store::azure::MicrosoftAzureBuilder::from_env()
        .with_container_name(host)
        .build()
        .unwrap(),
    ),
  };

  store_cache.insert(key, store.clone());

  store
}

mod aws {
  use aws_config::{BehaviorVersion, Region};
  use aws_credential_types::Credentials;
  use aws_sdk_sso::config::ProvideCredentials;

  pub async fn get_credentials_and_region() -> (Credentials, Region) {
    let sdk_config = aws_config::load_defaults(BehaviorVersion::latest()).await;

    let Some(region) = sdk_config.region() else {
      crate::utils::exit_with_error("AWS config does not specify a region", "");
    };

    let credentials = sdk_config
      .credentials_provider()
      .unwrap()
      .provide_credentials()
      .await;

    match credentials {
      Ok(credentials) => (credentials, region.clone()),

      Err(e) => {
        crate::utils::exit_with_error(
          "Failed getting AWS credentials",
          format!("{:?}", e),
        );
      }
    }
  }
}
