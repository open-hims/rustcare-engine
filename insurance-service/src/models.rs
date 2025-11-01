use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Insurance plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsurancePlan {
    pub id: Uuid,
    pub name: String,
    pub payer_id: String,
    pub payer_name: String,
    pub plan_type: PlanType,
    pub effective_date: DateTime<Utc>,
    pub termination_date: Option<DateTime<Utc>>,
    pub benefits: PlanBenefits,
    pub is_active: bool,
}

/// Plan type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanType {
    Commercial,
    Medicare,
    Medicaid,
    Tricare,
    SelfPay,
}

/// Plan benefits summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanBenefits {
    pub deductible: f64,
    pub out_of_pocket_max: f64,
    pub copay_primary: Option<f64>,
    pub copay_specialist: Option<f64>,
    pub coinsurance: Option<f64>,
}

/// Eligibility check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityResult {
    pub patient_id: Uuid,
    pub insurance_id: Uuid,
    pub is_eligible: bool,
    pub active: bool,
    pub effective_date: Option<DateTime<Utc>>,
    pub termination_date: Option<DateTime<Utc>>,
    pub coverage_type: String,
    pub copay_info: Option<String>,
    pub deductible_info: Option<String>,
    pub checked_at: DateTime<Utc>,
}

/// Prior authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorAuthorization {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub insurance_id: Uuid,
    pub provider_id: Uuid,
    pub service_code: String,
    pub description: String,
    pub requested_date: DateTime<Utc>,
    pub status: AuthorizationStatus,
    pub auth_number: Option<String>,
    pub effective_date: Option<DateTime<Utc>>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub approved_amount: Option<f64>,
    pub denial_reason: Option<String>,
    pub notes: Option<String>,
}

/// Authorization status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationStatus {
    Pending,
    Approved,
    Denied,
    PartiallyApproved,
    Expired,
}

/// Pre-certification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCertification {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub insurance_id: Uuid,
    pub service_code: String,
    pub description: String,
    pub status: PreCertStatus,
    pub cert_number: Option<String>,
    pub required: bool,
}

/// Pre-cert status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreCertStatus {
    Required,
    Obtained,
    NotRequired,
    Expired,
}

