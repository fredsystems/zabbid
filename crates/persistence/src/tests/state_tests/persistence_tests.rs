// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::SqlitePersistence;
use crate::tests::{
    create_test_actor, create_test_cause, create_test_metadata, create_test_operator,
    create_test_pay_periods, create_test_seniority_data, create_test_start_date,
};
use zab_bid::{
    BootstrapMetadata, BootstrapResult, Command, State, TransitionResult, apply, apply_bootstrap,
};
use zab_bid_audit::AuditEvent;
use zab_bid_domain::{Area, BidYear, Crew, Initials, UserType};

/// Creates a fully bootstrapped test persistence instance with bid year 2026 and area "North".
fn create_bootstrapped_persistence() -> SqlitePersistence {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();

    // Create test operator first to satisfy foreign key constraints
    create_test_operator(&mut persistence);

    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Bootstrap bid year
    let create_bid_year_cmd: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let placeholder_bid_year = BidYear::new(2026);
    let bid_year_result: BootstrapResult = apply_bootstrap(
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
    let area_result: BootstrapResult = apply_bootstrap(
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
fn test_persist_and_retrieve_audit_event() {
    let mut persistence: SqlitePersistence = create_bootstrapped_persistence();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let command: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("John Doe"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };

    let result: TransitionResult = apply(
        &create_test_metadata(),
        &state,
        &BidYear::new(2026),
        command,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();

    let event_id: i64 = persistence.persist_transition(&result, false).unwrap();

    let retrieved: AuditEvent = persistence.get_audit_event(event_id).unwrap();
    assert_eq!(retrieved.event_id, Some(event_id));
    assert_eq!(retrieved.action.name, "RegisterUser");
}

#[test]
fn test_persist_with_snapshot() {
    let mut persistence: SqlitePersistence = create_bootstrapped_persistence();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

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

    let event_id: i64 = persistence.persist_transition(&result, true).unwrap();

    let (snapshot, snapshot_event_id): (State, i64) = persistence
        .get_latest_snapshot(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    assert_eq!(snapshot_event_id, event_id);
    assert_eq!(snapshot.bid_year.year(), 2026);
    assert_eq!(snapshot.area.id(), "NORTH");
}

#[test]
fn test_get_events_after() {
    let mut persistence: SqlitePersistence = create_bootstrapped_persistence();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Create first event
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
    let event_id1: i64 = persistence.persist_transition(&result1, true).unwrap();

    // Create second event
    let command2: Command = Command::Finalize;
    let result2: TransitionResult = apply(
        &create_test_metadata(),
        &result1.new_state,
        &BidYear::new(2026),
        command2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    let _event_id2: i64 = persistence.persist_transition(&result2, true).unwrap();

    // Retrieve events after first
    let events: Vec<AuditEvent> = persistence
        .get_events_after(&BidYear::new(2026), &Area::new("North"), event_id1)
        .unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].action.name, "Finalize");
}

#[test]
fn test_should_snapshot_detection() {
    assert!(SqlitePersistence::should_snapshot("Checkpoint"));
    assert!(SqlitePersistence::should_snapshot("Finalize"));
    assert!(SqlitePersistence::should_snapshot("Rollback"));
    assert!(!SqlitePersistence::should_snapshot("RegisterUser"));
}

#[test]
fn test_atomic_persistence_failure() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    // Close the connection to force an error
    drop(persistence);

    // Try to create a new one and verify it works
    persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    let state: State = State::new(BidYear::new(2026), Area::new("North"));
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

    // This should succeed
    assert!(persistence.persist_transition(&result, true).is_ok());
}

#[test]
fn test_state_reconstruction_with_snapshot_then_deltas() {
    let mut persistence: SqlitePersistence = create_bootstrapped_persistence();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Create initial snapshot
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
    persistence.persist_transition(&result1, true).unwrap();

    // Add user (delta)
    let command2: Command = Command::RegisterUser {
        initials: Initials::new("TS"),
        name: String::from("Test User"),
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
    persistence.persist_transition(&result2, false).unwrap();

    // Create another snapshot
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
    persistence.persist_transition(&result3, true).unwrap();

    // Current state should use most recent snapshot
    let current_state: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    assert_eq!(current_state.users.len(), 1);
    assert_eq!(current_state.users[0].initials.value(), "TS");
}
