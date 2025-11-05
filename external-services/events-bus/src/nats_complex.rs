use async_nats::{self, jetstream};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::{
    error::{EventBusError, Result},
    event::{Event, DomainEvent},
    brokers::EventBroker,
};

/// NATS JetStream broker implementation for high-performance healthcare messaging
pub struct NatsJetStreamBroker {
    /// NATS client connection
    client: Option<async_nats::Client>,
    /// JetStream context
    jetstream: Option<jetstream::Context>,
    /// Stream configurations
    streams: RwLock<HashMap<String, jetstream::stream::Stream>>,
    /// Consumer configurations
    consumers: RwLock<HashMap<String, jetstream::consumer::Consumer<jetstream::consumer::pull::Config>>>,
    /// Broker configuration
    config: NatsConfig,
}

/// NATS JetStream configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    /// NATS server URL
    pub server_url: String,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Maximum reconnection attempts
    pub max_reconnection_attempts: u32,
    /// Enable TLS encryption
    pub enable_tls: bool,
    /// Client certificate path (for mTLS)
    pub client_cert_path: Option<String>,
    /// Client private key path (for mTLS)
    pub client_key_path: Option<String>,
    /// CA certificate path
    pub ca_cert_path: Option<String>,
    /// JetStream domain (for multi-tenant)
    pub jetstream_domain: Option<String>,
    /// Default stream configuration
    pub default_stream_config: StreamConfig,
}

/// Stream configuration for JetStream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Stream name
    pub name: String,
    /// Stream subjects
    pub subjects: Vec<String>,
    /// Maximum number of messages
    pub max_messages: Option<i64>,
    /// Maximum age of messages
    pub max_age_seconds: Option<u64>,
    /// Maximum bytes in stream
    pub max_bytes: Option<i64>,
    /// Storage type (file or memory)
    pub storage: String,
    /// Number of replicas
    pub replicas: u32,
    /// Enable stream deduplication
    pub duplicate_window_seconds: Option<u64>,
}

/// Healthcare-specific event wrapper for NATS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsHealthcareEvent {
    /// Original event
    pub event: Event,
    /// HIPAA compliance flags
    pub hipaa_compliant: bool,
    /// Patient ID (if applicable, encrypted)
    pub patient_id_hash: Option<String>,
    /// Healthcare provider ID
    pub provider_id: Option<String>,
    /// Event criticality level
    pub criticality: EventCriticality,
    /// Retention policy
    pub retention_policy: RetentionPolicy,
}

/// Event criticality levels for healthcare
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventCriticality {
    /// Low priority events
    Low,
    /// Normal priority events
    Normal,
    /// High priority events (patient safety)
    High,
    /// Critical events (emergency)
    Critical,
}

/// Data retention policy for healthcare events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionPolicy {
    /// Retain for specified days
    Days(u32),
    /// Retain for specified months
    Months(u32),
    /// Retain for specified years
    Years(u32),
    /// Retain indefinitely (for audit trails)
    Indefinite,
}

impl NatsJetStreamBroker {
    /// Create a new NATS JetStream broker
    pub async fn new(config: NatsConfig) -> Result<Self> {
        let broker = Self {
            client: None,
            jetstream: None,
            streams: RwLock::new(HashMap::new()),
            consumers: RwLock::new(HashMap::new()),
            config,
        };

        Ok(broker)
    }

    /// Publish healthcare event with HIPAA compliance
    pub async fn publish_healthcare_event(
        &self,
        subject: &str,
        event: Event,
        hipaa_compliant: bool,
        criticality: EventCriticality,
        retention_policy: RetentionPolicy,
    ) -> Result<()> {
        let healthcare_event = NatsHealthcareEvent {
            event,
            hipaa_compliant,
            patient_id_hash: None, // TODO: Extract and hash patient ID if present
            provider_id: None,     // TODO: Extract provider ID from context
            criticality,
            retention_policy,
        };

        self.publish_raw(subject, &healthcare_event).await
    }

    /// Subscribe to healthcare events with filtering
    pub async fn subscribe_healthcare_events(
        &self,
        subject: &str,
        consumer_name: &str,
        filter_config: HealthcareEventFilter,
    ) -> Result<jetstream::consumer::Consumer<jetstream::consumer::pull::Config>> {
        // Create consumer with healthcare-specific configuration
        let jetstream = self.jetstream.as_ref()
            .ok_or_else(|| EventBusError::BrokerConnectionError)?;

        let consumer_config = jetstream::consumer::pull::Config {
            name: Some(consumer_name.to_string()),
            durable_name: Some(consumer_name.to_string()),
            filter_subject: subject.to_string(),
            ack_policy: jetstream::consumer::AckPolicy::Explicit,
            max_deliver: 3, // Retry up to 3 times for healthcare events
            ..Default::default()
        };

        let stream_name = self.get_stream_for_subject(subject).await?;
        
        // For now, return a mock consumer until we can properly integrate with the async-nats API
        let consumer = jetstream::consumer::Consumer::new(consumer_config);
        
        // TODO: Implement proper consumer creation with current async-nats API

        // Store consumer reference
        let mut consumers = self.consumers.write().await;
        consumers.insert(consumer_name.to_string(), consumer.clone());

        info!("Created healthcare event consumer: {} for subject: {}", consumer_name, subject);

        Ok(consumer)
    }

