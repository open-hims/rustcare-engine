use tonic::{transport::Server, Request, Response, Status};
use tracing::{debug, error, info, warn};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::server::RustCareServer;
use error_common::{RustCareError, Result as RustCareResult};
use logger_redacted::RedactedLogger;

// Include the generated gRPC code
pub mod healthcare {
    tonic::include_proto!("rustcare.healthcare.v1");
}

pub mod auth {
    tonic::include_proto!("rustcare.auth.v1");
}

use healthcare::{
    healthcare_service_server::{HealthcareService, HealthcareServiceServer},
    *,
};

use auth::{
    auth_service_server::{AuthService, AuthServiceServer},
    authorization_service_server::{AuthorizationService, AuthorizationServiceServer},
    *,
};

/// Healthcare gRPC service implementation
#[derive(Debug, Clone)]
pub struct HealthcareServiceImpl {
    server: RustCareServer,
    logger: std::sync::Arc<RedactedLogger>,
}

impl HealthcareServiceImpl {
    pub async fn new(server: RustCareServer) -> RustCareResult<Self> {
        let logger = std::sync::Arc::new(RedactedLogger::new("grpc_healthcare").await);
        Ok(Self { server, logger })
    }
}

#[tonic::async_trait]
impl HealthcareService for HealthcareServiceImpl {
    async fn create_patient(
        &self,
        request: Request<CreatePatientRequest>,
    ) -> Result<Response<PatientResponse>, Status> {
        let req = request.into_inner();
        
        self.logger.info(&format!(
            "gRPC CreatePatient request received for patient: {} {}",
            req.patient.as_ref().map(|p| p.first_name.clone()).unwrap_or_default(),
            req.patient.as_ref().map(|p| p.last_name.clone()).unwrap_or_default()
        )).await;

        // TODO: Integrate with actual patient creation logic
        let patient_id = Uuid::new_v4().to_string();
        let audit_log_id = Uuid::new_v4().to_string();

        let mut patient = req.patient.unwrap_or_default();
        patient.patient_id = patient_id;
        patient.created_at = Some(prost_types::Timestamp::from(std::time::SystemTime::now()));
        patient.updated_at = Some(prost_types::Timestamp::from(std::time::SystemTime::now()));

        let response = PatientResponse {
            patient: Some(patient),
            audit_log_id,
        };

        self.logger.info("Patient created successfully via gRPC").await;

        Ok(Response::new(response))
    }

    async fn get_patient(
        &self,
        request: Request<GetPatientRequest>,
    ) -> Result<Response<PatientResponse>, Status> {
        let req = request.into_inner();
        
        self.logger.info(&format!(
            "gRPC GetPatient request for patient_id: {} by provider: {}",
            req.patient_id,
            req.requesting_provider_id
        )).await;

        // TODO: Integrate with actual patient retrieval logic
        // For now, return a mock patient
        let patient = Patient {
            patient_id: req.patient_id.clone(),
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            date_of_birth: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            gender: Gender::Male as i32,
            contact_info: Some(ContactInfo {
                primary_phone: "+1-555-0123".to_string(),
                email: "john.doe@example.com".to_string(),
                ..Default::default()
            }),
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            updated_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            pii_encrypted: true,
            ..Default::default()
        };

        let response = PatientResponse {
            patient: Some(patient),
            audit_log_id: Uuid::new_v4().to_string(),
        };

        Ok(Response::new(response))
    }

    async fn update_patient(
        &self,
        request: Request<UpdatePatientRequest>,
    ) -> Result<Response<PatientResponse>, Status> {
        let req = request.into_inner();
        
        self.logger.info(&format!(
            "gRPC UpdatePatient request for patient: {} by provider: {}",
            req.patient.as_ref().map(|p| p.patient_id.clone()).unwrap_or_default(),
            req.updating_provider_id
        )).await;

        // TODO: Integrate with actual patient update logic
        let mut patient = req.patient.unwrap_or_default();
        patient.updated_at = Some(prost_types::Timestamp::from(std::time::SystemTime::now()));

        let response = PatientResponse {
            patient: Some(patient),
            audit_log_id: Uuid::new_v4().to_string(),
        };

        Ok(Response::new(response))
    }

