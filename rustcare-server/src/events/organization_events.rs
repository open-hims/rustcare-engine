//! Organization lifecycle events for NATS event bus
//!
//! Publishes events when organizations are created, updated, verified, or deleted.

use events_bus::{Event, NatsJetStreamBroker};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

/// Organization event publisher
pub struct OrganizationEventPublisher {
    event_bus: Arc<NatsJetStreamBroker>,
}

impl OrganizationEventPublisher {
    /// Create a new organization event publisher
    pub fn new(event_bus: Arc<NatsJetStreamBroker>) -> Self {
        Self { event_bus }
    }

    /// Publish organization created event
    pub async fn publish_organization_created(
        &self,
        org_id: Uuid,
        org_name: &str,
        org_slug: &str,
        subscription_tier: &str,
        created_by: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event = Event {
            id: Uuid::new_v4(),
            event_type: "organization.created".to_string(),
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({
                "organization_id": org_id.to_string(),
                "name": org_name,
                "slug": org_slug,
                "subscription_tier": subscription_tier,
                "created_by": created_by.to_string(),
                "source": "rustcare-engine",
                "event_category": "organization_lifecycle",
            }),
        };

        info!(
            organization_id = %org_id,
            name = org_name,
            slug = org_slug,
            "Publishing organization.created event"
        );

        self.event_bus
            .publish_event(&event)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to publish organization.created event");
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })
    }

    /// Publish organization verified event
    pub async fn publish_organization_verified(
        &self,
        org_id: Uuid,
        org_name: &str,
        verification_method: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event = Event {
            id: Uuid::new_v4(),
            event_type: "organization.verified".to_string(),
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({
                "organization_id": org_id.to_string(),
                "name": org_name,
                "verification_method": verification_method,
                "source": "rustcare-engine",
                "event_category": "organization_lifecycle",
            }),
        };

        info!(
            organization_id = %org_id,
            name = org_name,
            "Publishing organization.verified event"
        );

        self.event_bus
            .publish_event(&event)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to publish organization.verified event");
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })
    }

    /// Publish organization updated event
    pub async fn publish_organization_updated(
        &self,
        org_id: Uuid,
        org_name: &str,
        updated_fields: Vec<String>,
        updated_by: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event = Event {
            id: Uuid::new_v4(),
            event_type: "organization.updated".to_string(),
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({
                "organization_id": org_id.to_string(),
                "name": org_name,
                "updated_fields": updated_fields,
                "updated_by": updated_by.to_string(),
                "source": "rustcare-engine",
                "event_category": "organization_lifecycle",
            }),
        };

        info!(
            organization_id = %org_id,
            name = org_name,
            fields = ?updated_fields,
            "Publishing organization.updated event"
        );

        self.event_bus
            .publish_event(&event)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to publish organization.updated event");
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })
    }

    /// Publish organization deleted event
    pub async fn publish_organization_deleted(
        &self,
        org_id: Uuid,
        org_name: &str,
        deleted_by: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event = Event {
            id: Uuid::new_v4(),
            event_type: "organization.deleted".to_string(),
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({
                "organization_id": org_id.to_string(),
                "name": org_name,
                "deleted_by": deleted_by.to_string(),
                "source": "rustcare-engine",
                "event_category": "organization_lifecycle",
            }),
        };

        info!(
            organization_id = %org_id,
            name = org_name,
            "Publishing organization.deleted event"
        );

        self.event_bus
            .publish_event(&event)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to publish organization.deleted event");
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })
    }

    /// Publish organization subscription changed event
    pub async fn publish_subscription_changed(
        &self,
        org_id: Uuid,
        org_name: &str,
        old_tier: &str,
        new_tier: &str,
        changed_by: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event = Event {
            id: Uuid::new_v4(),
            event_type: "organization.subscription_changed".to_string(),
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({
                "organization_id": org_id.to_string(),
                "name": org_name,
                "old_tier": old_tier,
                "new_tier": new_tier,
                "changed_by": changed_by.to_string(),
                "source": "rustcare-engine",
                "event_category": "billing",
            }),
        };

        info!(
            organization_id = %org_id,
            name = org_name,
            old_tier = old_tier,
            new_tier = new_tier,
            "Publishing organization.subscription_changed event"
        );

        self.event_bus
            .publish_event(&event)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to publish organization.subscription_changed event");
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })
    }

    /// Publish email domain verified event
    pub async fn publish_email_domain_verified(
        &self,
        org_id: Uuid,
        org_name: &str,
        domain: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event = Event {
            id: Uuid::new_v4(),
            event_type: "organization.email_domain_verified".to_string(),
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({
                "organization_id": org_id.to_string(),
                "name": org_name,
                "domain": domain,
                "source": "rustcare-engine",
                "event_category": "email_configuration",
            }),
        };

        info!(
            organization_id = %org_id,
            name = org_name,
            domain = domain,
            "Publishing organization.email_domain_verified event"
        );

        self.event_bus
            .publish_event(&event)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to publish organization.email_domain_verified event");
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })
    }
}

/// Organization event data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationCreatedEvent {
    pub organization_id: Uuid,
    pub name: String,
    pub slug: String,
    pub subscription_tier: String,
    pub created_by: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationVerifiedEvent {
    pub organization_id: Uuid,
    pub name: String,
    pub verification_method: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationUpdatedEvent {
    pub organization_id: Uuid,
    pub name: String,
    pub updated_fields: Vec<String>,
    pub updated_by: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