    /// Get stream name for a subject
    async fn get_stream_for_subject(&self, subject: &str) -> Result<String> {
        // For healthcare events, we use domain-specific streams
        let stream_name = if subject.starts_with("healthcare.patient.") {
            "HEALTHCARE_PATIENT_EVENTS"
        } else if subject.starts_with("healthcare.provider.") {
            "HEALTHCARE_PROVIDER_EVENTS"
        } else if subject.starts_with("healthcare.audit.") {
            "HEALTHCARE_AUDIT_EVENTS"
        } else if subject.starts_with("healthcare.workflow.") {
            "HEALTHCARE_WORKFLOW_EVENTS"
        } else {
            "HEALTHCARE_GENERAL_EVENTS"
        };

        Ok(stream_name.to_string())
    }

    /// Publish raw message to NATS JetStream
    async fn publish_raw<T>(&self, subject: &str, payload: &T) -> Result<()>
    where
        T: Serialize,
    {
        let jetstream = self.jetstream.as_ref()
            .ok_or_else(|| EventBusError::BrokerConnectionError)?;

        let serialized = serde_json::to_vec(payload)
            .map_err(|_| EventBusError::SerializationError)?;

        jetstream
            .publish(subject, serialized.into())
            .await
            .map_err(|_| EventBusError::PublishError)?;

        debug!("Published message to subject: {}", subject);
        Ok(())
    }

    /// Create healthcare-specific streams
    async fn setup_healthcare_streams(&self) -> Result<()> {
        let jetstream = self.jetstream.as_ref()
            .ok_or_else(|| EventBusError::BrokerConnectionError)?;

        // Healthcare streams with different retention policies
        let stream_configs = vec![
            // Patient events - 7 year retention for legal compliance
            ("HEALTHCARE_PATIENT_EVENTS", vec!["healthcare.patient.*"], 7 * 365 * 24 * 3600), // 7 years
            // Provider events - 3 year retention
            ("HEALTHCARE_PROVIDER_EVENTS", vec!["healthcare.provider.*"], 3 * 365 * 24 * 3600), // 3 years
            // Audit events - indefinite retention
            ("HEALTHCARE_AUDIT_EVENTS", vec!["healthcare.audit.*"], 0), // Indefinite
            // Workflow events - 1 year retention
            ("HEALTHCARE_WORKFLOW_EVENTS", vec!["healthcare.workflow.*"], 365 * 24 * 3600), // 1 year
            // General events - 30 day retention
            ("HEALTHCARE_GENERAL_EVENTS", vec!["healthcare.*"], 30 * 24 * 3600), // 30 days
        ];

        for (stream_name, subjects, max_age_seconds) in stream_configs {
            let stream_config = jetstream::stream::Config {
                name: stream_name.to_string(),
                subjects: subjects.iter().map(|s| s.to_string()).collect(),
                max_age: if max_age_seconds > 0 {
                    Some(std::time::Duration::from_secs(max_age_seconds))
                } else {
                    None
                },
                storage: jetstream::stream::StorageType::File, // Persistent storage for healthcare
                num_replicas: 3, // High availability for healthcare data
                duplicate_window: Some(std::time::Duration::from_secs(300)), // 5 minute dedup window
                ..Default::default()
            };

            match jetstream.create_stream(stream_config).await {
                Ok(stream) => {
                    let mut streams = self.streams.write().await;
                    streams.insert(stream_name.to_string(), stream);
                    info!("Created healthcare stream: {}", stream_name);
                }
                Err(e) => {
                    warn!("Failed to create stream {}: {:?}", stream_name, e);
                    // Check if stream already exists
                    if let Ok(stream) = jetstream.get_stream(stream_name).await {
                        let mut streams = self.streams.write().await;
                        streams.insert(stream_name.to_string(), stream);
                        info!("Using existing healthcare stream: {}", stream_name);
                    } else {
                        return Err(EventBusError::BrokerConnectionError);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Healthcare event filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthcareEventFilter {
    /// Filter by criticality level
    pub min_criticality: Option<EventCriticality>,
    /// Filter by HIPAA compliance requirement
    pub hipaa_only: bool,
    /// Filter by provider ID
    pub provider_id: Option<String>,
    /// Filter by event type pattern
    pub event_type_pattern: Option<String>,
}

#[async_trait]
impl EventBroker for NatsJetStreamBroker {
    async fn connect(&self) -> Result<()> {
        info!("Connecting to NATS JetStream server: {}", self.config.server_url);

        // Connect to NATS server
        let client = async_nats::connect(&self.config.server_url)
            .await
            .map_err(|_| EventBusError::BrokerConnectionError)?;

        // Get JetStream context
        let jetstream = jetstream::new(client.clone());

        // Store connections
        let broker = unsafe { &mut *(self as *const Self as *mut Self) };
        broker.client = Some(client);
        broker.jetstream = Some(jetstream);

        info!("Connected to NATS JetStream successfully");

        // Setup healthcare-specific streams
        self.setup_healthcare_streams().await?;

        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting from NATS JetStream");

        // Clear consumers
        let mut consumers = self.consumers.write().await;
        consumers.clear();

        // Clear streams
        let mut streams = self.streams.write().await;
        streams.clear();

        // The NATS client will be dropped automatically
        info!("Disconnected from NATS JetStream");

        Ok(())
    }
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            server_url: "nats://localhost:4222".to_string(),
            connection_timeout: 30,
            max_reconnection_attempts: 10,
            enable_tls: false,
            client_cert_path: None,
            client_key_path: None,
            ca_cert_path: None,
            jetstream_domain: None,
            default_stream_config: StreamConfig::default(),
        }
    }
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            name: "RUSTCARE_EVENTS".to_string(),
            subjects: vec!["rustcare.*".to_string()],
            max_messages: Some(1_000_000),
            max_age_seconds: Some(30 * 24 * 3600), // 30 days
            max_bytes: Some(1024 * 1024 * 1024), // 1GB
            storage: "file".to_string(),
            replicas: 1,
            duplicate_window_seconds: Some(300), // 5 minutes
        }
    }
}