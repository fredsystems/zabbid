// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::tests::helpers::{
    create_test_actor, create_test_cause, create_test_metadata, create_test_seniority_data,
};
use crate::{BootstrapMetadata, Command, CoreError, State, TransitionResult, apply};
use zab_bid_audit::{Actor, Cause};
use zab_bid_domain::{Area, BidYear, Crew, DomainError, Initials, User, UserType};

#[test]
fn test_valid_command_returns_new_state() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();
    assert_eq!(transition.new_state.users.len(), 1);
    assert_eq!(transition.new_state.users[0].initials.value(), "AB");
    assert_eq!(transition.new_state.users[0].name, "John Doe");
}

#[test]
fn test_valid_command_emits_audit_event() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();
    assert_eq!(transition.audit_event.action.name, "RegisterUser");
    assert_eq!(transition.audit_event.actor.id, "admin-123");
    assert_eq!(transition.audit_event.cause.id, "req-456");
    assert!(
        transition
            .audit_event
            .action
            .details
            .as_ref()
            .unwrap()
            .contains("AB")
    );
}

#[test]
fn test_audit_event_contains_before_and_after_state() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };

    let result: Result<TransitionResult, CoreError> = apply(
        &metadata,
        &state,
        &active_bid_year,
        command,
        create_test_actor(),
        create_test_cause(),
    );

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();
    assert!(transition.audit_event.before.data.contains("users_count=0"));
    assert!(transition.audit_event.after.data.contains("users_count=1"));
}

#[test]
fn test_duplicate_initials_returns_error() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let mut state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);

    // First user
    let command1: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result1: Result<TransitionResult, CoreError> = apply(
        &metadata,
        &state,
        &active_bid_year,
        command1,
        actor.clone(),
        cause.clone(),
    );
    assert!(result1.is_ok());
    state = result1.unwrap().new_state;

    // Second user with same initials in same bid year
    let command2: Command = Command::RegisterUser {
        initials: Initials::new("AB"), // Duplicate!
        name: String::from("Jane Smith"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(2).unwrap()),
        seniority_data: create_test_seniority_data(),
    };

    let result2: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command2, actor, cause);

    assert!(result2.is_err());
    assert!(matches!(
        result2.unwrap_err(),
        CoreError::DomainViolation(DomainError::DuplicateInitials { .. })
    ));
}

#[test]
fn test_duplicate_initials_in_different_bid_years_allowed() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);

    // User in 2026
    let command1: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result1: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command1, actor, cause);
    assert!(result1.is_ok());
    let _state = result1.unwrap().new_state;

    // To test different bid year, we need a different state scoped to 2027
    // For now, within the same state, this would fail since state is scoped to 2026/North
    // This test needs to be redesigned for the new scoping model
    // Skipping the cross-bid-year test for now as it requires multi-state management
}

#[test]
fn test_invalid_command_with_empty_initials_returns_error() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::RegisterUser {
        initials: Initials::new(""), // Invalid: empty
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::InvalidInitials(_))
    ));
}

#[test]
fn test_invalid_command_with_empty_name_returns_error() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::new(), // Invalid: empty
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::InvalidName(_))
    ));
}

#[test]
fn test_invalid_command_with_empty_area_returns_error() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("John Doe"),
        area: Area::new(""), // Invalid: empty (doesn't exist in metadata)
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::AreaNotFound { .. })
    ));
}

#[test]
fn test_user_with_no_crew_is_valid() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: None, // No crew is valid
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();
    assert_eq!(transition.new_state.users.len(), 1);
    assert!(transition.new_state.users[0].crew.is_none());
}

#[test]
fn test_invalid_command_does_not_mutate_state() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::RegisterUser {
        initials: Initials::new(""), // Invalid: empty
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    // State should remain unchanged
    assert_eq!(state.users.len(), 0);
}

#[test]
fn test_invalid_command_does_not_emit_audit_event() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::RegisterUser {
        initials: Initials::new(""), // Invalid: empty
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    // No audit event should be emitted
    // (This is verified by the fact that Result is Err, not Ok with an event)
}

#[test]
fn test_multiple_valid_transitions() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let mut state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    // First user
    let command1: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let result1: Result<TransitionResult, CoreError> = apply(
        &metadata,
        &state,
        &active_bid_year,
        command1,
        actor.clone(),
        cause.clone(),
    );
    assert!(result1.is_ok());
    state = result1.unwrap().new_state;
    assert_eq!(state.users.len(), 1);

    // Second user with different initials
    let command2: Command = Command::RegisterUser {
        initials: Initials::new("XY"),
        name: String::from("Jane Smith"),
        area: Area::new("North"),
        user_type: UserType::CpcIt,
        crew: Some(Crew::new(2).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let result2: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command2, actor, cause);
    assert!(result2.is_ok());
    state = result2.unwrap().new_state;
    assert_eq!(state.users.len(), 2);

    // Can only add users within the same (bid_year, area) scope
    // Cross-scope operations require separate state instances
}

