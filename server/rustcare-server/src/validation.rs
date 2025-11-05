//! Request validation utilities for consistent validation across handlers
//!
//! This module provides a `RequestValidation` trait and helper macros to
//! centralize validation logic and ensure consistent error messages.

use crate::error::ApiError;

/// Trait for validating request payloads
///
/// Implement this trait for all create/update request types to ensure
/// consistent validation across the API.
///
/// # Example
///
/// ```rust
/// use crate::validation::RequestValidation;
/// use crate::error::ApiError;
///
/// struct CreateUserRequest {
///     email: String,
///     name: String,
/// }
///
/// impl RequestValidation for CreateUserRequest {
///     fn validate(&self) -> Result<(), ApiError> {
///         validate_field!(self.email, !self.email.trim().is_empty(), "Email is required");
///         validate_field!(self.email, self.email.contains('@'), "Invalid email format");
///         validate_field!(self.name, !self.name.trim().is_empty(), "Name is required");
///         validate_field!(self.name, self.name.len() >= 2, "Name must be at least 2 characters");
///         Ok(())
///     }
/// }
/// ```
pub trait RequestValidation {
    /// Validates the request and returns an error if validation fails
    ///
    /// Returns `Ok(())` if validation passes, or `Err(ApiError)` with
    /// a validation error message if validation fails.
    fn validate(&self) -> Result<(), ApiError>;
}

/// Macro for validating fields with custom predicates
///
/// # Usage
///
/// ```rust
/// validate_field!(self.email, !self.email.trim().is_empty(), "Email is required");
/// validate_field!(self.age, self.age >= 18, "User must be at least 18 years old");
/// ```
#[macro_export]
macro_rules! validate_field {
    ($field:expr, $predicate:expr, $message:expr) => {
        if !$predicate {
            return Err($crate::error::ApiError::validation($message));
        }
    };
}

/// Macro for validating required fields (non-empty strings)
///
/// # Usage
///
/// ```rust
/// validate_required!(self.name, "Name is required");
/// validate_required!(self.email, "Email is required");
/// ```
#[macro_export]
macro_rules! validate_required {
    ($field:expr, $message:expr) => {
        validate_field!($field, !$field.trim().is_empty(), $message);
    };
}

/// Macro for validating UUID fields (non-nil)
///
/// # Usage
///
/// ```rust
/// validate_uuid!(self.patient_id, "Patient ID is required");
/// validate_uuid!(self.organization_id, "Organization ID is required");
/// ```
#[macro_export]
macro_rules! validate_uuid {
    ($field:expr, $message:expr) => {
        validate_field!($field, !$field.is_nil(), $message);
    };
}

/// Macro for validating string length
///
/// # Usage
///
/// ```rust
/// validate_length!(self.name, 2, 100, "Name must be between 2 and 100 characters");
/// ```
#[macro_export]
macro_rules! validate_length {
    ($field:expr, $min:expr, $max:expr, $message:expr) => {
        let len = $field.len();
        validate_field!($field, len >= $min && len <= $max, $message);
    };
}

/// Macro for validating email format (basic check)
///
/// # Usage
///
/// ```rust
/// validate_email!(self.email, "Invalid email format");
/// ```
#[macro_export]
macro_rules! validate_email {
    ($field:expr, $message:expr) => {
        validate_field!($field, $field.contains('@') && $field.contains('.'), $message);
    };
}

/// Macro for validating numeric ranges
///
/// # Usage
///
/// ```rust
/// validate_range!(self.age, 0, 150, "Age must be between 0 and 150");
/// ```
#[macro_export]
macro_rules! validate_range {
    ($field:expr, $min:expr, $max:expr, $message:expr) => {
        validate_field!($field, *$field >= $min && *$field <= $max, $message);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ApiError;

    struct TestRequest {
        name: String,
        email: String,
        age: u32,
    }

    impl RequestValidation for TestRequest {
        fn validate(&self) -> Result<(), ApiError> {
            validate_required!(self.name, "Name is required");
            validate_length!(self.name, 2, 100, "Name must be between 2 and 100 characters");
            validate_email!(self.email, "Invalid email format");
            validate_range!(self.age, 0, 150, "Age must be between 0 and 150");
            Ok(())
        }
    }

    #[test]
    fn test_validation_success() {
        let request = TestRequest {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_name() {
        let request = TestRequest {
            name: "".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_validation_invalid_email() {
        let request = TestRequest {
            name: "John Doe".to_string(),
            email: "invalid-email".to_string(),
            age: 30,
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_validation_age_out_of_range() {
        let request = TestRequest {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 200,
        };
        assert!(request.validate().is_err());
    }
}

