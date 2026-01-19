// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Tests for password management functionality.

use crate::ApiError;
use crate::auth::{AuthenticatedActor, Role};
use crate::handlers::{change_password, create_operator, reset_password};
use crate::request_response::{ChangePasswordRequest, CreateOperatorRequest, ResetPasswordRequest};
use crate::tests::helpers::create_test_cause;
use zab_bid_persistence::SqlitePersistence;

#[test]
fn test_operator_can_change_own_password() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create an operator
    let operator_id = persistence
        .create_operator("testop", "Test Operator", "OldPassword123!", "Bidder")
        .unwrap();

    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    let actor = AuthenticatedActor {
        id: operator_id.to_string(),
        role: Role::Bidder,
    };

    let request = ChangePasswordRequest {
        current_password: String::from("OldPassword123!"),
        new_password: String::from("NewPassword456!"),
        new_password_confirmation: String::from("NewPassword456!"),
    };

    let cause = create_test_cause();

    let result = change_password(&mut persistence, &request, &actor, &operator, cause);

    assert!(result.is_ok());
}

#[test]
fn test_change_password_with_wrong_current_password_fails() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "OldPassword123!", "Bidder")
        .unwrap();

    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    let actor = AuthenticatedActor {
        id: operator_id.to_string(),
        role: Role::Bidder,
    };

    let request = ChangePasswordRequest {
        current_password: String::from("WrongPassword123!"),
        new_password: String::from("NewPassword456!"),
        new_password_confirmation: String::from("NewPassword456!"),
    };

    let cause = create_test_cause();

    let result = change_password(&mut persistence, &request, &actor, &operator, cause);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::AuthenticationFailed { .. } => (),
        _ => panic!("Expected AuthenticationFailed error"),
    }
}

#[test]
fn test_change_password_enforces_policy() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "OldPassword123!", "Bidder")
        .unwrap();

    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    let actor = AuthenticatedActor {
        id: operator_id.to_string(),
        role: Role::Bidder,
    };

    // Try to use a password that's too short
    let request = ChangePasswordRequest {
        current_password: String::from("OldPassword123!"),
        new_password: String::from("Short1!"),
        new_password_confirmation: String::from("Short1!"),
    };

    let cause = create_test_cause();

    let result = change_password(&mut persistence, &request, &actor, &operator, cause);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::PasswordPolicyViolation { .. } => (),
        _ => panic!("Expected PasswordPolicyViolation error"),
    }
}

#[test]
fn test_change_password_invalidates_sessions() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "OldPassword123!", "Bidder")
        .unwrap();

    // Create a session for the operator
    let expires_at = "2026-12-31T23:59:59Z";
    persistence
        .create_session("session_token_123", operator_id, expires_at)
        .unwrap();

    // Verify session exists
    let session = persistence
        .get_session_by_token("session_token_123")
        .unwrap();
    assert!(session.is_some());

    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    let actor = AuthenticatedActor {
        id: operator_id.to_string(),
        role: Role::Bidder,
    };

    let request = ChangePasswordRequest {
        current_password: String::from("OldPassword123!"),
        new_password: String::from("NewPassword456!"),
        new_password_confirmation: String::from("NewPassword456!"),
    };

    let cause = create_test_cause();

    let result = change_password(&mut persistence, &request, &actor, &operator, cause);
    assert!(result.is_ok());

    // Verify session was invalidated
    let session = persistence
        .get_session_by_token("session_token_123")
        .unwrap();
    assert!(session.is_none());
}

#[test]
fn test_admin_can_reset_another_operators_password() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create admin
    let admin_id = persistence
        .create_operator("admin", "Admin User", "AdminPassword123!", "Admin")
        .unwrap();

    let admin = persistence.get_operator_by_id(admin_id).unwrap().unwrap();
    let admin_actor = AuthenticatedActor {
        id: admin_id.to_string(),
        role: Role::Admin,
    };

    // Create target operator
    let target_id = persistence
        .create_operator("target", "Target User", "TargetPassword123!", "Bidder")
        .unwrap();

    let request = ResetPasswordRequest {
        operator_id: target_id,
        new_password: String::from("ResetPassword456!"),
        new_password_confirmation: String::from("ResetPassword456!"),
    };

    let cause = create_test_cause();

    let result = reset_password(&mut persistence, &request, &admin_actor, &admin, cause);

    assert!(result.is_ok());
}

