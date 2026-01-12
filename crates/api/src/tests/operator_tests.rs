// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Tests for operator management API handlers.

use crate::auth::{AuthenticatedActor, Role};
use crate::error::ApiError;
use crate::{
    DeleteOperatorRequest, DeleteOperatorResponse, DisableOperatorRequest, DisableOperatorResponse,
    EnableOperatorRequest, EnableOperatorResponse, ListOperatorsResponse, create_operator,
    delete_operator, disable_operator, enable_operator, list_operators,
};
use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::{Area, BidYear};
use zab_bid_persistence::SqlitePersistence;

fn create_test_admin() -> AuthenticatedActor {
    AuthenticatedActor {
        id: String::from("admin"),
        role: Role::Admin,
    }
}

fn create_test_bidder() -> AuthenticatedActor {
    AuthenticatedActor {
        id: String::from("bidder"),
        role: Role::Bidder,
    }
}

fn create_test_cause() -> Cause {
    Cause::new(String::from("test"), String::from("Test operation"))
}

#[test]
fn test_list_operators_requires_admin() {
    let persistence = SqlitePersistence::new_in_memory().unwrap();
    let bidder = create_test_bidder();

    let result = list_operators(&persistence, &bidder);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized { action, .. } => {
            assert_eq!(action, "list_operators");
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}

#[test]
fn test_list_operators_succeeds_for_admin() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let admin = create_test_admin();

    // Create some operators
    persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    persistence
        .create_operator("bidder1", "Bidder One", "password", "Bidder")
        .unwrap();

    let result = list_operators(&persistence, &admin);

    assert!(result.is_ok());
    let response: ListOperatorsResponse = result.unwrap();
    assert_eq!(response.operators.len(), 2);
}

#[test]
fn test_disable_operator_requires_admin() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let bidder = create_test_bidder();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();

    let request = DisableOperatorRequest { operator_id };
    let cause = create_test_cause();

    let result = disable_operator(&mut persistence, request, &bidder, &operator, cause);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized { action, .. } => {
            assert_eq!(action, "disable_operator");
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}

#[test]
fn test_disable_operator_succeeds_for_admin() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let admin = create_test_admin();

    // Create admin operator for audit attribution
    let admin_op_id = persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    let admin_operator = persistence
        .get_operator_by_id(admin_op_id)
        .unwrap()
        .unwrap();

    // Create target operator
    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    let request = DisableOperatorRequest { operator_id };
    let cause = create_test_cause();

    let result = disable_operator(&mut persistence, request, &admin, &admin_operator, cause);

    assert!(result.is_ok());
    let response: DisableOperatorResponse = result.unwrap();
    assert!(response.message.contains("disabled"));

    // Verify operator is disabled in database
    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    assert!(operator.is_disabled);
}

#[test]
fn test_disable_nonexistent_operator_fails() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let admin = create_test_admin();

    let admin_op_id = persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    let admin_operator = persistence
        .get_operator_by_id(admin_op_id)
        .unwrap()
        .unwrap();

    let request = DisableOperatorRequest { operator_id: 999 };
    let cause = create_test_cause();

    let result = disable_operator(&mut persistence, request, &admin, &admin_operator, cause);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::ResourceNotFound { .. } => {}
        other => panic!("Expected ResourceNotFound error, got: {other:?}"),
    }
}

#[test]
fn test_enable_operator_requires_admin() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let bidder = create_test_bidder();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();

    let request = EnableOperatorRequest { operator_id };
    let cause = create_test_cause();

    let result = enable_operator(&mut persistence, request, &bidder, &operator, cause);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized { action, .. } => {
            assert_eq!(action, "enable_operator");
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}

#[test]
fn test_enable_operator_succeeds_for_admin() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let admin = create_test_admin();

    let admin_op_id = persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    let admin_operator = persistence
        .get_operator_by_id(admin_op_id)
        .unwrap()
        .unwrap();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    // Disable first
    persistence.disable_operator(operator_id).unwrap();

    let request = EnableOperatorRequest { operator_id };
    let cause = create_test_cause();

    let result = enable_operator(&mut persistence, request, &admin, &admin_operator, cause);

    assert!(result.is_ok());
    let response: EnableOperatorResponse = result.unwrap();
    assert!(response.message.contains("enabled"));

    // Verify operator is enabled in database
    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    assert!(!operator.is_disabled);
}

#[test]
fn test_delete_operator_requires_admin() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let bidder = create_test_bidder();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();

    let request = DeleteOperatorRequest { operator_id };
    let cause = create_test_cause();

    let result = delete_operator(&mut persistence, request, &bidder, &operator, cause);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized { action, .. } => {
            assert_eq!(action, "delete_operator");
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}

#[test]
fn test_delete_operator_succeeds_when_not_referenced() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let admin = create_test_admin();

    let admin_op_id = persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    let admin_operator = persistence
        .get_operator_by_id(admin_op_id)
        .unwrap()
        .unwrap();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    let request = DeleteOperatorRequest { operator_id };
    let cause = create_test_cause();

    let result = delete_operator(&mut persistence, request, &admin, &admin_operator, cause);

    assert!(result.is_ok());
    let response: DeleteOperatorResponse = result.unwrap();
    assert!(response.message.contains("deleted"));

    // Verify operator is deleted in database
    assert!(
        persistence
            .get_operator_by_id(operator_id)
            .unwrap()
            .is_none()
    );
}

