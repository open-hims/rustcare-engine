pub mod traits;

#[cfg(feature = "aws-kms")]
pub mod aws;

#[cfg(feature = "vault-kms")]
pub mod vault;

pub use traits::{KeyManagementService, KmsResult, KeyMetadata, KeyRotationPolicy};

#[cfg(feature = "aws-kms")]
pub use aws::AwsKmsProvider;

#[cfg(feature = "vault-kms")]
pub use vault::VaultKmsProvider;