#[test]
fn test_bidder_cannot_reset_password() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create bidder
    let bidder_id = persistence
        .create_operator("bidder", "Bidder User", "BidderPassword123!", "Bidder")
        .unwrap();

    let bidder = persistence.get_operator_by_id(bidder_id).unwrap().unwrap();
    let bidder_actor = AuthenticatedActor {
        id: bidder_id.to_string(),
        role: Role::Bidder,
    };

    // Create target operator
    let target_id = persistence
        .create_operator("target", "Target User", "TargetPassword123!", "Bidder")
        .unwrap();

    let request = ResetPasswordRequest {
        operator_id: target_id,
        new_password: String::from("ResetPassword456!"),
        new_password_confirmation: String::from("ResetPassword456!"),
    };

    let cause = create_test_cause();

    let result = reset_password(&mut persistence, &request, &bidder_actor, &bidder, cause);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized { .. } => (),
        _ => panic!("Expected Unauthorized error"),
    }
}

#[test]
fn test_reset_password_enforces_policy() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let admin_id = persistence
        .create_operator("admin", "Admin User", "AdminPassword123!", "Admin")
        .unwrap();

    let admin = persistence.get_operator_by_id(admin_id).unwrap().unwrap();
    let admin_actor = AuthenticatedActor {
        id: admin_id.to_string(),
        role: Role::Admin,
    };

    let target_id = persistence
        .create_operator("target", "Target User", "TargetPassword123!", "Bidder")
        .unwrap();

    // Try password that's too short
    let request = ResetPasswordRequest {
        operator_id: target_id,
        new_password: String::from("Short1!"),
        new_password_confirmation: String::from("Short1!"),
    };

    let cause = create_test_cause();

    let result = reset_password(&mut persistence, &request, &admin_actor, &admin, cause);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::PasswordPolicyViolation { .. } => (),
        _ => panic!("Expected PasswordPolicyViolation error"),
    }
}

#[test]
fn test_reset_password_invalidates_target_sessions() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let admin_id = persistence
        .create_operator("admin", "Admin User", "AdminPassword123!", "Admin")
        .unwrap();

    let admin = persistence.get_operator_by_id(admin_id).unwrap().unwrap();
    let admin_actor = AuthenticatedActor {
        id: admin_id.to_string(),
        role: Role::Admin,
    };

    let target_id = persistence
        .create_operator("target", "Target User", "TargetPassword123!", "Bidder")
        .unwrap();

    // Create session for target
    let expires_at = "2026-12-31T23:59:59Z";
    persistence
        .create_session("target_session_123", target_id, expires_at)
        .unwrap();

    // Verify session exists
    let session = persistence
        .get_session_by_token("target_session_123")
        .unwrap();
    assert!(session.is_some());

    let request = ResetPasswordRequest {
        operator_id: target_id,
        new_password: String::from("ResetPassword456!"),
        new_password_confirmation: String::from("ResetPassword456!"),
    };

    let cause = create_test_cause();

    let result = reset_password(&mut persistence, &request, &admin_actor, &admin, cause);
    assert!(result.is_ok());

    // Verify target's session was invalidated
    let session = persistence
        .get_session_by_token("target_session_123")
        .unwrap();
    assert!(session.is_none());
}

#[test]
fn test_create_operator_requires_password() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let admin_id = persistence
        .create_operator("admin", "Admin User", "AdminPassword123!", "Admin")
        .unwrap();

    let admin = persistence.get_operator_by_id(admin_id).unwrap().unwrap();
    let admin_actor = AuthenticatedActor {
        id: admin_id.to_string(),
        role: Role::Admin,
    };

    let request = CreateOperatorRequest {
        login_name: String::from("newop"),
        display_name: String::from("New Operator"),
        role: String::from("Bidder"),
        password: String::from("ValidPassword123!"),
        password_confirmation: String::from("ValidPassword123!"),
    };

    let cause = create_test_cause();

    let result = create_operator(&mut persistence, request, &admin_actor, &admin, cause);

    assert!(result.is_ok());
}

