// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Tests for operator lifecycle persistence operations.

use crate::{PersistenceError, SqlitePersistence};
use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};

#[test]
fn test_enable_operator_succeeds() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create an operator
    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Admin")
        .unwrap();

    // Disable the operator
    persistence.disable_operator(operator_id).unwrap();

    // Verify operator is disabled
    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    assert!(operator.is_disabled);
    assert!(operator.disabled_at.is_some());

    // Re-enable the operator
    persistence.enable_operator(operator_id).unwrap();

    // Verify operator is enabled
    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    assert!(!operator.is_disabled);
    assert!(operator.disabled_at.is_none());
}

#[test]
fn test_delete_operator_succeeds_when_not_referenced() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create an operator
    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    // Verify operator exists
    assert!(
        persistence
            .get_operator_by_id(operator_id)
            .unwrap()
            .is_some()
    );

    // Delete the operator
    persistence.delete_operator(operator_id).unwrap();

    // Verify operator is deleted
    assert!(
        persistence
            .get_operator_by_id(operator_id)
            .unwrap()
            .is_none()
    );
}

#[test]
fn test_delete_operator_fails_when_referenced_by_audit_event() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create an operator
    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Admin")
        .unwrap();

    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();

    // Create an audit event referencing this operator
    let actor = Actor::with_operator(
        operator_id.to_string(),
        String::from("operator"),
        operator_id,
        operator.login_name.clone(),
        operator.display_name,
    );

    let cause = Cause::new(String::from("test"), String::from("Test cause"));
    let action = Action::new(String::from("TestAction"), None);
    let before = StateSnapshot::new(String::from("before"));
    let after = StateSnapshot::new(String::from("after"));

    let audit_event = AuditEvent::new_global(actor, cause, action, before, after);

    persistence.persist_audit_event(&audit_event).unwrap();

    // Attempt to delete the operator
    let result = persistence.delete_operator(operator_id);

    // Should fail with OperatorReferenced error
    assert!(result.is_err());
    match result.unwrap_err() {
        PersistenceError::OperatorReferenced { operator_id: id } => {
            assert_eq!(id, operator_id);
        }
        other => panic!("Expected OperatorReferenced error, got: {other:?}"),
    }

    // Verify operator still exists
    assert!(
        persistence
            .get_operator_by_id(operator_id)
            .unwrap()
            .is_some()
    );
}

#[test]
fn test_delete_nonexistent_operator_fails() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Attempt to delete a nonexistent operator
    let result = persistence.delete_operator(999);

    // Should fail with OperatorNotFound error
    assert!(result.is_err());
    match result.unwrap_err() {
        PersistenceError::OperatorNotFound(msg) => {
            assert!(msg.contains("999"));
        }
        other => panic!("Expected OperatorNotFound error, got: {other:?}"),
    }
}

#[test]
fn test_is_operator_referenced_returns_true_when_referenced() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create an operator
    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Admin")
        .unwrap();

    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();

    // Initially not referenced
    assert!(!persistence.is_operator_referenced(operator_id).unwrap());

    // Create an audit event referencing this operator
    let actor = Actor::with_operator(
        operator_id.to_string(),
        String::from("operator"),
        operator_id,
        operator.login_name.clone(),
        operator.display_name,
    );

    let cause = Cause::new(String::from("test"), String::from("Test cause"));
    let action = Action::new(String::from("TestAction"), None);
    let before = StateSnapshot::new(String::from("before"));
    let after = StateSnapshot::new(String::from("after"));

    let audit_event = AuditEvent::new_global(actor, cause, action, before, after);

    persistence.persist_audit_event(&audit_event).unwrap();

    // Now should be referenced
    assert!(persistence.is_operator_referenced(operator_id).unwrap());
}

#[test]
fn test_is_operator_referenced_returns_false_when_not_referenced() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create an operator
    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    // Should not be referenced
    assert!(!persistence.is_operator_referenced(operator_id).unwrap());
}

#[test]
fn test_operator_lifecycle_complete_flow() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create operator
    let operator_id = persistence
        .create_operator("lifecycle", "Lifecycle Test", "password", "Bidder")
        .unwrap();

    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    assert!(!operator.is_disabled);

    // Disable operator
    persistence.disable_operator(operator_id).unwrap();
    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    assert!(operator.is_disabled);

    // Re-enable operator
    persistence.enable_operator(operator_id).unwrap();
    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    assert!(!operator.is_disabled);

    // Disable again
    persistence.disable_operator(operator_id).unwrap();
    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    assert!(operator.is_disabled);

    // Re-enable again
    persistence.enable_operator(operator_id).unwrap();
    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    assert!(!operator.is_disabled);

    // Delete operator (not referenced)
    persistence.delete_operator(operator_id).unwrap();
    assert!(
        persistence
            .get_operator_by_id(operator_id)
            .unwrap()
            .is_none()
    );
}
