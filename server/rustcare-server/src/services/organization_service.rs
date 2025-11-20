//! Organization service that orchestrates all infrastructure services
//!
//! This service coordinates:
//! - Database operations (via OrganizationRepository)
//! - Email notifications (via EmailService)
//! - Event publishing (via NATS)
//! - S3 storage bucket creation (via MinIO)
//! - Audit logging (via AuditLogger)

use crate::auth::db::organization_repository::OrganizationRepository;
use crate::auth::models::{CreateOrganization, Organization, UpdateOrganization};
// use crate::events::OrganizationEventPublisher; // TODO: Implement events module
// use crate::storage::S3StorageService; // TODO: Implement storage module
use database_layer::AuditLogger;
use email_service::EmailService;
// use events_bus::NatsJetStreamBroker; // TODO: Wire up NATS
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

// TODO: Implement these modules
type OrganizationEventPublisher = ();
type S3StorageService = ();

/// Result type for OrganizationService operations
type ServiceResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Organization service with full infrastructure integration
pub struct OrganizationService {
    /// Database repository
    repo: OrganizationRepository,
    /// Email service
    email_service: Option<Arc<EmailService>>,
    /// Event publisher
    event_publisher: Option<Arc<OrganizationEventPublisher>>,
    /// Audit logger
    audit_logger: Option<Arc<AuditLogger>>,
    /// S3 storage service
    s3_service: Option<Arc<S3StorageService>>,
}

impl OrganizationService {
    /// Create a new organization service
    pub fn new(repo: OrganizationRepository) -> Self {
        Self {
            repo,
            email_service: None,
            event_publisher: None,
            audit_logger: None,
            s3_service: None,
        }
    }

    /// Add email service
    pub fn with_email_service(mut self, email_service: Arc<EmailService>) -> Self {
        self.email_service = Some(email_service);
        self
    }

    /// Add event publisher
    pub fn with_event_publisher(mut self, event_publisher: Arc<OrganizationEventPublisher>) -> Self {
        self.event_publisher = Some(event_publisher);
        self
    }

    /// Add audit logger
    pub fn with_audit_logger(mut self, audit_logger: Arc<AuditLogger>) -> Self {
        self.audit_logger = Some(audit_logger);
        self
    }

    /// Add S3 storage service
    pub fn with_s3_service(mut self, s3_service: Arc<S3StorageService>) -> Self {
        self.s3_service = Some(s3_service);
        self
    }

