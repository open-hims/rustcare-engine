use crate::models::AccountsReceivable;
use crate::error::AccountingResult;
use uuid::Uuid;

/// Accounts Receivable service
pub struct AccountsReceivableService;

impl AccountsReceivableService {
    /// Create a new A/R service
    pub fn new() -> Self {
        Self
    }

    /// Get A/R for a patient
    pub async fn get_patient_ar(&self, patient_id: Uuid) -> AccountingResult<AccountsReceivable> {
        // TODO: Implement patient A/R lookup
        Ok(AccountsReceivable {
            id: Uuid::new_v4(),
            patient_id,
            account_id: Uuid::new_v4(),
            current_balance: rust_decimal::Decimal::ZERO,
            age_0_30: rust_decimal::Decimal::ZERO,
            age_31_60: rust_decimal::Decimal::ZERO,
            age_61_90: rust_decimal::Decimal::ZERO,
            age_91_plus: rust_decimal::Decimal::ZERO,
            total_due: rust_decimal::Decimal::ZERO,
            last_payment_date: None,
            last_charge_date: None,
        })
    }

    /// Update A/R balance
    pub async fn update_balance(&self, patient_id: Uuid, amount: rust_decimal::Decimal) -> AccountingResult<()> {
        // TODO: Implement balance update
        Ok(())
    }

    /// Get aging report
    pub async fn get_aging_report(&self, organization_id: Uuid) -> AccountingResult<Vec<AccountsReceivable>> {
        // TODO: Implement aging calculation
        Ok(vec![])
    }
}

impl Default for AccountsReceivableService {
    fn default() -> Self {
        Self::new()
    }
}

