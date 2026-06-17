//! Google Cloud Storage object store adapter for Tesela.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use reqwest::blocking::Client;
use serde::Deserialize;
use tesela_core::{Error, Value};
use tesela_runtime::ports::ObjectStore;
use tesela_runtime::query::{ArtifactLocator, ObjectMetadata, SignedUpload};

/// GCS object store configuration.
#[derive(Debug, Clone)]
pub struct GcsConfig {
    /// Bucket name.
    pub bucket: String,
    /// Optional OAuth access token. If absent, metadata server auth is used.
    pub access_token: Option<String>,
}

/// GCS-backed Tesela object store.
pub struct GcsObjectStore {
    config: GcsConfig,
    client: Client,
}

impl GcsObjectStore {
    /// Create a GCS object store.
    pub fn new(config: GcsConfig) -> Result<Arc<Self>, Error> {
        validate_bucket(&config.bucket)?;
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| Error::adapter_with_source("gcs client", error.to_string()))?;
        Ok(Arc::new(Self { config, client }))
    }

    /// Store bytes at `path`.
    pub fn put_object(&self, path: &str, bytes: &[u8], media_type: &str) -> Result<(), Error> {
        validate_path(path)?;
        let url = format!(
            "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
            self.config.bucket,
            url_component(path)
        );
        let response = self
            .client
            .post(url)
            .bearer_auth(self.token()?)
            .header(reqwest::header::CONTENT_TYPE, media_type)
            .body(bytes.to_vec())
            .send()
            .map_err(|error| Error::adapter_with_source("gcs upload", error.to_string()))?;
        if response.status().is_success() {
            return Ok(());
        }
        Err(Error::adapter(format!(
            "gcs upload failed: {} {}",
            response.status(),
            response.text().unwrap_or_default()
        )))
    }

    /// Read bytes from `path`.
    pub fn get_object(&self, path: &str) -> Result<Vec<u8>, Error> {
        validate_path(path)?;
        let url = format!(
            "https://storage.googleapis.com/storage/v1/b/{}/o/{}?alt=media",
            self.config.bucket,
            url_component(path)
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(self.token()?)
            .send()
            .map_err(|error| Error::adapter_with_source("gcs read", error.to_string()))?;
        if !response.status().is_success() {
            return Err(Error::not_found("gcs_object", path));
        }
        response
            .bytes()
            .map(|bytes| bytes.to_vec())
            .map_err(|error| Error::adapter_with_source("gcs read body", error.to_string()))
    }

    fn token(&self) -> Result<String, Error> {
        if let Some(token) = &self.config.access_token {
            return Ok(token.clone());
        }
        if let Ok(token) = std::env::var("GOOGLE_OAUTH_ACCESS_TOKEN") {
            return Ok(token);
        }
        let response = self
            .client
            .get("http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token")
            .header("Metadata-Flavor", "Google")
            .send()
            .map_err(|error| Error::unauthorized(format!("metadata token unavailable: {error}")))?;
        if !response.status().is_success() {
            return Err(Error::unauthorized(format!(
                "metadata token unavailable: {}",
                response.status()
            )));
        }
        response
            .json::<MetadataToken>()
            .map(|token| token.access_token)
            .map_err(|error| Error::unauthorized(format!("metadata token parse failed: {error}")))
    }
}

impl ObjectStore for GcsObjectStore {
    fn signed_upload_url(
        &self,
        path: &str,
        ttl_secs: u64,
        _metadata: &BTreeMap<String, Value>,
    ) -> Result<SignedUpload, Error> {
        validate_path(path)?;
        Ok(SignedUpload {
            url: format!("gcs://{}/{}", self.config.bucket, path),
            path: path.to_string(),
            expires_at: expires_at(ttl_secs),
            headers: BTreeMap::new(),
            upload_id: None,
        })
    }

    fn signed_read_url(
        &self,
        path: &str,
        ttl_secs: u64,
        _metadata: &BTreeMap<String, Value>,
    ) -> Result<ArtifactLocator, Error> {
        validate_path(path)?;
        Ok(ArtifactLocator {
            url: format!("gcs://{}/{}", self.config.bucket, path),
            path: path.to_string(),
            expires_at: expires_at(ttl_secs),
            media_type: None,
            headers: BTreeMap::new(),
            metadata: BTreeMap::new(),
        })
    }