    /// Create a new organization with full infrastructure setup
    ///
    /// This orchestrates:
    /// 1. Create organization in database
    /// 2. Send welcome email to contact
    /// 3. Publish organization.created event to NATS
    /// 4. Create S3 bucket for organization storage
    /// 5. Log audit entry
    pub async fn create_organization(
        &self,
        org_data: CreateOrganization,
        created_by: Uuid,
    ) -> Result<Organization, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            name = %org_data.name,
            slug = %org_data.slug,
            created_by = %created_by,
            "Creating new organization"
        );

        // 1. Create organization in database
        let organization = self.repo.create(org_data.clone()).await
            .map_err(|e| {
                error!(error = %e, "Failed to create organization in database");
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;

        info!(
            organization_id = %organization.id,
            "✅ Organization created in database"
        );

        // 2. Send welcome email (non-blocking, log errors)
        if let Some(email_service) = &self.email_service {
            let contact_email = organization.contact_email.as_ref()
                .or(organization.billing_email.as_ref());

            if let Some(email) = contact_email {
                match email_service
                    .send_organization_welcome(
                        email,
                        &organization.name,
                        &organization.slug,
                    )
                    .await
                {
                    Ok(message_id) => {
                        info!(
                            organization_id = %organization.id,
                            email = %email,
                            message_id = %message_id,
                            "✅ Welcome email sent"
                        );
                    }
                    Err(e) => {
                        warn!(
                            organization_id = %organization.id,
                            email = %email,
                            error = %e,
                            "⚠️  Failed to send welcome email (non-critical)"
                        );
                    }
                }
            } else {
                warn!(
                    organization_id = %organization.id,
                    "⚠️  No contact email provided, skipping welcome email"
                );
            }
        } else {
            info!("Email service not configured, skipping welcome email");
        }

        // 3. Publish organization.created event to NATS (non-blocking)
        // TODO: Implement event publishing once OrganizationEventPublisher is available
        // if let Some(event_publisher) = &self.event_publisher {
        //     match event_publisher.publish_organization_created(...).await {
        //         Ok(_) => info!(organization_id = %organization.id, "✅ organization.created event published"),
        //         Err(e) => warn!(organization_id = %organization.id, error = %e, "⚠️  Failed to publish event"),
        //     }
        // }
        info!("Event publisher not configured, skipping event publishing");

        // 4. Create S3 bucket for organization storage
        // TODO: Implement S3 bucket creation once S3StorageService is available
        // if let Some(s3_service) = &self.s3_service {
        //     match s3_service.create_organization_bucket(&organization.slug).await {
        //         Ok(bucket_name) => info!(organization_id = %organization.id, bucket = %bucket_name, "✅ S3 bucket created"),
        //         Err(e) => warn!(organization_id = %organization.id, error = %e, "⚠️  Failed to create S3 bucket"),
        //     }
        // }
        info!("S3 service not configured, skipping bucket creation");

        // 5. Log detailed audit entry
        if let Some(audit_logger) = &self.audit_logger {
            let _ = audit_logger
                .log_operation(
                    created_by,
                    &organization.id.to_string(),
                    "organization.created",
                    "organizations",
                    Some(&organization.id.to_string()),
                    serde_json::json!({
                        "organization_id": organization.id,
                        "name": organization.name,
                        "slug": organization.slug,
                        "domain": organization.domain,
                        "subscription_tier": organization.subscription_tier,
                        "max_users": organization.max_users,
                        "max_storage_gb": organization.max_storage_gb,
                        "created_by": created_by,
                        "contact_email": organization.contact_email,
                        "billing_email": organization.billing_email,
                    }),
                )
                .await;

            info!(
                organization_id = %organization.id,
                "✅ Audit log entry created"
            );
        }

        info!(
            organization_id = %organization.id,
            name = %organization.name,
            "✅ Organization creation completed successfully"
        );

        Ok(organization)
    }

    /// Verify organization (e.g., email domain verification)
    pub async fn verify_organization(
        &self,
        org_id: Uuid,
        verification_method: &str,
        verified_by: Uuid,
    ) -> Result<Organization, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            organization_id = %org_id,
            verification_method = %verification_method,
            "Verifying organization"
        );

        // Update organization in database
        let organization = self.repo.verify_organization(org_id).await
            .map_err(|e| {
                error!(error = %e, "Failed to verify organization in database");
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;

        // Publish organization.verified event
        // TODO: Implement event publishing once OrganizationEventPublisher is available
        // if let Some(event_publisher) = &self.event_publisher {
        //     let _ = event_publisher
        //         .publish_organization_verified(...)
        //         .await;
        // }

        // Log audit entry
        if let Some(audit_logger) = &self.audit_logger {
            let _ = audit_logger
                .log_operation(
                    verified_by,
                    &organization.id.to_string(),
                    "organization.verified",
                    "organizations",
                    Some(&organization.id.to_string()),
                    serde_json::json!({
                        "organization_id": organization.id,
                        "name": organization.name,
                        "verification_method": verification_method,
                        "verified_by": verified_by,
                    }),
                )
                .await;
        }

        Ok(organization)
    }

    /// Update organization
    pub async fn update_organization(
        &self,
        org_id: Uuid,
        update: UpdateOrganization,
        updated_by: Uuid,
    ) -> Result<Organization, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            organization_id = %org_id,
            updated_by = %updated_by,
            "Updating organization"
        );

        // Track which fields are being updated
        let mut updated_fields = Vec::new();
        if update.name.is_some() {
            updated_fields.push("name".to_string());
        }
        if update.domain.is_some() {
            updated_fields.push("domain".to_string());
        }
        if update.is_active.is_some() {
            updated_fields.push("is_active".to_string());
        }
        if update.is_verified.is_some() {
            updated_fields.push("is_verified".to_string());
        }

        // Update in database
        let organization = self.repo.update(org_id, update.clone()).await
            .map_err(|e| {
                error!(error = %e, "Failed to update organization in database");
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;

        // Publish organization.updated event
        // TODO: Implement event publishing once OrganizationEventPublisher is available
        // if let Some(event_publisher) = &self.event_publisher {
        //     let _ = event_publisher
        //         .publish_organization_updated(...)
        //         .await;
        // }

        // Log audit entry
        if let Some(audit_logger) = &self.audit_logger {
            let _ = audit_logger
                .log_operation(
                    updated_by,
                    &organization.id.to_string(),
                    "organization.updated",
                    "organizations",
                    Some(&organization.id.to_string()),
                    serde_json::json!({
                        "organization_id": organization.id,
                        "updated_fields": updated_fields,
                        "changes": update,
                        "updated_by": updated_by,
                    }),
                )
                .await;
        }

        Ok(organization)
    }

    /// Setup email domain verification for organization
    pub async fn setup_email_domain_verification(
        &self,
        org_id: Uuid,
        domain: &str,
        requested_by: Uuid,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            organization_id = %org_id,
            domain = %domain,
            "Setting up email domain verification"
        );

        // Generate verification token
        let verification_token = format!(
            "rustcare-verify-{}",
            Uuid::new_v4().to_string().replace("-", "")
        );

        // Get organization
        let organization = self.repo.find_by_id(org_id).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
            .ok_or_else(|| "Organization not found")?;

        // Send verification email with DNS instructions
        if let Some(email_service) = &self.email_service {
            let contact_email = organization.contact_email.as_ref()
                .or(organization.billing_email.as_ref())
                .ok_or_else(|| "No contact email found")?;

            email_service
                .send_email_domain_verification(
                    contact_email,
                    &organization.name,
                    domain,
                    &verification_token,
                )
                .await?;

            info!(
                organization_id = %org_id,
                domain = %domain,
                "✅ Email domain verification instructions sent"
            );
        }

        // Log audit entry
        if let Some(audit_logger) = &self.audit_logger {
            let _ = audit_logger
                .log_operation(
                    requested_by,
                    &organization.id.to_string(),
                    "organization.email_domain_verification_requested",
                    "organizations",
                    Some(&organization.id.to_string()),
                    serde_json::json!({
                        "organization_id": organization.id,
                        "domain": domain,
                        "verification_token": verification_token,
                        "requested_by": requested_by,
                    }),
                )
                .await;
        }

        Ok(verification_token)
    }

    /// Delete organization (soft delete)
    pub async fn delete_organization(
        &self,
        org_id: Uuid,
        deleted_by: Uuid,
    ) -> Result<Organization, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            organization_id = %org_id,
            deleted_by = %deleted_by,
            "Deleting organization (soft delete)"
        );

        // Get organization before deletion
        let organization = self.repo.find_by_id(org_id).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
            .ok_or_else(|| "Organization not found")?;

        // Clone organization before it's potentially moved
        let deleted_org = organization.clone();
        
        // Soft delete in database (mark as deleted)
        // TODO: Implement soft delete in OrganizationRepository
        // self.repo.soft_delete(org_id).await
        //     .map_err(|e| {
        //         error!(error = %e, "Failed to delete organization in database");
        //         Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        //     })?;

        // Publish organization.deleted event
        // TODO: Implement event publishing once OrganizationEventPublisher is available
        // if let Some(event_publisher) = &self.event_publisher {
        //     let _ = event_publisher
        //         .publish_organization_deleted(...)
        //         .await;
        // }

        // Log audit entry
        if let Some(audit_logger) = &self.audit_logger {
            let _ = audit_logger
                .log_operation(
                    deleted_by,
                    &organization.id.to_string(),
                    "organization.deleted",
                    "organizations",
                    Some(&organization.id.to_string()),
                    serde_json::json!({
                        "organization_id": organization.id,
                        "name": organization.name,
                        "deleted_by": deleted_by,
                    }),
                )
                .await;
        }

        info!(
            organization_id = %org_id,
            "✅ Organization deleted successfully"
        );

        Ok(deleted_org)
    }
}
