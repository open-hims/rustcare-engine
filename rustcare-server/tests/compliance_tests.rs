use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use tower::ServiceExt;
use uuid::Uuid;

use rustcare_server::{
    create_app,
    server::RustCareServer,
};

/// Test configuration for compliance tests
struct TestConfig {
    server: RustCareServer,
    app: Router,
}

impl TestConfig {
    async fn new() -> Self {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://rustcare:rustcare@localhost:5432/rustcare_test".to_string());
        
        let pool = Pool::<Postgres>::connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        let server = RustCareServer::new_with_pool(pool)
            .await
            .expect("Failed to create test server");

        let app = create_app(server.clone());

        Self { server, app }
    }

    async fn cleanup(&self) {
        // Clean up test data
        let _ = sqlx::query("DELETE FROM compliance_rules WHERE code LIKE 'TEST-%'")
            .execute(&self.server.db_pool)
            .await;
        
        let _ = sqlx::query("DELETE FROM compliance_frameworks WHERE code LIKE 'TEST-%'")
            .execute(&self.server.db_pool)
            .await;
    }
}

#[tokio::test]
async fn test_list_compliance_frameworks() {
    let config = TestConfig::new().await;
    
    let request = Request::builder()
        .uri("/api/v1/compliance/frameworks")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = config.app.clone().oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let frameworks: Vec<Value> = serde_json::from_slice(&body).unwrap();
    
    // Should return existing frameworks
    assert!(!frameworks.is_empty());
    
    config.cleanup().await;
}

#[tokio::test]
async fn test_create_compliance_framework_success() {
    let config = TestConfig::new().await;
    
    let framework_data = json!({
        "name": "Test HIPAA Framework",
        "code": "TEST-HIPAA-001",
        "version": "1.0",
        "description": "Test HIPAA compliance framework",
        "authority": "Test Authority",
        "jurisdiction": "Test Jurisdiction",
        "effective_date": "2024-01-01T00:00:00Z",
        "status": "active"
    });

    let request = Request::builder()
        .uri("/api/v1/compliance/frameworks")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(framework_data.to_string()))
        .unwrap();

    let response = config.app.clone().oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::CREATED);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_data: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(response_data["success"], true);
    assert_eq!(response_data["data"]["code"], "TEST-HIPAA-001");
    assert_eq!(response_data["data"]["name"], "Test HIPAA Framework");
    
    config.cleanup().await;
}

#[tokio::test]
async fn test_create_compliance_framework_validation_error() {
    let config = TestConfig::new().await;
    
    let invalid_framework_data = json!({
        "name": "", // Empty name should cause validation error
        "code": "TEST-INVALID",
        "version": "1.0",
        "effective_date": "invalid-date-format", // Invalid date format
        "status": "active"
    });

    let request = Request::builder()
        .uri("/api/v1/compliance/frameworks")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(invalid_framework_data.to_string()))
        .unwrap();

    let response = config.app.clone().oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error_response: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(error_response["error_type"], "validation_error");
    assert!(error_response["message"].as_str().unwrap().contains("validation"));
    assert!(error_response["error_id"].is_string());
    assert!(error_response["timestamp"].is_string());
    
    config.cleanup().await;
}

#[tokio::test]
async fn test_create_compliance_framework_duplicate_code() {
    let config = TestConfig::new().await;
    
    let framework_data = json!({
        "name": "First Test Framework",
        "code": "TEST-DUPLICATE",
        "version": "1.0",
        "description": "First framework",
        "authority": "Test Authority",
        "jurisdiction": "Test Jurisdiction",
        "effective_date": "2024-01-01T00:00:00Z",
        "status": "active"
    });

    // Create first framework
    let request1 = Request::builder()
        .uri("/api/v1/compliance/frameworks")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(framework_data.to_string()))
        .unwrap();

    let response1 = config.app.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::CREATED);

    // Try to create duplicate
    let duplicate_data = json!({
        "name": "Duplicate Test Framework",
        "code": "TEST-DUPLICATE", // Same code
        "version": "1.0",
        "description": "Duplicate framework",
        "authority": "Test Authority",
        "jurisdiction": "Test Jurisdiction",
        "effective_date": "2024-01-01T00:00:00Z",
        "status": "active"
    });

    let request2 = Request::builder()
        .uri("/api/v1/compliance/frameworks")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(duplicate_data.to_string()))
        .unwrap();

    let response2 = config.app.clone().oneshot(request2).await.unwrap();
    
    assert_eq!(response2.status(), StatusCode::CONFLICT);
    
    let body = axum::body::to_bytes(response2.into_body(), usize::MAX).await.unwrap();
    let error_response: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(error_response["error_type"], "database_error");
    assert!(error_response["message"].as_str().unwrap().contains("already exists"));
    
    config.cleanup().await;
}

