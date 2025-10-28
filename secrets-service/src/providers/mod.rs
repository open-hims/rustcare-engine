//! Secret provider implementations

pub mod vault;
pub mod aws;

pub use vault::VaultProvider;
pub use aws::AwsSecretsManagerProvider;