#[test]
fn test_delete_operator_fails_when_referenced() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let admin = create_test_admin();

    let admin_op_id = persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    let admin_operator = persistence
        .get_operator_by_id(admin_op_id)
        .unwrap()
        .unwrap();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    // Create an audit event referencing this operator
    let target_operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();

    let actor = Actor::with_operator(
        operator_id.to_string(),
        String::from("operator"),
        operator_id,
        target_operator.login_name.clone(),
        target_operator.display_name,
    );

    let cause = create_test_cause();
    let action = Action::new(String::from("TestAction"), None);
    let before = StateSnapshot::new(String::from("before"));
    let after = StateSnapshot::new(String::from("after"));
    let bid_year = BidYear::new(0);
    let area = Area::new("_operator_management");

    let audit_event = AuditEvent::new(actor, cause, action, before, after, bid_year, area);
    persistence.persist_audit_event(&audit_event).unwrap();

    // Attempt to delete
    let request = DeleteOperatorRequest { operator_id };
    let cause = create_test_cause();

    let result = delete_operator(&mut persistence, request, &admin, &admin_operator, cause);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::DomainRuleViolation { rule, .. } => {
            assert_eq!(rule, "operator_not_referenced");
        }
        other => panic!("Expected DomainRuleViolation error, got: {other:?}"),
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
fn test_create_operator_emits_audit_event() {
    use crate::CreateOperatorRequest;

    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let admin = create_test_admin();

    let admin_op_id = persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    let admin_operator = persistence
        .get_operator_by_id(admin_op_id)
        .unwrap()
        .unwrap();

    let request = CreateOperatorRequest {
        login_name: String::from("newop"),
        display_name: String::from("New Operator"),
        role: String::from("Bidder"),
    };

    let cause = create_test_cause();

    // Get event count before
    let events_before = persistence
        .get_audit_timeline(&BidYear::new(0), &Area::new("_operator_management"))
        .unwrap();
    let count_before = events_before.len();

    let result = create_operator(&mut persistence, request, &admin, &admin_operator, cause);

    assert!(result.is_ok());

    // Verify audit event was created
    let events_after = persistence
        .get_audit_timeline(&BidYear::new(0), &Area::new("_operator_management"))
        .unwrap();
    let count_after = events_after.len();

    assert_eq!(count_after, count_before + 1);

    // Verify the audit event details
    let last_event = &events_after[events_after.len() - 1];
    assert_eq!(last_event.action.name, "CreateOperator");
}

#[test]
fn test_disable_operator_emits_audit_event() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let admin = create_test_admin();

    let admin_op_id = persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    let admin_operator = persistence
        .get_operator_by_id(admin_op_id)
        .unwrap()
        .unwrap();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    let events_before = persistence
        .get_audit_timeline(&BidYear::new(0), &Area::new("_operator_management"))
        .unwrap();
    let count_before = events_before.len();

    let request = DisableOperatorRequest { operator_id };
    let cause = create_test_cause();

    disable_operator(&mut persistence, request, &admin, &admin_operator, cause).unwrap();

    let events_after = persistence
        .get_audit_timeline(&BidYear::new(0), &Area::new("_operator_management"))
        .unwrap();
    let count_after = events_after.len();

    assert_eq!(count_after, count_before + 1);

    let last_event = &events_after[events_after.len() - 1];
    assert_eq!(last_event.action.name, "DisableOperator");
}

#[test]
fn test_enable_operator_emits_audit_event() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let admin = create_test_admin();

    let admin_op_id = persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    let admin_operator = persistence
        .get_operator_by_id(admin_op_id)
        .unwrap()
        .unwrap();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    persistence.disable_operator(operator_id).unwrap();

    let events_before = persistence
        .get_audit_timeline(&BidYear::new(0), &Area::new("_operator_management"))
        .unwrap();
    let count_before = events_before.len();

    let request = EnableOperatorRequest { operator_id };
    let cause = create_test_cause();

    enable_operator(&mut persistence, request, &admin, &admin_operator, cause).unwrap();

    let events_after = persistence
        .get_audit_timeline(&BidYear::new(0), &Area::new("_operator_management"))
        .unwrap();
    let count_after = events_after.len();

    assert_eq!(count_after, count_before + 1);

    let last_event = &events_after[events_after.len() - 1];
    assert_eq!(last_event.action.name, "EnableOperator");
}

#[test]
fn test_delete_operator_emits_audit_event() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let admin = create_test_admin();

    let admin_op_id = persistence
        .create_operator("admin1", "Admin One", "password", "Admin")
        .unwrap();
    let admin_operator = persistence
        .get_operator_by_id(admin_op_id)
        .unwrap()
        .unwrap();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    let events_before = persistence
        .get_audit_timeline(&BidYear::new(0), &Area::new("_operator_management"))
        .unwrap();
    let count_before = events_before.len();

    let request = DeleteOperatorRequest { operator_id };
    let cause = create_test_cause();

    delete_operator(&mut persistence, request, &admin, &admin_operator, cause).unwrap();

    let events_after = persistence
        .get_audit_timeline(&BidYear::new(0), &Area::new("_operator_management"))
        .unwrap();
    let count_after = events_after.len();

    assert_eq!(count_after, count_before + 1);

    let last_event = &events_after[events_after.len() - 1];
    assert_eq!(last_event.action.name, "DeleteOperator");
}

#[test]
fn test_unauthorized_action_does_not_emit_audit_event() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    let bidder = create_test_bidder();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "password", "Bidder")
        .unwrap();

    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();

    let events_before = persistence
        .get_audit_timeline(&BidYear::new(0), &Area::new("_operator_management"))
        .unwrap();
    let count_before = events_before.len();

    let request = DisableOperatorRequest { operator_id };
    let cause = create_test_cause();

    let result = disable_operator(&mut persistence, request, &bidder, &operator, cause);

    assert!(result.is_err());

    // Verify no audit event was created
    let events_after = persistence
        .get_audit_timeline(&BidYear::new(0), &Area::new("_operator_management"))
        .unwrap();
    let count_after = events_after.len();

    assert_eq!(count_after, count_before);
}