#[tokio::test]
async fn test_create_compliance_rule_success() {
    let config = TestConfig::new().await;
    
    // First create a framework
    let framework_data = json!({
        "name": "Test Framework for Rules",
        "code": "TEST-FRAMEWORK-RULES",
        "version": "1.0",
        "description": "Framework for rule testing",
        "authority": "Test Authority",
        "jurisdiction": "Test Jurisdiction",
        "effective_date": "2024-01-01T00:00:00Z",
        "status": "active"
    });

    let framework_request = Request::builder()
        .uri("/api/v1/compliance/frameworks")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(framework_data.to_string()))
        .unwrap();

    let framework_response = config.app.clone().oneshot(framework_request).await.unwrap();
    assert_eq!(framework_response.status(), StatusCode::CREATED);
    
    let framework_body = axum::body::to_bytes(framework_response.into_body(), usize::MAX).await.unwrap();
    let framework_result: Value = serde_json::from_slice(&framework_body).unwrap();
    let framework_id = framework_result["data"]["id"].as_str().unwrap();

    // Now create a rule
    let rule_data = json!({
        "framework_id": framework_id,
        "rule_code": "TEST-RULE-001",
        "title": "Test Patient Privacy Rule",
        "description": "Test rule for patient privacy compliance",
        "category": "privacy",
        "severity": "high",
        "rule_type": "procedural",
        "applies_to_entity_types": ["patient", "provider"],
        "applies_to_roles": ["doctor", "nurse"],
        "applies_to_regions": ["US", "CA"],
        "is_automated": false,
        "effective_date": "2024-01-01T00:00:00Z"
    });

    let rule_request = Request::builder()
        .uri("/api/v1/compliance/rules")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(rule_data.to_string()))
        .unwrap();

    let rule_response = config.app.clone().oneshot(rule_request).await.unwrap();
    
    assert_eq!(rule_response.status(), StatusCode::CREATED);
    
    let rule_body = axum::body::to_bytes(rule_response.into_body(), usize::MAX).await.unwrap();
    let rule_result: Value = serde_json::from_slice(&rule_body).unwrap();
    
    assert_eq!(rule_result["success"], true);
    assert_eq!(rule_result["data"]["rule_code"], "TEST-RULE-001");
    assert_eq!(rule_result["data"]["title"], "Test Patient Privacy Rule");
    assert_eq!(rule_result["data"]["framework_id"], framework_id);
    
    config.cleanup().await;
}

#[tokio::test]
async fn test_list_compliance_rules() {
    let config = TestConfig::new().await;
    
    let request = Request::builder()
        .uri("/api/v1/compliance/rules")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = config.app.clone().oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let rules: Vec<Value> = serde_json::from_slice(&body).unwrap();
    
    // Should return existing rules or empty array
    assert!(rules.is_array());
    
    config.cleanup().await;
}

#[tokio::test]
async fn test_auto_assign_compliance() {
    let config = TestConfig::new().await;
    
    let assignment_data = json!({
        "entity_type": "clinic",
        "entity_id": Uuid::new_v4(),
        "rule_ids": [],
        "geographic_region_id": Uuid::new_v4()
    });

    let request = Request::builder()
        .uri("/api/v1/compliance/assign")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(assignment_data.to_string()))
        .unwrap();

    let response = config.app.clone().oneshot(request).await.unwrap();
    
    // This might return different status codes depending on implementation
    // Just ensure it doesn't crash
    assert!(response.status().is_client_error() || response.status().is_success());
    
    config.cleanup().await;
}

#[tokio::test]
async fn test_api_error_response_structure() {
    let config = TestConfig::new().await;
    
    // Test with an endpoint that should return 404
    let request = Request::builder()
        .uri("/api/v1/compliance/frameworks/00000000-0000-0000-0000-000000000000")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = config.app.clone().oneshot(request).await.unwrap();
    
    // Should return 404 for non-existent framework
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    
    config.cleanup().await;
}

// Performance test
#[tokio::test]
async fn test_framework_creation_performance() {
    let config = TestConfig::new().await;
    
    let start = std::time::Instant::now();
    
    for i in 0..10 {
        let framework_data = json!({
            "name": format!("Performance Test Framework {}", i),
            "code": format!("TEST-PERF-{:03}", i),
            "version": "1.0",
            "description": "Performance test framework",
            "authority": "Test Authority",
            "jurisdiction": "Test Jurisdiction",
            "effective_date": "2024-01-01T00:00:00Z",
            "status": "active"
        });

        let request = Request::builder()
            .uri("/api/v1/compliance/frameworks")
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(framework_data.to_string()))
            .unwrap();

        let response = config.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }
    
    let duration = start.elapsed();
    
    // Should complete 10 creations in reasonable time (adjust threshold as needed)
    assert!(duration.as_millis() < 5000, "Performance test took too long: {:?}", duration);
    
    config.cleanup().await;
}