#[test]
fn test_create_operator_enforces_password_policy() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let admin_id = persistence
        .create_operator("admin", "Admin User", "AdminPassword123!", "Admin")
        .unwrap();

    let admin = persistence.get_operator_by_id(admin_id).unwrap().unwrap();
    let admin_actor = AuthenticatedActor {
        id: admin_id.to_string(),
        role: Role::Admin,
    };

    // Try with password that's too short
    let request = CreateOperatorRequest {
        login_name: String::from("newop"),
        display_name: String::from("New Operator"),
        role: String::from("Bidder"),
        password: String::from("Short1!"),
        password_confirmation: String::from("Short1!"),
    };

    let cause = create_test_cause();

    let result = create_operator(&mut persistence, request, &admin_actor, &admin, cause);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::PasswordPolicyViolation { .. } => (),
        _ => panic!("Expected PasswordPolicyViolation error"),
    }
}

#[test]
fn test_password_change_emits_audit_event() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let operator_id = persistence
        .create_operator("testop", "Test Operator", "OldPassword123!", "Bidder")
        .unwrap();

    let operator = persistence
        .get_operator_by_id(operator_id)
        .unwrap()
        .unwrap();
    let actor = AuthenticatedActor {
        id: operator_id.to_string(),
        role: Role::Bidder,
    };

    let request = ChangePasswordRequest {
        current_password: String::from("OldPassword123!"),
        new_password: String::from("NewPassword456!"),
        new_password_confirmation: String::from("NewPassword456!"),
    };

    let cause = create_test_cause();

    // Get event count before
    let events_before = persistence.get_global_audit_events().unwrap();
    let count_before = events_before.len();

    let result = change_password(&mut persistence, &request, &actor, &operator, cause);
    assert!(result.is_ok());

    // Verify audit event was created
    let events_after = persistence.get_global_audit_events().unwrap();
    let count_after = events_after.len();

    assert_eq!(count_after, count_before + 1);

    // Verify the audit event details
    let last_event = &events_after[events_after.len() - 1];
    assert_eq!(last_event.action.name, "ChangePassword");
    assert_eq!(last_event.actor.operator_id, Some(operator_id));
}

#[test]
fn test_password_reset_emits_audit_event() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let admin_id = persistence
        .create_operator("admin", "Admin User", "AdminPassword123!", "Admin")
        .unwrap();

    let admin = persistence.get_operator_by_id(admin_id).unwrap().unwrap();
    let admin_actor = AuthenticatedActor {
        id: admin_id.to_string(),
        role: Role::Admin,
    };

    let target_id = persistence
        .create_operator("target", "Target User", "TargetPassword123!", "Bidder")
        .unwrap();

    let request = ResetPasswordRequest {
        operator_id: target_id,
        new_password: String::from("ResetPassword456!"),
        new_password_confirmation: String::from("ResetPassword456!"),
    };

    let cause = create_test_cause();

    // Get event count before
    let events_before = persistence.get_global_audit_events().unwrap();
    let count_before = events_before.len();

    let result = reset_password(&mut persistence, &request, &admin_actor, &admin, cause);
    assert!(result.is_ok());

    // Verify audit event was created
    let events_after = persistence.get_global_audit_events().unwrap();
    let count_after = events_after.len();

    assert_eq!(count_after, count_before + 1);

    // Verify the audit event details
    let last_event = &events_after[events_after.len() - 1];
    assert_eq!(last_event.action.name, "ResetPassword");
    assert_eq!(last_event.actor.operator_id, Some(admin_id));
}

/// `PHASE_27H.7`: Verify `reset_password` fails when target operator does not exist
#[test]
fn test_reset_password_with_nonexistent_operator() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    let admin_id = persistence
        .create_operator("admin", "Admin User", "AdminPassword123!", "Admin")
        .unwrap();

    let admin = persistence.get_operator_by_id(admin_id).unwrap().unwrap();
    let admin_actor = AuthenticatedActor {
        id: admin_id.to_string(),
        role: Role::Admin,
    };

    // Use an operator ID that doesn't exist
    let nonexistent_id = 99999;

    let request = ResetPasswordRequest {
        operator_id: nonexistent_id,
        new_password: String::from("ResetPassword456!"),
        new_password_confirmation: String::from("ResetPassword456!"),
    };

    let cause = create_test_cause();

    let result = reset_password(&mut persistence, &request, &admin_actor, &admin, cause);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::ResourceNotFound {
            resource_type,
            message,
        } => {
            assert_eq!(resource_type, "Operator");
            assert!(message.contains("99999"));
            assert!(message.contains("not found"));
        }
        _ => panic!("Expected ResourceNotFound error"),
    }
}
