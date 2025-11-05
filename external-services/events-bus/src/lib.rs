//! Event-driven messaging system for RustCare Engine
//! 
//! This module provides a robust, scalable event bus implementation supporting:
//! - Publish/Subscribe patterns
//! - Event sourcing capabilities
//! - Multiple broker backends (Kafka, RabbitMQ, Redis, In-Memory)
//! - Guaranteed delivery and at-least-once semantics
//! - Dead letter queues for failed events
//! - Event replay and time travel debugging
//! - Schema evolution and versioning
//! 
//! # Event Types
//! 
//! - **Domain Events**: Business logic events (UserRegistered, OrderPlaced, etc.)
//! - **Integration Events**: Cross-service communication events
//! - **System Events**: Infrastructure and operational events
//! - **Audit Events**: Security and compliance tracking events
//! 
//! # Example
//! 
//! ```rust
//! use events_bus::{EventBus, Event, DomainEvent};
//! use serde_json::json;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let bus = EventBus::new().await?;
//!     
//!     // Publish an event
//!     let event = DomainEvent::new(
//!         "user.registered",
//!         json!({
//!             "user_id": "123",
//!             "email": "user@example.com"
//!         })
//!     );
//!     
//!     bus.publish(event).await?;
//!     
//!     // Subscribe to events
//!     let mut subscriber = bus.subscribe("user.*").await?;
//!     while let Some(event) = subscriber.next().await {
//!         println!("Received event: {:?}", event);
//!     }
//!     
//!     Ok(())
//! }
//! ```

pub mod bus;
pub mod event;
pub mod handlers;
pub mod brokers;
pub mod subscriber;
pub mod publisher;
pub mod error;
pub mod nats;

pub use bus::*;
pub use event::*;
pub use subscriber::*;
pub use publisher::*;
pub use error::*;
pub use nats::*;