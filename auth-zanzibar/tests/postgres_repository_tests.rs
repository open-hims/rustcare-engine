//! PostgreSQL Repository Tests
//! 
//! To run these tests:
//! 1. Ensure PostgreSQL is running on port 5433
//! 2. Run migrations: sqlx migrate run
//! 3. cargo test --test postgres_repository_tests -- --test-threads=1

use auth_zanzibar::*;
use auth_zanzibar::repository::TupleRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn setup_test_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://rustcare:We4rpJVJ0PUUWBj21q1FDIWgXT7mCz@localhost:5433/rustcare_dev".to_string());
    
    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn cleanup_test_data(pool: &PgPool) {
    sqlx::query("DELETE FROM zanzibar_tuples WHERE subject_id LIKE 'test_%'")
        .execute(pool)
        .await
        .expect("Failed to cleanup test data");
}

#[tokio::test]
#[ignore] // Run with: cargo test postgres_write_read_tuple -- --ignored
async fn test_postgres_write_read_tuple() {
    let pool = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let repo = repository::PostgresTupleRepository::new(pool.clone());

    let tuple = Tuple::new(
        Subject::user("test_alice"),
        Relation::new("viewer"),
        Object::new("patient_record", "test_101"),
    );

    // Write tuple
    repo.write_tuple(tuple.clone()).await.unwrap();

    // Check existence
    let exists = repo.tuple_exists(&tuple).await.unwrap();
    assert!(exists, "Tuple should exist after write");

    // Read back
    let tuples = repo.read_tuples(
        Some(Subject::user("test_alice")),
        None,
        None,
    ).await.unwrap();

    assert!(!tuples.is_empty(), "Should find at least one tuple");
    assert_eq!(tuples[0].relation.name, "viewer");
    assert_eq!(tuples[0].object.object_id, "test_101");

    // Cleanup
    cleanup_test_data(&pool).await;
    println!("✅ PostgreSQL write/read test PASSED");
}

#[tokio::test]
#[ignore]
async fn test_postgres_delete_tuple() {
    let pool = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let repo = repository::PostgresTupleRepository::new(pool.clone());

    let tuple = Tuple::new(
        Subject::user("test_bob"),
        Relation::new("editor"),
        Object::new("document", "test_doc1"),
    );

    // Write then delete
    repo.write_tuple(tuple.clone()).await.unwrap();
    assert!(repo.tuple_exists(&tuple).await.unwrap());

    repo.delete_tuple(tuple.clone()).await.unwrap();
    assert!(!repo.tuple_exists(&tuple).await.unwrap(), "Tuple should not exist after delete");

    cleanup_test_data(&pool).await;
    println!("✅ PostgreSQL delete test PASSED");
}

#[tokio::test]
#[ignore]
async fn test_postgres_batch_write() {
    let pool = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let repo = repository::PostgresTupleRepository::new(pool.clone());

    let tuples = vec![
        Tuple::new(Subject::user("test_user1"), Relation::new("viewer"), Object::new("patient_record", "test_p1")),
        Tuple::new(Subject::user("test_user1"), Relation::new("viewer"), Object::new("patient_record", "test_p2")),
        Tuple::new(Subject::user("test_user2"), Relation::new("viewer"), Object::new("patient_record", "test_p3")),
    ];

    let request = WriteRequest {
        writes: tuples.clone(),
        deletes: vec![],
    };

    // Batch write
    repo.batch_write(request).await.unwrap();

    // Verify all written
    for tuple in &tuples {
        assert!(repo.tuple_exists(tuple).await.unwrap(), "Batch written tuple should exist");
    }

    cleanup_test_data(&pool).await;
    println!("✅ PostgreSQL batch write test PASSED");
}

