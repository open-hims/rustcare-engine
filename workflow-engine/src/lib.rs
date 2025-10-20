//! Workflow orchestration and process automation engine for RustCare Engine
//! 
//! This module provides a comprehensive workflow engine supporting:
//! - Declarative workflow definitions (YAML/JSON/Code)
//! - State machine-based execution with compensation patterns
//! - Parallel and sequential task execution
//! - Conditional branching and loops
//! - Human-in-the-loop tasks and approvals
//! - Timeout handling and retry policies
//! - Saga pattern for distributed transactions
//! - Event-driven workflow triggers
//! - Workflow versioning and migration
//! - Visual workflow monitoring and debugging
//! 
//! # Workflow Types
//! 
//! - **Sequential Workflows**: Step-by-step linear execution
//! - **Parallel Workflows**: Concurrent task execution with barriers
//! - **State Machine Workflows**: Complex branching and decision logic
//! - **Event-Driven Workflows**: Reactive workflows triggered by events
//! - **Long-Running Workflows**: Durable workflows that survive restarts
//! - **Scheduled Workflows**: Cron-based and time-triggered workflows
//! 
//! # Example
//! 
//! ```rust
//! use workflow_engine::{WorkflowEngine, Workflow, Task, TaskType};
//! use serde_json::json;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = WorkflowEngine::new().await?;
//!     
//!     // Define a workflow
//!     let workflow = Workflow::builder("user_onboarding")
//!         .add_task(Task::new("send_welcome_email", TaskType::HttpRequest))
//!         .add_task(Task::new("create_profile", TaskType::DatabaseOperation))
//!         .add_task(Task::new("assign_role", TaskType::Custom))
//!         .build();
//!     
//!     // Execute workflow
//!     let execution = engine.execute(workflow, json!({
//!         "user_id": "123",
//!         "email": "user@example.com"
//!     })).await?;
//!     
//!     // Monitor execution
//!     while !execution.is_complete().await? {
//!         let status = execution.get_status().await?;
//!         println!("Workflow status: {:?}", status);
//!         tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//!     }
//!     
//!     Ok(())
//! }
//! ```

pub mod engine;
pub mod workflow;
pub mod task;
pub mod scheduler;
pub mod executor;
pub mod state_machine;
pub mod conditions;
pub mod compensation;
pub mod error;

pub use engine::*;
pub use workflow::*;
pub use task::*;
pub use error::*;