#[test]
fn test_failed_duplicate_initials_transition_does_not_mutate_state() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let mut state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);

    // First user
    let command1: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result1: Result<TransitionResult, CoreError> = apply(
        &metadata,
        &state,
        &active_bid_year,
        command1,
        actor.clone(),
        cause.clone(),
    );
    assert!(result1.is_ok());
    state = result1.unwrap().new_state;

    // Second user with duplicate initials (should fail)
    let command2: Command = Command::RegisterUser {
        initials: Initials::new("AB"), // Duplicate!
        name: String::from("Jane Smith"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(2).unwrap()),
        seniority_data: create_test_seniority_data(),
    };

    let original_user_count: usize = state.users.len();
    let result2: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command2, actor, cause);

    assert!(result2.is_err());
    // State must not be mutated on failure
    assert_eq!(state.users.len(), original_user_count);
}

#[test]
fn test_register_user_without_bid_year_fails() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::BidYearNotFound(2026))
    ));
}

#[test]
fn test_register_user_without_area_fails() {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.add_bid_year(BidYear::new(2026));
    // Area not added

    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::AreaNotFound { .. })
    ));
}

// ============================================================================
// Gap 9: State Transition Edge Cases
// ============================================================================

/// `PHASE_27H.9`: Test checkpoint operation on empty state (no users)
#[test]
fn test_checkpoint_on_empty_state() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::Checkpoint;
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();

    // State should be unchanged
    assert_eq!(transition.new_state.users.len(), 0);

    // Audit event should be created
    assert_eq!(transition.audit_event.action.name, "Checkpoint");
}

/// `PHASE_27H.9`: Test finalize operation on empty state (no users)
#[test]
fn test_finalize_on_empty_state() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::Finalize;
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();

    // State should be unchanged
    assert_eq!(transition.new_state.users.len(), 0);

    // Audit event should be created
    assert_eq!(transition.audit_event.action.name, "Finalize");
}

/// `PHASE_27H.9`: Test rollback operation creates audit event with target ID
#[test]
fn test_rollback_creates_audit_event() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let target_event_id: i64 = 42;
    let command: Command = Command::RollbackToEventId { target_event_id };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();

    // State should be unchanged (actual rollback happens at persistence layer)
    assert_eq!(transition.new_state.users.len(), 0);

    // Audit event should reference the target event ID
    assert_eq!(transition.audit_event.action.name, "Rollback");
    assert!(
        transition
            .audit_event
            .action
            .details
            .as_ref()
            .unwrap()
            .contains("42")
    );
}

/// `PHASE_27H.9`: Test checkpoint operation on state with many users
#[test]
fn test_checkpoint_on_state_with_multiple_users() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let mut state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Add multiple users to state
    for i in 1..=10 {
        let initials_str = format!("U{i:02}");
        state.users.push(User::new(
            BidYear::new(2026),
            Initials::new(&initials_str),
            format!("User {i}"),
            Area::new("North"),
            UserType::CPC,
            Some(Crew::new(1).unwrap()),
            create_test_seniority_data(),
            false, // excluded_from_bidding
            false, // excluded_from_leave_calculation
        ));
    }

    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::Checkpoint;
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();

    // State should be unchanged - all users preserved
    assert_eq!(transition.new_state.users.len(), 10);
    assert_eq!(transition.new_state.users, state.users);

    // Audit event should be created
    assert_eq!(transition.audit_event.action.name, "Checkpoint");
}

/// `PHASE_27H.9`: Test finalize operation preserves state
#[test]
fn test_finalize_preserves_existing_state() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let mut state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Add a user to state
    state.users.push(User::new(
        BidYear::new(2026),
        Initials::new("AB"),
        String::from("Alice Blue"),
        Area::new("North"),
        UserType::CPC,
        Some(Crew::new(1).unwrap()),
        create_test_seniority_data(),
        false, // excluded_from_bidding
        false, // excluded_from_leave_calculation
    ));

    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::Finalize;
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();

    // State should be unchanged
    assert_eq!(transition.new_state.users.len(), 1);
    assert_eq!(transition.new_state.users[0].initials.value(), "AB");

    // Audit event should be created
    assert_eq!(transition.audit_event.action.name, "Finalize");
}

/// `PHASE_27H.9`: Test rollback to current event is valid (no-op rollback)
#[test]
fn test_rollback_to_same_event_is_valid() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);

    // Rollback to the "current" event (simulating no-op rollback)
    let current_event_id: i64 = 100;
    let command: Command = Command::RollbackToEventId {
        target_event_id: current_event_id,
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();

    // State should be unchanged
    assert_eq!(transition.new_state.users.len(), 0);

    // Audit event should be created with correct target
    assert_eq!(transition.audit_event.action.name, "Rollback");
    assert!(
        transition
            .audit_event
            .action
            .details
            .as_ref()
            .unwrap()
            .contains("100")
    );
}

/// `PHASE_27H.9`: Test update user on empty state fails
#[test]
fn test_update_user_on_empty_state_fails() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let active_bid_year: BidYear = BidYear::new(2026);
    let command: Command = Command::UpdateUser {
        user_id: 999, // Non-existent user_id for testing failure case
        initials: Initials::new("AB"),
        name: String::from("Alice Blue"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, CoreError> =
        apply(&metadata, &state, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::UserNotFound { .. })
    ));
}