    async fn list_patients(
        &self,
        request: Request<ListPatientsRequest>,
    ) -> Result<Response<ListPatientsResponse>, Status> {
        let req = request.into_inner();
        
        self.logger.info(&format!(
            "gRPC ListPatients request by provider: {} (page_size: {})",
            req.provider_id,
            req.page_size
        )).await;

        // TODO: Integrate with actual patient listing logic
        let patients = vec![];

        let response = ListPatientsResponse {
            patients,
            next_page_token: "".to_string(),
            total_count: 0,
        };

        Ok(Response::new(response))
    }

    async fn create_medical_record(
        &self,
        request: Request<CreateMedicalRecordRequest>,
    ) -> Result<Response<MedicalRecordResponse>, Status> {
        let req = request.into_inner();
        
        self.logger.info(&format!(
            "gRPC CreateMedicalRecord request by provider: {}",
            req.creating_provider_id
        )).await;

        // TODO: Integrate with actual medical record creation logic
        let mut record = req.record.unwrap_or_default();
        record.record_id = Uuid::new_v4().to_string();
        record.created_at = Some(prost_types::Timestamp::from(std::time::SystemTime::now()));
        record.updated_at = Some(prost_types::Timestamp::from(std::time::SystemTime::now()));
        record.access_log_id = Uuid::new_v4().to_string();

        let response = MedicalRecordResponse {
            record: Some(record),
            audit_log_id: Uuid::new_v4().to_string(),
        };

        Ok(Response::new(response))
    }

    async fn get_medical_record(
        &self,
        request: Request<GetMedicalRecordRequest>,
    ) -> Result<Response<MedicalRecordResponse>, Status> {
        let req = request.into_inner();
        
        self.logger.info(&format!(
            "gRPC GetMedicalRecord request for record_id: {} by provider: {}",
            req.record_id,
            req.requesting_provider_id
        )).await;

        // TODO: Integrate with actual medical record retrieval logic
        return Err(Status::not_found("Medical record not found"));
    }

    async fn list_medical_records(
        &self,
        request: Request<ListMedicalRecordsRequest>,
    ) -> Result<Response<ListMedicalRecordsResponse>, Status> {
        let req = request.into_inner();
        
        self.logger.info(&format!(
            "gRPC ListMedicalRecords request for patient: {} by provider: {}",
            req.patient_id,
            req.requesting_provider_id
        )).await;

        // TODO: Integrate with actual medical record listing logic
        let records = vec![];

        let response = ListMedicalRecordsResponse {
            records,
            next_page_token: "".to_string(),
            total_count: 0,
        };

        Ok(Response::new(response))
    }

    // Additional method implementations would go here...
    // For brevity, I'll implement stub responses for the remaining methods

    async fn create_provider(
        &self,
        _request: Request<CreateProviderRequest>,
    ) -> Result<Response<ProviderResponse>, Status> {
        Err(Status::unimplemented("CreateProvider not yet implemented"))
    }

    async fn get_provider(
        &self,
        _request: Request<GetProviderRequest>,
    ) -> Result<Response<ProviderResponse>, Status> {
        Err(Status::unimplemented("GetProvider not yet implemented"))
    }

    async fn list_providers(
        &self,
        _request: Request<ListProvidersRequest>,
    ) -> Result<Response<ListProvidersResponse>, Status> {
        Err(Status::unimplemented("ListProviders not yet implemented"))
    }

    async fn create_appointment(
        &self,
        _request: Request<CreateAppointmentRequest>,
    ) -> Result<Response<AppointmentResponse>, Status> {
        Err(Status::unimplemented("CreateAppointment not yet implemented"))
    }

