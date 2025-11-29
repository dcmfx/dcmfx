use object_store::{
  ObjectStore, aws::AmazonS3Builder, path::Path as ObjectPath,
};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;
use url::Url;

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
) -> Result<(Arc<dyn object_store::ObjectStore>, ObjectPath), ()> {
  let Ok(url) = Url::parse(url) else {
    return Err(());
  };

  let path = ObjectPath::parse(url.path()).unwrap();

  if url.scheme() == "file" {
    let store = get_cached_store(ObjectStoreScheme::File, "").await;
    return Ok((store, path));
  }

  let Some(host) = url.host_str() else {
    return Err(());
  };

  if url.scheme() == "s3" {
    let store = get_cached_store(ObjectStoreScheme::AmazonS3, host).await;
    return Ok((store, path));
  }

  if url.scheme() == "gs" {
    let store =
      get_cached_store(ObjectStoreScheme::GoogleCloudStorage, host).await;
    return Ok((store, path));
  }

  if url.scheme() == "az" {
    let store =
      get_cached_store(ObjectStoreScheme::AzureBlobStorage, host).await;
    return Ok((store, path));
  }

  Err(())
}

/// Converts. a relative or absoluate path on the local filesystem to an object
/// store and path pair. Internally this will use the file:// schema.
///
pub async fn local_path_to_store_and_path<P: AsRef<std::path::Path>>(
  path: P,
) -> (Arc<dyn object_store::ObjectStore>, ObjectPath) {
  let normalized_path = format!(
    "file://{}",
    crate::utils::normalize_path(path.as_ref()).display()
  );

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
      use aws_sdk_sso::config::ProvideCredentials;

      // Get credentials based on what's configured in the environment
      let sdk_config =
        aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
      let aws_credentials = sdk_config
        .credentials_provider()
        .unwrap()
        .provide_credentials()
        .await;

      match aws_credentials {
        Ok(aws_credentials) => {
          let Some(region) = sdk_config.region() else {
            crate::utils::exit_with_error(
              "AWS region not provided by credentials chain",
              "",
            );
          };

          let mut builder = AmazonS3Builder::new()
            .with_bucket_name(host)
            .with_region(region.to_string())
            .with_access_key_id(aws_credentials.access_key_id())
            .with_secret_access_key(aws_credentials.secret_access_key());

          if let Ok(endpoint_url) = std::env::var("AWS_ENDPOINT_URL") {
            builder = builder.with_endpoint(endpoint_url);
          }

          if let Some(token) = aws_credentials.session_token() {
            builder = builder.with_token(token);
          };

          if let Ok(endpoint_url) = std::env::var("AWS_ENDPOINT_URL")
            && endpoint_url.starts_with("http://")
          {
            builder = builder.with_allow_http(true);
          }

          Arc::new(builder.build().unwrap())
        }

        Err(e) => {
          crate::utils::exit_with_error("Failed getting AWS credentials", e);
        }
      }
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
