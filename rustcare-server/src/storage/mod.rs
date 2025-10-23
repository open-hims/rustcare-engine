//! Storage services for RustCare Engine
//!
//! Provides S3-compatible storage using MinIO

pub mod s3_service;

pub use s3_service::S3StorageService;
