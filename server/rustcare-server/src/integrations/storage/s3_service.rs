//! MinIO S3 storage service for organization buckets
//!
//! Provides bucket management and file operations for organizations

use aws_config::BehaviorVersion;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{
    BucketLocationConstraint, CreateBucketConfiguration, Delete, ObjectIdentifier,
};
use aws_sdk_s3::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// S3 storage service configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct S3Config {
    pub endpoint: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub use_path_style: bool,
    pub use_ssl: bool,
}

impl S3Config {
    /// Load S3 configuration from environment variables
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            endpoint: std::env::var("S3_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:9000".to_string()),
            region: std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            access_key: std::env::var("S3_ACCESS_KEY")
                .unwrap_or_else(|_| "minioadmin".to_string()),
            secret_key: std::env::var("S3_SECRET_KEY")
                .unwrap_or_else(|_| "minioadmin".to_string()),
            use_path_style: std::env::var("S3_USE_PATH_STYLE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            use_ssl: std::env::var("S3_USE_SSL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(false),
        })
    }
}

/// S3 storage service for managing organization buckets
pub struct S3StorageService {
    client: Client,
    config: S3Config,
}

impl S3StorageService {
    /// Create a new S3 storage service
    pub async fn new(
        config: S3Config,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            endpoint = %config.endpoint,
            region = %config.region,
            "Initializing S3 storage service"
        );

