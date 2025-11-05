use async_nats;
use crate::event::Event;
use crate::error::{EventBusError, Result as EventBusResult};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;
use serde_json;
use futures::StreamExt;
use chrono::Utc;

pub struct NatsJetStreamBroker {
    client: Arc<async_nats::Client>,
    subscriptions: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl NatsJetStreamBroker {
    pub async fn new(nats_url: &str) -> EventBusResult<Self> {
        let client = async_nats::connect(nats_url).await
            .map_err(|_| EventBusError::BrokerConnectionError)?;
        
        Ok(Self {
            client: Arc::new(client),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn publish_event(&self, event: &Event) -> EventBusResult<()> {
        let subject = event.event_type.clone();
        
        // Add healthcare-specific metadata as headers
        let mut headers = async_nats::HeaderMap::new();
        headers.insert("event_id", event.id.to_string().as_str());
        headers.insert("timestamp", event.timestamp.to_rfc3339().as_str());
        
        // HIPAA compliance tracking - extract from data field
        if let Some(patient_id) = event.data.get("patient_id").and_then(|v| v.as_str()) {
            headers.insert("patient_id", patient_id);
            headers.insert("phi_flag", "true"); // Protected Health Information
        }
        
        if let Some(user_id) = event.data.get("user_id").and_then(|v| v.as_str()) {
            headers.insert("user_id", user_id);
        }

        let payload = serde_json::to_vec(&event)
            .map_err(|_| EventBusError::SerializationError)?;

        self.client.publish_with_headers(subject, headers, payload.into()).await
            .map_err(|_| EventBusError::PublishError)?;

        Ok(())
    }

    pub async fn subscribe_to_events<F>(&self, subject: &str, handler: F) -> EventBusResult<String>
    where
        F: Fn(Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + Sync + 'static,
    {
        let subscription_id = Uuid::new_v4().to_string();
        let client = self.client.clone();
        let subject = subject.to_string();
        let handler = Arc::new(handler);
        
        let handle = tokio::spawn(async move {
            if let Ok(mut subscriber) = client.subscribe(subject).await {
                while let Some(message) = subscriber.next().await {
                    if let Ok(event) = serde_json::from_slice::<Event>(&message.payload) {
                        if let Err(e) = handler(event) {
                            eprintln!("Handler error: {}", e);
                        }
                    }
                }
            }
        });

        self.subscriptions.write().await.insert(subscription_id.clone(), handle);
        Ok(subscription_id)
    }

    pub async fn unsubscribe(&self, subscription_id: &str) -> EventBusResult<()> {
        if let Some(handle) = self.subscriptions.write().await.remove(subscription_id) {
            handle.abort();
        }
        Ok(())
    }

    // Healthcare-specific utility methods
    
    pub async fn audit_log_access(&self, user_id: &str, resource: &str, action: &str) -> EventBusResult<()> {
        let audit_event = Event {
            id: Uuid::new_v4(),
            event_type: "audit.access".to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({
                "user_id": user_id,
                "resource": resource,
                "action": action,
                "source": "rustcare-engine",
                "audit_type": "access",
                "timestamp": Utc::now().to_rfc3339()
            }),
        };

        self.publish_event(&audit_event).await
    }

    pub async fn publish_vital_alert(&self, patient_id: &str, vital_type: &str, value: f64, threshold: f64) -> EventBusResult<()> {
        let vital_event = Event {
            id: Uuid::new_v4(),
            event_type: if value > threshold * 1.2 { "vitals.critical" } else { "vitals.alert" }.to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({
                "patient_id": patient_id,
                "vital_type": vital_type,
                "value": value,
                "threshold": threshold,
                "severity": if value > threshold * 1.2 { "critical" } else { "warning" },
                "source": "vital-monitor",
                "phi_flag": true,
                "alert_type": vital_type
            }),
        };

        self.publish_event(&vital_event).await
    }

    // Compliance and reporting methods
    
    pub async fn generate_hipaa_audit_report(&self, start_date: chrono::DateTime<Utc>, end_date: chrono::DateTime<Utc>) -> EventBusResult<serde_json::Value> {
        // Basic audit report structure for HIPAA compliance
        Ok(serde_json::json!({
            "report_type": "hipaa_audit",
            "start_date": start_date.to_rfc3339(),
            "end_date": end_date.to_rfc3339(),
            "total_access_events": 0,
            "phi_access_events": 0,
            "unauthorized_access_attempts": 0
        }))
    }
}