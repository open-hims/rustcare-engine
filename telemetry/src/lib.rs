//! Comprehensive observability and telemetry platform for RustCare Engine
//! 
//! This module provides production-ready observability capabilities including:
//! - Distributed tracing with OpenTelemetry
//! - Metrics collection and Prometheus integration
//! - Structured logging with correlation IDs
//! - Health checks and service monitoring
//! - Performance profiling and bottleneck detection
//! - Error tracking and alerting
//! - Real-time dashboards and visualizations
//! - SLA/SLO monitoring and reporting
//! - Capacity planning and resource optimization
//! 
//! # Observability Pillars
//! 
//! - **Metrics**: Quantitative measurements (counters, gauges, histograms)
//! - **Logs**: Structured event records with context
//! - **Traces**: Request flow through distributed systems
//! - **Profiles**: CPU, memory, and performance analysis
//! - **Health**: Service availability and dependency monitoring
//! 
//! # Example
//! 
//! ```rust
//! use telemetry::{TelemetryEngine, MetricsCollector, TracingSystem};
//! use tracing::{info, instrument};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let telemetry = TelemetryEngine::new()
//!         .with_jaeger("http://localhost:14268/api/traces")
//!         .with_prometheus("0.0.0.0:9090")
//!         .with_structured_logging()
//!         .init()
//!         .await?;
//!     
//!     // Instrument functions
//!     process_request("user123").await?;
//!     
//!     // Manual metrics
//!     telemetry.counter("requests_total")
//!         .with_label("method", "GET")
//!         .increment();
//!     
//!     telemetry.histogram("request_duration_seconds")
//!         .record(0.045);
//!     
//!     Ok(())
//! }
//! 
//! #[instrument]
//! async fn process_request(user_id: &str) -> Result<(), Box<dyn std::error::Error>> {
//!     info!("Processing request for user: {}", user_id);
//!     // Business logic here
//!     Ok(())
//! }
//! ```

pub mod metrics;
pub mod tracing;
pub mod logging;
pub mod health;
pub mod alerts;
pub mod dashboard;
pub mod exporters;
pub mod collectors;
pub mod error;

pub use metrics::*;
pub use tracing::*;
pub use logging::*;
pub use health::*;
pub use error::*;