        // Build AWS SDK config for MinIO
        let credentials = aws_sdk_s3::config::Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "rustcare-s3",
        );

        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(aws_config::Region::new(config.region.clone()))
            .load()
            .await;

        let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&sdk_config)
            .endpoint_url(&config.endpoint)
            .force_path_style(config.use_path_style);

        let client = Client::from_conf(s3_config_builder.build());

        info!("✅ S3 storage service initialized");

        Ok(Self { client, config })
    }

    /// Create a bucket for an organization
    pub async fn create_organization_bucket(
        &self,
        org_slug: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let bucket_name = format!("rustcare-org-{}", org_slug.to_lowercase());

        info!(
            bucket = %bucket_name,
            organization = %org_slug,
            "Creating organization bucket"
        );

        // Check if bucket already exists
        match self.client.head_bucket().bucket(&bucket_name).send().await {
            Ok(_) => {
                info!(
                    bucket = %bucket_name,
                    "Bucket already exists, skipping creation"
                );
                return Ok(bucket_name);
            }
            Err(_) => {
                debug!(bucket = %bucket_name, "Bucket does not exist, creating");
            }
        }

        // Create bucket configuration
        let constraint = BucketLocationConstraint::from(self.config.region.as_str());
        let cfg = CreateBucketConfiguration::builder()
            .location_constraint(constraint)
            .build();

        // Create the bucket
        match self
            .client
            .create_bucket()
            .bucket(&bucket_name)
            .create_bucket_configuration(cfg)
            .send()
            .await
        {
            Ok(_) => {
                info!(
                    bucket = %bucket_name,
                    "✅ Organization bucket created successfully"
                );
                Ok(bucket_name)
            }
            Err(e) => {
                error!(
                    bucket = %bucket_name,
                    error = %e,
                    "Failed to create organization bucket"
                );
                Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
        }
    }

    /// List all buckets
    pub async fn list_buckets(
        &self,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client.list_buckets().send().await?;

        let buckets = response
            .buckets()
            .iter()
            .filter_map(|b| b.name().map(|s| s.to_string()))
            .collect();

        Ok(buckets)
    }

    /// Check if a bucket exists
    pub async fn bucket_exists(&self, bucket_name: &str) -> bool {
        self.client
            .head_bucket()
            .bucket(bucket_name)
            .send()
            .await
            .is_ok()
    }

    /// Upload a file to a bucket
    pub async fn upload_file(
        &self,
        bucket_name: &str,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            bucket = %bucket_name,
            key = %key,
            size = data.len(),
            "Uploading file to S3"
        );

        let body = ByteStream::from(data);

        let mut request = self
            .client
            .put_object()
            .bucket(bucket_name)
            .key(key)
            .body(body);

        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }

        match request.send().await {
            Ok(output) => {
                let etag = output.e_tag().unwrap_or("unknown").to_string();
                info!(
                    bucket = %bucket_name,
                    key = %key,
                    etag = %etag,
                    "✅ File uploaded successfully"
                );
                Ok(etag)
            }
            Err(e) => {
                error!(
                    bucket = %bucket_name,
                    key = %key,
                    error = %e,
                    "Failed to upload file"
                );
                Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
        }
    }

    /// Download a file from a bucket
    pub async fn download_file(
        &self,
        bucket_name: &str,
        key: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            bucket = %bucket_name,
            key = %key,
            "Downloading file from S3"
        );

        let response = self
            .client
            .get_object()
            .bucket(bucket_name)
            .key(key)
            .send()
            .await?;

        let data = response.body.collect().await?.into_bytes().to_vec();

        info!(
            bucket = %bucket_name,
            key = %key,
            size = data.len(),
            "✅ File downloaded successfully"
        );

        Ok(data)
    }

    /// Delete a file from a bucket
    pub async fn delete_file(
        &self,
        bucket_name: &str,
        key: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            bucket = %bucket_name,
            key = %key,
            "Deleting file from S3"
        );

        self.client
            .delete_object()
            .bucket(bucket_name)
            .key(key)
            .send()
            .await?;

        info!(
            bucket = %bucket_name,
            key = %key,
            "✅ File deleted successfully"
        );

        Ok(())
    }

    /// List files in a bucket with optional prefix
    pub async fn list_files(
        &self,
        bucket_name: &str,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut request = self.client.list_objects_v2().bucket(bucket_name);

        if let Some(p) = prefix {
            request = request.prefix(p);
        }

        let response = request.send().await?;

        let files = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(|s| s.to_string()))
            .collect();

        Ok(files)
    }

    /// Delete a bucket (must be empty)
    pub async fn delete_bucket(
        &self,
        bucket_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        warn!(
            bucket = %bucket_name,
            "Deleting bucket (bucket must be empty)"
        );

        self.client
            .delete_bucket()
            .bucket(bucket_name)
            .send()
            .await?;

        info!(
            bucket = %bucket_name,
            "✅ Bucket deleted successfully"
        );

        Ok(())
    }

    /// Delete all files in a bucket (for cleanup)
    pub async fn empty_bucket(
        &self,
        bucket_name: &str,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        warn!(
            bucket = %bucket_name,
            "Emptying bucket - this will delete all files"
        );

        let files = self.list_files(bucket_name, None).await?;
        let file_count = files.len();

        if files.is_empty() {
            info!(bucket = %bucket_name, "Bucket is already empty");
            return Ok(0);
        }

        // Delete in batches of 1000 (S3 limit)
        for chunk in files.chunks(1000) {
            let objects: Vec<ObjectIdentifier> = chunk
                .iter()
                .map(|key| ObjectIdentifier::builder().key(key).build().unwrap())
                .collect();

            let delete = Delete::builder().set_objects(Some(objects)).build()?;

            self.client
                .delete_objects()
                .bucket(bucket_name)
                .delete(delete)
                .send()
                .await?;
        }

        info!(
            bucket = %bucket_name,
            files_deleted = file_count,
            "✅ Bucket emptied successfully"
        );

        Ok(file_count)
    }

    /// Get file metadata
    pub async fn get_file_metadata(
        &self,
        bucket_name: &str,
        key: &str,
    ) -> Result<FileMetadata, Box<dyn std::error::Error + Send + Sync>> {
        let response = self
            .client
            .head_object()
            .bucket(bucket_name)
            .key(key)
            .send()
            .await?;

        Ok(FileMetadata {
            size: response.content_length().unwrap_or(0) as u64,
            content_type: response.content_type().map(|s| s.to_string()),
            etag: response.e_tag().map(|s| s.to_string()),
            last_modified: response
                .last_modified()
                .and_then(|dt| dt.fmt(aws_smithy_types::date_time::Format::DateTime).ok()),
        })
    }
}

/// File metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub size: u64,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires MinIO running
    async fn test_s3_config_from_env() {
        std::env::set_var("S3_ENDPOINT", "http://localhost:9000");
        std::env::set_var("S3_REGION", "us-east-1");
        std::env::set_var("S3_ACCESS_KEY", "minioadmin");
        std::env::set_var("S3_SECRET_KEY", "minioadmin");

        let config = S3Config::from_env().unwrap();
        assert_eq!(config.endpoint, "http://localhost:9000");
        assert_eq!(config.region, "us-east-1");
    }
}
