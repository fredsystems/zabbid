// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::SqlitePersistence;
use crate::error::PersistenceError;
use crate::tests::{
    create_test_actor, create_test_cause, create_test_metadata, create_test_operator,
    create_test_pay_periods, create_test_seniority_data, create_test_start_date,
};
use zab_bid::{Command, State, TransitionResult, apply};
use zab_bid_audit::AuditEvent;
use zab_bid_domain::{Area, BidYear, Crew, Initials, UserType};

/// Creates a fully bootstrapped test persistence instance with bid year 2026 and area "North".
fn create_bootstrapped_persistence() -> SqlitePersistence {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let mut metadata = zab_bid::BootstrapMetadata::new();

    // Bootstrap bid year
    let create_bid_year_cmd: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let placeholder_bid_year = BidYear::new(2026);
    let bid_year_result = zab_bid::apply_bootstrap(
        &metadata,
        &placeholder_bid_year,
        create_bid_year_cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&bid_year_result).unwrap();
    metadata.bid_years.push(BidYear::new(2026));

    // Bootstrap area
    let create_area_cmd: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let active_bid_year = BidYear::new(2026);
    let area_result = zab_bid::apply_bootstrap(
        &metadata,
        &active_bid_year,
        create_area_cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&area_result).unwrap();

    persistence
}

#[test]
fn test_get_historical_state_at_specific_timestamp() {
    let mut persistence: SqlitePersistence = create_bootstrapped_persistence();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Create first snapshot with no users
    let command1: Command = Command::Checkpoint;
    let result1: TransitionResult = apply(
        &create_test_metadata(),
        &state,
        &BidYear::new(2026),
        command1,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result1).unwrap();

    // Register a user (non-snapshot event)
    let command2: Command = Command::RegisterUser {
        initials: Initials::new("NE"),
        name: String::from("New User"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let result2: TransitionResult = apply(
        &create_test_metadata(),
        &result1.new_state,
        &BidYear::new(2026),
        command2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result2).unwrap();

    // Create second snapshot with user
    let command3: Command = Command::Checkpoint;
    let result3: TransitionResult = apply(
        &create_test_metadata(),
        &result2.new_state,
        &BidYear::new(2026),
        command3,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result3).unwrap();

    // Query historical state at very early time - should return error (no snapshot yet)
    let early_timestamp: String = String::from("1970-01-01 00:00:00");
    let result_early: Result<State, PersistenceError> = persistence.get_historical_state(
        &BidYear::new(2026),
        &Area::new("North"),
        &early_timestamp,
    );
    assert!(result_early.is_err());

    // Query historical state at far future time - should use most recent snapshot (with user)
    let future_timestamp: String = String::from("9999-12-31 23:59:59");
    let historical_state: State = persistence
        .get_historical_state(&BidYear::new(2026), &Area::new("North"), &future_timestamp)
        .unwrap();

    assert_eq!(historical_state.users.len(), 1);
    assert_eq!(historical_state.users[0].initials.value(), "NE");
}

#[test]
fn test_get_historical_state_before_any_snapshot_returns_error() {
    let mut persistence: SqlitePersistence = create_bootstrapped_persistence();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Create a snapshot
    let command: Command = Command::Checkpoint;
    let result: TransitionResult = apply(
        &create_test_metadata(),
        &state,
        &BidYear::new(2026),
        command,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result).unwrap();

    // Try to query before the snapshot was created
    let early_timestamp: String = String::from("2020-01-01 00:00:00");
    let result: Result<State, PersistenceError> = persistence.get_historical_state(
        &BidYear::new(2026),
        &Area::new("North"),
        &early_timestamp,
    );

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PersistenceError::SnapshotNotFound { .. }
    ));
}

#[test]
fn test_get_historical_state_is_deterministic() {
    let mut persistence: SqlitePersistence = create_bootstrapped_persistence();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Create snapshot
    let command: Command = Command::Checkpoint;
    let result: TransitionResult = apply(
        &create_test_metadata(),
        &state,
        &BidYear::new(2026),
        command,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result).unwrap();

    // Use a far-future timestamp that will definitely be after the persisted event
    let timestamp: String = String::from("9999-12-31 23:59:59");

    // Query multiple times
    let state1: State = persistence
        .get_historical_state(&BidYear::new(2026), &Area::new("North"), &timestamp)
        .unwrap();

    let state2: State = persistence
        .get_historical_state(&BidYear::new(2026), &Area::new("North"), &timestamp)
        .unwrap();

    let state3: State = persistence
        .get_historical_state(&BidYear::new(2026), &Area::new("North"), &timestamp)
        .unwrap();

    // All should be identical
    assert_eq!(state1.users.len(), state2.users.len());
    assert_eq!(state2.users.len(), state3.users.len());
    assert_eq!(state1.users.len(), 0);
}

#[test]
fn test_get_historical_state_does_not_mutate() {
    let mut persistence: SqlitePersistence = create_bootstrapped_persistence();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Create snapshot
    let command: Command = Command::Checkpoint;
    let result: TransitionResult = apply(
        &create_test_metadata(),
        &state,
        &BidYear::new(2026),
        command,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result).unwrap();

    let timestamp: String = String::from("9999-12-31 23:59:59");

    // Count events before read
    let timeline_before: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    // Perform historical read
    let _historical_state: State = persistence
        .get_historical_state(&BidYear::new(2026), &Area::new("North"), &timestamp)
        .unwrap();

    // Count events after read
    let timeline_after: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    // No new events should be created
    assert_eq!(timeline_before.len(), timeline_after.len());
}