    async fn get_appointment(
        &self,
        _request: Request<GetAppointmentRequest>,
    ) -> Result<Response<AppointmentResponse>, Status> {
        Err(Status::unimplemented("GetAppointment not yet implemented"))
    }

    async fn list_appointments(
        &self,
        _request: Request<ListAppointmentsRequest>,
    ) -> Result<Response<ListAppointmentsResponse>, Status> {
        Err(Status::unimplemented("ListAppointments not yet implemented"))
    }

    async fn update_appointment_status(
        &self,
        _request: Request<UpdateAppointmentStatusRequest>,
    ) -> Result<Response<AppointmentResponse>, Status> {
        Err(Status::unimplemented("UpdateAppointmentStatus not yet implemented"))
    }

    async fn submit_vital_signs(
        &self,
        _request: Request<SubmitVitalSignsRequest>,
    ) -> Result<Response<VitalSignsResponse>, Status> {
        Err(Status::unimplemented("SubmitVitalSigns not yet implemented"))
    }

    async fn get_vital_signs(
        &self,
        _request: Request<GetVitalSignsRequest>,
    ) -> Result<Response<ListVitalSignsResponse>, Status> {
        Err(Status::unimplemented("GetVitalSigns not yet implemented"))
    }
}

/// Authentication gRPC service implementation
#[derive(Debug, Clone)]
pub struct AuthServiceImpl {
    server: RustCareServer,
    logger: std::sync::Arc<RedactedLogger>,
}

impl AuthServiceImpl {
    pub async fn new(server: RustCareServer) -> RustCareResult<Self> {
        let logger = std::sync::Arc::new(RedactedLogger::new("grpc_auth").await);
        Ok(Self { server, logger })
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        let req = request.into_inner();
        
        self.logger.info("gRPC Authentication request received").await;

        // TODO: Integrate with actual authentication logic
        let response = AuthenticateResponse {
            success: true,
            access_token: "mock_jwt_token".to_string(),
            refresh_token: "mock_refresh_token".to_string(),
            expires_in: 3600,
            user: Some(User {
                user_id: Uuid::new_v4().to_string(),
                username: "mock_user".to_string(),
                email: "user@example.com".to_string(),
                role: UserRole::HealthcareProvider as i32,
                status: UserStatus::Active as i32,
                ..Default::default()
            }),
            session_id: Uuid::new_v4().to_string(),
            mfa_required: false,
            mfa_token: "".to_string(),
            error_message: "".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn refresh_token(
        &self,
        _request: Request<RefreshTokenRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        Err(Status::unimplemented("RefreshToken not yet implemented"))
    }

    async fn validate_token(
        &self,
        _request: Request<ValidateTokenRequest>,
    ) -> Result<Response<ValidateTokenResponse>, Status> {
        Err(Status::unimplemented("ValidateToken not yet implemented"))
    }

    async fn revoke_token(
        &self,
        _request: Request<RevokeTokenRequest>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("RevokeToken not yet implemented"))
    }

    async fn initiate_o_auth(
        &self,
        _request: Request<InitiateOAuthRequest>,
    ) -> Result<Response<InitiateOAuthResponse>, Status> {
        Err(Status::unimplemented("InitiateOAuth not yet implemented"))
    }

    async fn complete_o_auth(
        &self,
        _request: Request<CompleteOAuthRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        Err(Status::unimplemented("CompleteOAuth not yet implemented"))
    }

    async fn initiate_mfa(
        &self,
        _request: Request<InitiateMfaRequest>,
    ) -> Result<Response<InitiateMfaResponse>, Status> {
        Err(Status::unimplemented("InitiateMFA not yet implemented"))
    }

    async fn complete_mfa(
        &self,
        _request: Request<CompleteMfaRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        Err(Status::unimplemented("CompleteMFA not yet implemented"))
    }

    async fn get_session(
        &self,
        _request: Request<GetSessionRequest>,
    ) -> Result<Response<SessionResponse>, Status> {
        Err(Status::unimplemented("GetSession not yet implemented"))
    }

    async fn invalidate_session(
        &self,
        _request: Request<InvalidateSessionRequest>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("InvalidateSession not yet implemented"))
    }

