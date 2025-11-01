use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Chart of Accounts - Account definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartOfAccounts {
    pub id: Uuid,
    pub code: String, // e.g., "4000", "4010", "4100"
    pub name: String,
    pub account_type: AccountType,
    pub parent_account_id: Option<Uuid>,
    pub is_active: bool,
}

/// Account type in accounting
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

/// General Ledger entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralLedgerEntry {
    pub id: Uuid,
    pub account_id: Uuid,
    pub entry_date: DateTime<Utc>,
    pub journal_entry_id: Uuid,
    pub description: String,
    pub debit_amount: Decimal,
    pub credit_amount: Decimal,
    pub balance: Decimal,
    pub reference_type: String,
    pub reference_id: Uuid,
}

/// Journal entry (double-entry bookkeeping)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: Uuid,
    pub entry_number: String,
    pub entry_date: DateTime<Utc>,
    pub description: String,
    pub ledger_entries: Vec<GeneralLedgerEntry>,
    pub total_debits: Decimal,
    pub total_credits: Decimal,
    pub is_balanced: bool,
    pub status: JournalStatus,
}

/// Journal entry status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JournalStatus {
    Draft,
    Posted,
    Reversed,
}

/// Accounts Receivable balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountsReceivable {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub account_id: Uuid,
    pub current_balance: Decimal,
    pub age_0_30: Decimal,
    pub age_31_60: Decimal,
    pub age_61_90: Decimal,
    pub age_91_plus: Decimal,
    pub total_due: Decimal,
    pub last_payment_date: Option<DateTime<Utc>>,
    pub last_charge_date: Option<DateTime<Utc>>,
}

/// Financial report period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialReport {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub report_type: ReportType,
    pub revenue: Decimal,
    pub expenses: Decimal,
    pub net_income: Decimal,
    pub accounts_receivable: Decimal,
    pub cash_on_hand: Decimal,
}

/// Report type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportType {
    IncomeStatement,
    BalanceSheet,
    CashFlow,
    Receivables,
    Aging,
}

