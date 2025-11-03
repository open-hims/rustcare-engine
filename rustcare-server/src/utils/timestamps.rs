//! Timestamp utilities for consistent timestamp handling across the codebase
//!
//! This module provides utilities to standardize timestamp formats and eliminate
//! inconsistencies between DateTime<Utc>, String, and RFC3339 formats.

use chrono::{DateTime, Utc, NaiveDate};
use serde::{Serialize, Serializer};
use crate::error::ApiError;

/// Wrapper type for consistent timestamp serialization
///
/// This type ensures all timestamps in API responses are serialized as RFC3339 strings
/// while maintaining type safety with DateTime<Utc> internally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiTimestamp(pub DateTime<Utc>);

impl ApiTimestamp {
    /// Create a new ApiTimestamp from the current time
    pub fn now() -> Self {
        Self(Utc::now())
    }
    
    /// Create from a DateTime<Utc>
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
    
    /// Parse from RFC3339 string
    pub fn from_rfc3339(s: &str) -> Result<Self, ApiError> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| Self(dt.with_timezone(&Utc)))
            .map_err(|_| ApiError::validation("Invalid RFC3339 timestamp format. Expected format: YYYY-MM-DDTHH:MM:SSZ"))
    }
    
    /// Get the inner DateTime<Utc>
    pub fn to_datetime(self) -> DateTime<Utc> {
        self.0
    }
    
    /// Convert to RFC3339 string
    pub fn to_rfc3339(self) -> String {
        self.0.to_rfc3339()
    }
}

impl Serialize for ApiTimestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_rfc3339())
    }
}

impl From<DateTime<Utc>> for ApiTimestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        ApiTimestamp(dt)
    }
}

impl From<ApiTimestamp> for DateTime<Utc> {
    fn from(ts: ApiTimestamp) -> Self {
        ts.0
    }
}

impl Default for ApiTimestamp {
    fn default() -> Self {
        Self::now()
    }
}

// Utility functions for common operations

/// Get current time as RFC3339 string
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

/// Parse RFC3339 string to NaiveDate (for date-only fields)
pub fn parse_rfc3339_to_naive_date(s: &str) -> Result<NaiveDate, ApiError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.date_naive())
        .map_err(|_| ApiError::validation("Invalid RFC3339 date format. Expected format: YYYY-MM-DDTHH:MM:SSZ"))
}

/// Convert NaiveDate to RFC3339 string (with time set to 00:00:00 UTC)
pub fn naive_date_to_rfc3339(naive: NaiveDate) -> String {
    naive.and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .to_rfc3339()
}

/// Convert DateTime<Utc> to RFC3339 string
pub fn date_to_rfc3339(date: DateTime<Utc>) -> String {
    date.to_rfc3339()
}

/// Parse RFC3339 to DateTime<Utc>
pub fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, ApiError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| ApiError::validation("Invalid RFC3339 timestamp format. Expected format: YYYY-MM-DDTHH:MM:SSZ"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_api_timestamp_serialization() {
        let ts = ApiTimestamp::now();
        let json = serde_json::to_string(&ts).unwrap();
        assert!(json.starts_with('"') && json.ends_with('"'));
        assert!(json.contains('T') || json.contains('-'));
    }

    #[test]
    fn test_api_timestamp_from_datetime() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
        let ts = ApiTimestamp::from_datetime(dt);
        assert_eq!(ts.to_rfc3339(), "2024-01-15T10:30:00+00:00");
    }

    #[test]
    fn test_api_timestamp_from_rfc3339() {
        let s = "2024-01-15T10:30:00Z";
        let result = ApiTimestamp::from_rfc3339(s);
        assert!(result.is_ok());
        let ts = result.unwrap();
        assert_eq!(ts.to_rfc3339(), "2024-01-15T10:30:00+00:00");
    }

    #[test]
    fn test_api_timestamp_from_rfc3339_invalid() {
        let s = "invalid-date";
        let result = ApiTimestamp::from_rfc3339(s);
        assert!(result.is_err());
    }

    #[test]
    fn test_api_timestamp_from_rfc3339_with_timezone() {
        let s = "2024-01-15T10:30:00+05:00";
        let result = ApiTimestamp::from_rfc3339(s);
        assert!(result.is_ok());
        // Should convert to UTC
        let ts = result.unwrap();
        assert!(ts.to_rfc3339().contains("+00:00") || ts.to_rfc3339().ends_with('Z'));
    }

    #[test]
    fn test_api_timestamp_to_datetime() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
        let ts = ApiTimestamp::from_datetime(dt);
        let converted = ts.to_datetime();
        assert_eq!(converted, dt);
    }

    #[test]
    fn test_api_timestamp_from_trait() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
        let ts: ApiTimestamp = dt.into();
        assert_eq!(ts.to_rfc3339(), "2024-01-15T10:30:00+00:00");
    }

    #[test]
    fn test_api_timestamp_into_datetime() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
        let ts = ApiTimestamp::from_datetime(dt);
        let converted: DateTime<Utc> = ts.into();
        assert_eq!(converted, dt);
    }

    #[test]
    fn test_api_timestamp_default() {
        let ts = ApiTimestamp::default();
        // Default should be current time, so we just verify it's created
        assert!(!ts.to_rfc3339().is_empty());
    }

    #[test]
    fn test_parse_rfc3339() {
        let s = "2024-01-15T10:30:00Z";
        let result = parse_rfc3339(s);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2024-01-15 10:30:00");
    }

    #[test]
    fn test_parse_rfc3339_invalid() {
        let s = "invalid-date";
        let result = parse_rfc3339(s);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_rfc3339_to_naive_date() {
        let s = "2024-01-15T10:30:00Z";
        let result = parse_rfc3339_to_naive_date(s);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().format("%Y-%m-%d").to_string(), "2024-01-15");
    }

    #[test]
    fn test_parse_rfc3339_to_naive_date_invalid() {
        let s = "invalid-date";
        let result = parse_rfc3339_to_naive_date(s);
        assert!(result.is_err());
    }

    #[test]
    fn test_naive_date_to_rfc3339() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let result = naive_date_to_rfc3339(date);
        assert!(result.contains("2024-01-15"));
        assert!(result.contains("T00:00:00") || result.contains("T00:00:00Z"));
    }

    #[test]
    fn test_date_to_rfc3339() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
        let result = date_to_rfc3339(dt);
        assert_eq!(result, "2024-01-15T10:30:00+00:00");
    }

    #[test]
    fn test_now_rfc3339() {
        let result = now_rfc3339();
        assert!(!result.is_empty());
        assert!(result.contains('T'));
    }

    #[test]
    fn test_api_timestamp_equality() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
        let ts1 = ApiTimestamp::from_datetime(dt);
        let ts2 = ApiTimestamp::from_datetime(dt);
        assert_eq!(ts1, ts2);
    }

    #[test]
    fn test_api_timestamp_clone() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
        let ts1 = ApiTimestamp::from_datetime(dt);
        let ts2 = ts1.clone();
        assert_eq!(ts1, ts2);
    }
}