    async fn list_active_sessions(
        &self,
        _request: Request<ListActiveSessionsRequest>,
    ) -> Result<Response<ListActiveSessionsResponse>, Status> {
        Err(Status::unimplemented("ListActiveSessions not yet implemented"))
    }
}

/// Authorization gRPC service implementation (Zanzibar-style)
#[derive(Debug, Clone)]
pub struct AuthorizationServiceImpl {
    server: RustCareServer,
    logger: std::sync::Arc<RedactedLogger>,
}

impl AuthorizationServiceImpl {
    pub async fn new(server: RustCareServer) -> RustCareResult<Self> {
        let logger = std::sync::Arc::new(RedactedLogger::new("grpc_authz").await);
        Ok(Self { server, logger })
    }
}

#[tonic::async_trait]
impl AuthorizationService for AuthorizationServiceImpl {
    async fn check_permission(
        &self,
        request: Request<CheckPermissionRequest>,
    ) -> Result<Response<CheckPermissionResponse>, Status> {
        let req = request.into_inner();
        
        self.logger.info(&format!(
            "gRPC CheckPermission request for object: {:?}, relation: {}",
            req.object,
            req.relation
        )).await;

        // TODO: Integrate with auth-zanzibar module
        let response = CheckPermissionResponse {
            allowed: true, // Mock response
            consistency_token: Uuid::new_v4().to_string(),
        };

        Ok(Response::new(response))
    }

    async fn batch_check_permissions(
        &self,
        _request: Request<BatchCheckPermissionsRequest>,
    ) -> Result<Response<BatchCheckPermissionsResponse>, Status> {
        Err(Status::unimplemented("BatchCheckPermissions not yet implemented"))
    }

    async fn write_relationship(
        &self,
        _request: Request<WriteRelationshipRequest>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("WriteRelationship not yet implemented"))
    }

    async fn delete_relationship(
        &self,
        _request: Request<DeleteRelationshipRequest>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("DeleteRelationship not yet implemented"))
    }

    async fn read_relationships(
        &self,
        _request: Request<ReadRelationshipsRequest>,
    ) -> Result<Response<ReadRelationshipsResponse>, Status> {
        Err(Status::unimplemented("ReadRelationships not yet implemented"))
    }

    async fn expand(
        &self,
        _request: Request<ExpandRequest>,
    ) -> Result<Response<ExpandResponse>, Status> {
        Err(Status::unimplemented("Expand not yet implemented"))
    }

    type WatchStream = tokio_stream::wrappers::ReceiverStream<Result<WatchResponse, Status>>;

    async fn watch(
        &self,
        _request: Request<WatchRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        Err(Status::unimplemented("Watch not yet implemented"))
    }
}

/// Start the gRPC server
pub async fn start_grpc_server(
    addr: SocketAddr,
    server: RustCareServer,
) -> RustCareResult<()> {
    let healthcare_service = HealthcareServiceImpl::new(server.clone()).await?;
    let auth_service = AuthServiceImpl::new(server.clone()).await?;
    let authz_service = AuthorizationServiceImpl::new(server).await?;

    info!("Starting gRPC server on {}", addr);

    Server::builder()
        .add_service(HealthcareServiceServer::new(healthcare_service))
        .add_service(AuthServiceServer::new(auth_service))
        .add_service(AuthorizationServiceServer::new(authz_service))
        .serve(addr)
        .await
        .map_err(|e| RustCareError::GrpcError(format!("gRPC server error: {}", e)))?;

    Ok(())
}