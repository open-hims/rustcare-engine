//! OAuth 2.0 authentication module for RustCare Engine
//! 
//! This module provides both OAuth client and provider functionality:
//! - OAuth 2.0 client for integrating with external providers (Google, Microsoft, etc.)
//! - OAuth 2.0 provider for allowing third-party applications to authenticate
//! - Support for Authorization Code, Client Credentials, and Refresh Token flows
//! - PKCE (Proof Key for Code Exchange) support for enhanced security
//! 
//! # Example
//! 
//! ```rust
//! use auth_oauth::{OAuthProvider, OAuthClient};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // As OAuth Provider
//!     let provider = OAuthProvider::new().await?;
//!     let auth_url = provider.generate_authorization_url("client_id", "redirect_uri").await?;
//!     
//!     // As OAuth Client
//!     let client = OAuthClient::new("google").await?;
//!     let token = client.exchange_code("authorization_code").await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod provider;
pub mod models;
pub mod error;
pub mod handlers;

pub use provider::*;
pub use models::*;
pub use error::*;