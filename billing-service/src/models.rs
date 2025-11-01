use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Billing charge from clinical encounter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Charge {
    pub id: Uuid,
    pub encounter_id: Uuid,
    pub patient_id: Uuid,
    pub provider_id: Uuid,
    pub service_code: String, // CPT, HCPCS, ICD-10-PCS
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub total_amount: Decimal,
    pub status: ChargeStatus,
    pub bill_to: BillTo,
    pub created_at: DateTime<Utc>,
}

/// Charge status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChargeStatus {
    Draft,
    Billed,
    Paid,
    Denied,
    Pending,
    WrittenOff,
}

/// Who to bill for the charge
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BillTo {
    Patient { patient_id: Uuid },
    Insurance {
        insurance_id: Uuid,
        policy_number: String,
        group_number: Option<String>,
    },
    Both {
        insurance_id: Uuid,
        policy_number: String,
        patient_portion: Decimal,
    },
}

/// Insurance claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: Uuid,
    pub claim_number: String,
    pub patient_id: Uuid,
    pub insurance_id: Uuid,
    pub provider_id: Uuid,
    pub charges: Vec<Charge>,
    pub claim_type: ClaimType,
    pub total_amount: Decimal,
    pub status: ClaimStatus,
    pub submission_date: Option<DateTime<Utc>>,
    pub remittance_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Claim type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ClaimType {
    Professional, // HCFA-1500 / 837P
    Institutional, // UB-04 / 837I
    Dental, // ADA / 837D
}

/// Claim status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Draft,
    Submitted,
    Processing,
    Accepted,
    Denied,
    Paid,
    Appealed,
}

/// Payment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub claim_id: Option<Uuid>,
    pub charge_id: Option<Uuid>,
    pub patient_id: Uuid,
    pub amount: Decimal,
    pub payment_method: PaymentMethod,
    pub payment_type: PaymentType,
    pub received_date: DateTime<Utc>,
    pub check_number: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Payment method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethod {
    Cash,
    CreditCard,
    DebitCard,
    Check,
    Ach,
    Wire,
    Insurance,
}

/// Payment type (what it's for)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentType {
    Copay,
    Deductible,
    Coinsurance,
    PaymentInFull,
    PartialPayment,
    Refund,
}

/// Denial reason
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenialReason {
    pub code: String,
    pub description: String,
    pub category: DenialCategory,
}

/// Denial category
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DenialCategory {
    NotCovered,
    PriorAuthorization,
    Duplicate,
    TimelyFiling,
    LimitExceeded,
    MissingInformation,
    Other,
}