#[tokio::test]
#[ignore]
async fn test_postgres_filter_by_relation() {
    let pool = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let repo = repository::PostgresTupleRepository::new(pool.clone());

    // Write tuples with different relations
    repo.write_tuple(Tuple::new(
        Subject::user("test_charlie"),
        Relation::new("viewer"),
        Object::new("patient_record", "test_501"),
    )).await.unwrap();

    repo.write_tuple(Tuple::new(
        Subject::user("test_charlie"),
        Relation::new("editor"),
        Object::new("patient_record", "test_502"),
    )).await.unwrap();

    // Filter by viewer relation
    let viewer_tuples = repo.read_tuples(
        Some(Subject::user("test_charlie")),
        Some(Relation::new("viewer")),
        None,
    ).await.unwrap();

    assert_eq!(viewer_tuples.len(), 1, "Should find only viewer tuples");
    assert_eq!(viewer_tuples[0].relation.name, "viewer");

    cleanup_test_data(&pool).await;
    println!("✅ PostgreSQL filter by relation test PASSED");
}

#[tokio::test]
#[ignore]
async fn test_postgres_filter_by_object() {
    let pool = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let repo = repository::PostgresTupleRepository::new(pool.clone());

    // Write tuples for different objects
    repo.write_tuple(Tuple::new(
        Subject::user("test_diana"),
        Relation::new("viewer"),
        Object::new("patient_record", "test_601"),
    )).await.unwrap();

    repo.write_tuple(Tuple::new(
        Subject::user("test_diana"),
        Relation::new("viewer"),
        Object::new("lab_report", "test_602"),
    )).await.unwrap();

    // Filter by patient_record objects
    let patient_tuples = repo.read_tuples(
        Some(Subject::user("test_diana")),
        None,
        Some(Object::new("patient_record", "test_601")),
    ).await.unwrap();

    assert_eq!(patient_tuples.len(), 1, "Should find only patient record tuples");
    assert_eq!(patient_tuples[0].object.object_type, "patient_record");

    cleanup_test_data(&pool).await;
    println!("✅ PostgreSQL filter by object test PASSED");
}

#[tokio::test]
#[ignore]
async fn test_postgres_duplicate_insert_idempotent() {
    let pool = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let repo = repository::PostgresTupleRepository::new(pool.clone());

    let tuple = Tuple::new(
        Subject::user("test_eve"),
        Relation::new("viewer"),
        Object::new("patient_record", "test_701"),
    );

    // Write twice - should be idempotent (ON CONFLICT DO NOTHING)
    repo.write_tuple(tuple.clone()).await.unwrap();
    repo.write_tuple(tuple.clone()).await.unwrap();

    // Should still exist once
    let tuples = repo.read_tuples(
        Some(Subject::user("test_eve")),
        None,
        None,
    ).await.unwrap();

    assert_eq!(tuples.len(), 1, "Duplicate insert should be idempotent");

    cleanup_test_data(&pool).await;
    println!("✅ PostgreSQL idempotent insert test PASSED");
}

#[tokio::test]
#[ignore]
async fn test_postgres_batch_write_with_deletes() {
    let pool = setup_test_pool().await;
    cleanup_test_data(&pool).await;

    let repo = repository::PostgresTupleRepository::new(pool.clone());

    // Setup initial tuples
    let tuple1 = Tuple::new(Subject::user("test_frank"), Relation::new("viewer"), Object::new("patient_record", "test_801"));
    let tuple2 = Tuple::new(Subject::user("test_frank"), Relation::new("viewer"), Object::new("patient_record", "test_802"));
    let tuple3 = Tuple::new(Subject::user("test_frank"), Relation::new("viewer"), Object::new("patient_record", "test_803"));

    repo.write_tuple(tuple1.clone()).await.unwrap();
    repo.write_tuple(tuple2.clone()).await.unwrap();

    // Batch: delete tuple2, add tuple3
    let request = WriteRequest {
        writes: vec![tuple3.clone()],
        deletes: vec![tuple2.clone()],
    };

    repo.batch_write(request).await.unwrap();

    // Verify: tuple1 exists, tuple2 deleted, tuple3 added
    assert!(repo.tuple_exists(&tuple1).await.unwrap(), "Tuple1 should still exist");
    assert!(!repo.tuple_exists(&tuple2).await.unwrap(), "Tuple2 should be deleted");
    assert!(repo.tuple_exists(&tuple3).await.unwrap(), "Tuple3 should be added");

    cleanup_test_data(&pool).await;
    println!("✅ PostgreSQL batch write with deletes test PASSED");
}
