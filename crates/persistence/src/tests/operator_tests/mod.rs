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

#[test]
fn test_list_operators_empty() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // List operators when none exist
    let operators = persistence.list_operators().unwrap();

    assert_eq!(
        operators.len(),
        0,
        "Should return empty list when no operators exist"
    );
}

#[test]
fn test_list_operators_returns_all() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create multiple operators
    persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    persistence
        .create_operator("bidder1", "Bidder One", "password", "Bidder")
        .unwrap();
    persistence
        .create_operator("admin2", "Admin Two", "password", "Admin")
        .unwrap();

    // List all operators
    let operators = persistence.list_operators().unwrap();

    assert_eq!(operators.len(), 3, "Should return all created operators");

    // Verify ordering (should be sorted by login_name)
    assert_eq!(operators[0].login_name, "ADMIN1");
    assert_eq!(operators[1].login_name, "ADMIN2");
    assert_eq!(operators[2].login_name, "BIDDER1");
}

#[test]
fn test_count_operators_zero() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Count when no operators exist
    let count = persistence.count_operators().unwrap();

    assert_eq!(count, 0, "Should return 0 when no operators exist");
}

#[test]
fn test_count_operators_multiple() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create operators
    persistence
        .create_operator("op1", "Operator One", "password", "Admin")
        .unwrap();
    persistence
        .create_operator("op2", "Operator Two", "password", "Bidder")
        .unwrap();
    persistence
        .create_operator("op3", "Operator Three", "password", "Admin")
        .unwrap();

    // Count operators
    let count = persistence.count_operators().unwrap();

    assert_eq!(count, 3, "Should return correct operator count");
}

#[test]
fn test_count_active_admin_operators_zero() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Count when no operators exist
    let count = persistence.count_active_admin_operators().unwrap();

    assert_eq!(count, 0, "Should return 0 when no admin operators exist");
}

#[test]
fn test_count_active_admin_operators_excludes_disabled() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create admin operators
    let admin1 = persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    let admin2 = persistence
        .create_operator("admin2", "Admin Two", "password", "Admin")
        .unwrap();

    // Create bidder (should not be counted)
    persistence
        .create_operator("bidder1", "Bidder One", "password", "Bidder")
        .unwrap();

    // Initially should count both admins
    let count = persistence.count_active_admin_operators().unwrap();
    assert_eq!(count, 2, "Should count both active admins");

    // Disable one admin
    persistence.disable_operator(admin1).unwrap();

    // Should now count only one
    let count = persistence.count_active_admin_operators().unwrap();
    assert_eq!(count, 1, "Should count only enabled admins");

    // Disable second admin
    persistence.disable_operator(admin2).unwrap();

    // Should count zero
    let count = persistence.count_active_admin_operators().unwrap();
    assert_eq!(count, 0, "Should count zero when all admins are disabled");
}

#[test]
fn test_count_active_admin_operators_excludes_bidders() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create admin and bidder
    persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    persistence
        .create_operator("bidder1", "Bidder One", "password", "Bidder")
        .unwrap();
    persistence
        .create_operator("bidder2", "Bidder Two", "password", "Bidder")
        .unwrap();

    // Should count only admin
    let count = persistence.count_active_admin_operators().unwrap();
    assert_eq!(count, 1, "Should count only Admin role, not Bidder");
}

#[test]
fn test_get_operator_by_login_not_found() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create an operator
    persistence
        .create_operator("existingop", "Existing Operator", "password", "Admin")
        .unwrap();

    // Try to get a nonexistent operator
    let result = persistence.get_operator_by_login("nonexistent").unwrap();

    assert!(
        result.is_none(),
        "Should return None for nonexistent operator"
    );
}

#[test]
fn test_get_operator_by_login_case_insensitive() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create operator with lowercase login
    persistence
        .create_operator("testop", "Test Operator", "password", "Admin")
        .unwrap();

    // Try different case variations
    let result1 = persistence.get_operator_by_login("testop").unwrap();
    let result2 = persistence.get_operator_by_login("TESTOP").unwrap();
    let result3 = persistence.get_operator_by_login("TestOp").unwrap();

    assert!(result1.is_some(), "Should find with lowercase");
    assert!(result2.is_some(), "Should find with uppercase");
    assert!(result3.is_some(), "Should find with mixed case");

    assert_eq!(result1.unwrap().login_name, "TESTOP");
    assert_eq!(result2.unwrap().login_name, "TESTOP");
    assert_eq!(result3.unwrap().login_name, "TESTOP");
}

#[test]
fn test_get_operator_by_id_not_found() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create an operator
    persistence
        .create_operator("existingop", "Existing Operator", "password", "Admin")
        .unwrap();

    // Try to get a nonexistent operator ID
    let result = persistence.get_operator_by_id(999).unwrap();

    assert!(
        result.is_none(),
        "Should return None for nonexistent operator ID"
    );
}

#[test]
fn test_get_session_by_token_not_found() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Try to get a nonexistent session
    let result = persistence
        .get_session_by_token("nonexistent-token")
        .unwrap();

    assert!(
        result.is_none(),
        "Should return None for nonexistent session token"
    );
}
