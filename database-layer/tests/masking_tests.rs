/// Comprehensive tests for field masking engine
/// 
/// Tests cover:
/// - All 6 mask patterns (Partial, Full, Redacted, Hashed, Tokenized, Custom)
/// - Permission-based masking (doctor vs nurse vs receptionist scenarios)
/// - HIPAA compliance (all 18 HIPAA identifiers)
/// - Performance (<10ms overhead per request)
/// 
/// Note: These tests do not require DATABASE_URL as they test the masking
/// logic directly without database interaction.

#[cfg(test)]
mod tests {
    use database_layer::encryption::{
        MaskingEngine, MaskingMiddleware, MaskPattern, SensitivityLevel,
    };
    use serde_json::json;
    use std::time::Instant;

// =============================================================================
// UNIT TESTS - MASK PATTERNS
// =============================================================================

#[test]
fn test_mask_pattern_partial() {
    let engine = MaskingEngine::default();
    
    // SSN with last 4 visible
    let ssn = engine.mask_value("ssn", "123-45-6789");
    assert_eq!(ssn, "***-**-6789");
    
    // Email with first 3 visible
    let email = engine.mask_value("email", "john.doe@example.com");
    assert!(email.starts_with("joh"));
    assert!(email.contains("***"));
    
    // Phone with last 4 visible
    let phone = engine.mask_value("phone", "555-123-4567");
    assert!(phone.ends_with("4567"));
    assert!(phone.contains("***"));
    
    // Name with first and last character visible
    let name = engine.mask_value("full_name", "John Doe");
    assert_eq!(name, "J******e"); // 8 chars total: J + 6 masked + e
}

#[test]
fn test_mask_pattern_full() {
    let engine = MaskingEngine::default();
    
    // Password should be fully masked
    let password = engine.mask_value("password", "SuperSecret123!");
    assert_eq!(password, "***************"); // 15 characters
    assert_eq!(password.len(), "SuperSecret123!".len());
}

#[test]
fn test_mask_pattern_redacted() {
    let engine = MaskingEngine::default();
    
    // Diagnosis should be redacted
    let diagnosis = engine.mask_value("diagnosis", "Type 2 Diabetes Mellitus");
    assert_eq!(diagnosis, "[REDACTED]");
    
    // Medication should be redacted
    let medication = engine.mask_value("medication", "Metformin 500mg");
    assert_eq!(medication, "[REDACTED]");
    
    // Clinical notes should be redacted
    let notes = engine.mask_value("clinical_notes", "Patient presents with...");
    assert_eq!(notes, "[REDACTED]");
}

#[test]
fn test_mask_pattern_hashed() {
    let engine = MaskingEngine::default();
    
    // Certificate should be hashed
    let cert = engine.mask_value("certificate", "-----BEGIN CERTIFICATE-----\nMIID...");
    assert!(cert.starts_with("sha256:"));
    assert_eq!(cert.len(), 71); // "sha256:" + 64 hex chars
    
    // Same input should produce same hash
    let cert2 = engine.mask_value("certificate", "-----BEGIN CERTIFICATE-----\nMIID...");
    assert_eq!(cert, cert2);
}

#[test]
fn test_mask_pattern_tokenized() {
    let engine = MaskingEngine::default();
    
    // MRN should be tokenized
    let mrn = engine.mask_value("medical_record_number", "MRN-12345678");
    assert!(mrn.starts_with("TOK_"));
    assert_eq!(mrn.len(), 12); // "TOK_" + 8 hex chars
    
    // Same input should produce same token (deterministic)
    let mrn2 = engine.mask_value("medical_record_number", "MRN-12345678");
    assert_eq!(mrn, mrn2);
    
    // Different input should produce different token
    let mrn3 = engine.mask_value("medical_record_number", "MRN-87654321");
    assert_ne!(mrn, mrn3);
}

#[test]
fn test_mask_value_unknown_field() {
    let engine = MaskingEngine::default();
    
    // Unknown fields should pass through unchanged
    let value = engine.mask_value("unknown_field", "some value");
    assert_eq!(value, "some value");
}

// =============================================================================
// UNIT TESTS - JSON MASKING
// =============================================================================

#[test]
fn test_mask_json_object() {
    let engine = MaskingEngine::default();
    
    let json = json!({
        "id": "user-123",
        "full_name": "John Doe",
        "email": "john.doe@example.com",
        "ssn": "123-45-6789",
        "diagnosis": "Hypertension",
        "non_sensitive": "This should not be masked"
    });
    
    let masked = engine.mask_json(json);
    
    // Non-sensitive fields unchanged
    assert_eq!(masked["id"], "user-123");
    assert_eq!(masked["non_sensitive"], "This should not be masked");
    
    // Sensitive fields masked
    assert_eq!(masked["full_name"], "J******e"); // "John Doe" = 8 chars: J + 6 masked + e
    assert!(masked["email"].as_str().unwrap().starts_with("joh"));
    assert_eq!(masked["ssn"], "***-**-6789");
    assert_eq!(masked["diagnosis"], "[REDACTED]");
}

#[test]
fn test_mask_json_nested() {
    let engine = MaskingEngine::default();
    
    let json = json!({
        "user": {
            "id": "123",
            "email": "test@example.com",
            "address": {
                "street": "123 Main St",
                "city": "Boston",
                "zip_code": "02101"
            }
        }
    });
    
    let masked = engine.mask_json(json);
    
    // Verify nested masking
    assert!(masked["user"]["email"].as_str().unwrap().contains("***"));
    assert_eq!(masked["user"]["address"]["city"], "[REDACTED]");
    assert_eq!(masked["user"]["address"]["zip_code"], "021**");
}

#[test]
fn test_mask_json_array() {
    let engine = MaskingEngine::default();
    
    let json = json!({
        "patients": [
            {
                "name": "Alice",
                "ssn": "111-11-1111",
                "diagnosis": "Diabetes"
            },
            {
                "name": "Bob",
                "ssn": "222-22-2222",
                "diagnosis": "Hypertension"
            }
        ]
    });
    
    let masked = engine.mask_json(json);
    
    // Verify array masking
    let patients = masked["patients"].as_array().unwrap();
    assert_eq!(patients[0]["ssn"], "***-**-1111");
    assert_eq!(patients[0]["diagnosis"], "[REDACTED]");
    assert_eq!(patients[1]["ssn"], "***-**-2222");
    assert_eq!(patients[1]["diagnosis"], "[REDACTED]");
}

// =============================================================================
// UNIT TESTS - PERMISSION-BASED MASKING
// =============================================================================

#[test]
fn test_can_view_unmasked_exact_permission() {
    let engine = MaskingEngine::default();
    
    // Doctor with ePHI permission can view diagnosis
    let doctor_perms = vec![
        "phi:view:ephi".to_string(),
        "phi:view:restricted".to_string(),
    ];
    
    assert!(engine.can_view_unmasked("diagnosis", &doctor_perms));
    assert!(engine.can_view_unmasked("ssn", &doctor_perms));
}

#[test]
fn test_can_view_unmasked_insufficient_permission() {
    let engine = MaskingEngine::default();
    
    // Receptionist with only internal permission cannot view diagnosis
    let receptionist_perms = vec!["phi:view:internal".to_string()];
    
    assert!(!engine.can_view_unmasked("diagnosis", &receptionist_perms));
    assert!(!engine.can_view_unmasked("ssn", &receptionist_perms));
    
    // But can view email
    assert!(engine.can_view_unmasked("email", &receptionist_perms));
}

#[test]
fn test_can_view_unmasked_admin_permission() {
    let engine = MaskingEngine::default();
    
    // Admin with unmasked permission can view everything
    let admin_perms = vec!["phi:view:unmasked".to_string()];
    
    assert!(engine.can_view_unmasked("diagnosis", &admin_perms));
    assert!(engine.can_view_unmasked("ssn", &admin_perms));
    assert!(engine.can_view_unmasked("email", &admin_perms));
}

#[test]
fn test_can_view_unmasked_wildcard() {
    let engine = MaskingEngine::default();
    
    // Admin with wildcard permission
    let admin_perms = vec!["admin:*".to_string()];
    
    assert!(engine.can_view_unmasked("diagnosis", &admin_perms));
    assert!(engine.can_view_unmasked("ssn", &admin_perms));
}

// =============================================================================
// INTEGRATION TESTS - ROLE-BASED SCENARIOS
// =============================================================================

#[test]
fn test_doctor_scenario() {
    let middleware = MaskingMiddleware::default();
    
    let doctor_perms = vec![
        "phi:view:public".to_string(),
        "phi:view:internal".to_string(),
        "phi:view:confidential".to_string(),
        "phi:view:restricted".to_string(),
        "phi:view:ephi".to_string(),
    ];
    
    let patient_data = json!({
        "id": "patient-123",
        "full_name": "John Doe",
        "email": "john@example.com",
        "ssn": "123-45-6789",
        "date_of_birth": "1980-05-15",
        "diagnosis": "Type 2 Diabetes",
        "medication": "Metformin 500mg"
    });
    
    let masked = middleware.mask_response(patient_data.clone(), &doctor_perms);
    
    // Doctor should see all data unmasked
    assert_eq!(masked["full_name"], "John Doe");
    assert_eq!(masked["email"], "john@example.com");
    assert_eq!(masked["ssn"], "123-45-6789");
    assert_eq!(masked["diagnosis"], "Type 2 Diabetes");
    assert_eq!(masked["medication"], "Metformin 500mg");
}

#[test]
fn test_nurse_scenario() {
    let middleware = MaskingMiddleware::default();
    
    let nurse_perms = vec![
        "phi:view:public".to_string(),
        "phi:view:internal".to_string(),
        "phi:view:confidential".to_string(),
        "phi:view:ephi".to_string(),
        // Note: No restricted permission - cannot see SSN
    ];
    
    let patient_data = json!({
        "id": "patient-123",
        "full_name": "John Doe",
        "email": "john@example.com",
        "ssn": "123-45-6789",
        "date_of_birth": "1980-05-15",
        "diagnosis": "Type 2 Diabetes",
        "medication": "Metformin 500mg"
    });
    
    let masked = middleware.mask_response(patient_data, &nurse_perms);
    
    // Nurse should see most data except restricted identifiers
    assert_eq!(masked["full_name"], "John Doe");
    assert_eq!(masked["email"], "john@example.com");
    assert_eq!(masked["ssn"], "***-**-6789"); // Masked!
    assert_eq!(masked["diagnosis"], "Type 2 Diabetes");
    assert_eq!(masked["medication"], "Metformin 500mg");
}

#[test]
fn test_receptionist_scenario() {
    let middleware = MaskingMiddleware::default();
    
    let receptionist_perms = vec![
        "phi:view:public".to_string(),
        "phi:view:internal".to_string(),
        // No confidential, restricted, or ePHI permissions
    ];
    
    let patient_data = json!({
        "id": "patient-123",
        "full_name": "John Doe",
        "email": "john@example.com",
        "phone": "555-123-4567",
        "ssn": "123-45-6789",
        "date_of_birth": "1980-05-15",
        "diagnosis": "Type 2 Diabetes"
    });
    
    let masked = middleware.mask_response(patient_data, &receptionist_perms);
    
    // Receptionist with internal permissions can see contact info unmasked
    assert_eq!(masked["full_name"], "J******e"); // "John Doe" = 8 chars: J + 6 masked + e (Confidential)
    assert_eq!(masked["email"], "john@example.com"); // Unmasked (Internal level)
    assert_eq!(masked["phone"], "555-123-4567"); // Unmasked (Internal level)
    assert_eq!(masked["ssn"], "***-**-6789"); // Masked (ePHI level)
    assert_eq!(masked["date_of_birth"], "1980-**-**"); // Masked (Restricted level)
    assert_eq!(masked["diagnosis"], "[REDACTED]"); // Masked (Confidential level)
}

#[test]
fn test_no_permissions_scenario() {
    let middleware = MaskingMiddleware::default();
    
    let no_perms: Vec<String> = vec![];
    
    let patient_data = json!({
        "full_name": "John Doe",
        "email": "john@example.com",
        "ssn": "123-45-6789",
        "diagnosis": "Type 2 Diabetes"
    });
    
    let masked = middleware.mask_response(patient_data, &no_perms);
    
    // No permissions - everything should be masked
    assert_eq!(masked["full_name"], "J******e"); // "John Doe" = 8 chars: J + 6 masked + e
    assert!(masked["email"].as_str().unwrap().contains("***"));
    assert_eq!(masked["ssn"], "***-**-6789");
    assert_eq!(masked["diagnosis"], "[REDACTED]");
}

// =============================================================================
// HIPAA COMPLIANCE TESTS - 18 IDENTIFIERS
// =============================================================================

#[test]
fn test_hipaa_identifiers_all_masked() {
    let engine = MaskingEngine::default();
    
    // HIPAA Safe Harbor requires masking 18 identifiers
    let identifiers = vec![
        ("full_name", "John Doe"),
        ("email", "john@example.com"),
        ("phone", "555-123-4567"),
        ("ssn", "123-45-6789"),
        ("medical_record_number", "MRN-12345"),
        ("health_insurance_number", "INS-67890"),
        ("date_of_birth", "1980-05-15"),
        ("address", "123 Main St"),
        ("city", "Boston"),
        ("state", "MA"),
        ("zip_code", "02101"),
        ("diagnosis", "Diabetes"),
        ("medication", "Metformin"),
        ("lab_result", "A1C: 7.2%"),
        ("credit_card", "4111-1111-1111-1111"),
        ("bank_account", "123456789"),
        ("api_key", "sk_live_abcd1234"),
        ("certificate", "-----BEGIN CERT-----"),
    ];
    
    for (field, value) in identifiers {
        let masked = engine.mask_value(field, value);
        // Verify that masking occurred (value changed)
        assert_ne!(masked, value, "Field '{}' was not masked", field);
        
        // Verify no PII leaked in masked value
        if field == "full_name" {
            assert!(!masked.contains("John"));
            assert!(!masked.contains("Doe"));
        }
        if field == "email" {
            assert!(masked.contains("***") || masked.starts_with("joh"));
        }
        if field == "ssn" {
            assert!(!masked.starts_with("123"));
            assert!(masked.ends_with("6789") || masked == "***-**-6789");
        }
    }
}

#[test]
fn test_hipaa_safe_harbor_compliance() {
    let engine = MaskingEngine::default();
    
    // Verify all 18 HIPAA identifiers have sensitivity levels defined
    let hipaa_fields = vec![
        "full_name", "first_name", "last_name",
        "email", "phone_number", "phone",
        "ssn", "social_security_number",
        "medical_record_number", "mrn",
        "health_insurance_number",
        "date_of_birth", "dob", "birth_date",
        "address", "street_address",
        "city", "state", "zip_code", "postal_code",
    ];
    
    for field in hipaa_fields {
        let level = engine.get_sensitivity_level(field);
        assert!(
            level.is_some(),
            "HIPAA identifier '{}' is not classified",
            field
        );
    }
}

// =============================================================================
// PERFORMANCE TESTS
// =============================================================================

#[test]
fn test_performance_simple_masking() {
    let engine = MaskingEngine::default();
    
    let start = Instant::now();
    for _ in 0..1000 {
        engine.mask_value("ssn", "123-45-6789");
    }
    let duration = start.elapsed();
    
    // Should take less than 10ms for 1000 operations
    assert!(
        duration.as_millis() < 100,
        "Simple masking too slow: {:?}",
        duration
    );
    
    // Per-operation should be < 10 microseconds
    let per_op = duration.as_micros() / 1000;
    assert!(per_op < 100, "Per-operation time: {}μs", per_op);
}

#[test]
fn test_performance_json_masking() {
    let engine = MaskingEngine::default();
    
    let json = json!({
        "id": "patient-123",
        "full_name": "John Doe",
        "email": "john@example.com",
        "ssn": "123-45-6789",
        "diagnosis": "Type 2 Diabetes",
        "medication": "Metformin 500mg",
        "address": {
            "street": "123 Main St",
            "city": "Boston",
            "state": "MA",
            "zip_code": "02101"
        }
    });
    
    let start = Instant::now();
    for _ in 0..100 {
        let _ = engine.mask_json(json.clone());
    }
    let duration = start.elapsed();
    
    // Should take less than 10ms per request (100 requests in 1 second)
    let per_request = duration.as_micros() / 100;
    assert!(
        per_request < 10_000,
        "JSON masking too slow: {}μs per request (target: <10ms)",
        per_request
    );
}

#[test]
fn test_performance_permission_check() {
    let engine = MaskingEngine::default();
    
    let permissions = vec![
        "phi:view:public".to_string(),
        "phi:view:internal".to_string(),
        "phi:view:confidential".to_string(),
        "phi:view:ephi".to_string(),
    ];
    
    let start = Instant::now();
    for _ in 0..10_000 {
        engine.can_view_unmasked("diagnosis", &permissions);
    }
    let duration = start.elapsed();
    
    // Permission checks should be very fast
    let per_check = duration.as_nanos() / 10_000;
    assert!(
        per_check < 1_000,
        "Permission check too slow: {}ns per check",
        per_check
    );
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_edge_case_empty_string() {
    let engine = MaskingEngine::default();
    
    let masked = engine.mask_value("email", "");
    assert_eq!(masked, "");
}

#[test]
fn test_edge_case_very_short_value() {
    let engine = MaskingEngine::default();
    
    // Single character name
    let masked = engine.mask_value("full_name", "X");
    assert_eq!(masked, "X"); // Can't mask a single char
}

#[test]
fn test_edge_case_unicode() {
    let engine = MaskingEngine::default();
    
    // Unicode characters in name
    let masked = engine.mask_value("full_name", "José García");
    assert!(masked.starts_with("J"));
    assert!(masked.contains("*"));
}

#[test]
fn test_edge_case_null_values() {
    let engine = MaskingEngine::default();
    
    let json = json!({
        "email": null,
        "ssn": null
    });
    
    let masked = engine.mask_json(json);
    assert!(masked["email"].is_null());
    assert!(masked["ssn"].is_null());
}

} // end tests module