    fn stat(&self, path: &str) -> Result<ObjectMetadata, Error> {
        validate_path(path)?;
        let url = format!(
            "https://storage.googleapis.com/storage/v1/b/{}/o/{}",
            self.config.bucket,
            url_component(path)
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(self.token()?)
            .send()
            .map_err(|error| Error::adapter_with_source("gcs stat", error.to_string()))?;
        if !response.status().is_success() {
            return Err(Error::not_found("gcs_object", path));
        }
        response
            .json::<GcsObject>()
            .map(ObjectMetadata::from)
            .map_err(|error| Error::adapter_with_source("gcs stat body", error.to_string()))
    }

    fn list(&self, prefix: &str) -> Result<Vec<ObjectMetadata>, Error> {
        validate_path(prefix)?;
        let url = format!(
            "https://storage.googleapis.com/storage/v1/b/{}/o?prefix={}",
            self.config.bucket,
            url_component(prefix)
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(self.token()?)
            .send()
            .map_err(|error| Error::adapter_with_source("gcs list", error.to_string()))?;
        if !response.status().is_success() {
            return Err(Error::adapter(format!(
                "gcs list failed: {} {}",
                response.status(),
                response.text().unwrap_or_default()
            )));
        }
        response
            .json::<GcsList>()
            .map(|list| list.items.into_iter().map(ObjectMetadata::from).collect())
            .map_err(|error| Error::adapter_with_source("gcs list body", error.to_string()))
    }

    fn delete(&self, path: &str) -> Result<(), Error> {
        validate_path(path)?;
        let url = format!(
            "https://storage.googleapis.com/storage/v1/b/{}/o/{}",
            self.config.bucket,
            url_component(path)
        );
        let response = self
            .client
            .delete(url)
            .bearer_auth(self.token()?)
            .send()
            .map_err(|error| Error::adapter_with_source("gcs delete", error.to_string()))?;
        if response.status().is_success() {
            return Ok(());
        }
        Err(Error::adapter(format!(
            "gcs delete failed: {} {}",
            response.status(),
            response.text().unwrap_or_default()
        )))
    }
}

#[derive(Debug, Deserialize)]
struct MetadataToken {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GcsList {
    #[serde(default)]
    items: Vec<GcsObject>,
}

#[derive(Debug, Deserialize)]
struct GcsObject {
    name: String,
    #[serde(rename = "size")]
    size: Option<String>,
    #[serde(rename = "contentType")]
    content_type: Option<String>,
    etag: Option<String>,
    #[serde(rename = "updated")]
    updated: Option<String>,
}

impl From<GcsObject> for ObjectMetadata {
    fn from(value: GcsObject) -> Self {
        Self {
            path: value.name,
            size_bytes: value.size.and_then(|size| size.parse().ok()),
            media_type: value.content_type,
            etag: value.etag,
            last_modified: value.updated,
            metadata: BTreeMap::new(),
        }
    }
}

fn expires_at(ttl_secs: u64) -> String {
    (Utc::now() + chrono::Duration::seconds(ttl_secs as i64)).to_rfc3339()
}

fn validate_bucket(bucket: &str) -> Result<(), Error> {
    if !bucket.is_empty()
        && bucket
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        return Ok(());
    }
    Err(Error::bad_request(format!("invalid gcs bucket: {bucket}")))
}

fn validate_path(path: &str) -> Result<(), Error> {
    if path.is_empty() || path.starts_with('/') || path.contains("..") {
        return Err(Error::bad_request(format!("invalid gcs path: {path}")));
    }
    Ok(())
}

fn url_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_component_encodes_slashes() {
        assert_eq!(url_component("a/b c"), "a%2Fb%20c");
    }

    #[test]
    fn rejects_unsafe_paths() {
        assert!(validate_path("/root").is_err());
        assert!(validate_path("../root").is_err());
        assert!(validate_path("layers/a.geojson").is_ok());
    }
}
