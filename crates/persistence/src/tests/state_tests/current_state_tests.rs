// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::SqlitePersistence;
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
    let bid_year_result = zab_bid::apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
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
    let area_result = zab_bid::apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        create_area_cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&area_result).unwrap();

    persistence
}

#[test]
fn test_get_current_state_with_no_deltas() {
    let mut persistence: SqlitePersistence = create_bootstrapped_persistence();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Create a snapshot with no users
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

    // Retrieve current state
    let current_state: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    assert_eq!(current_state.bid_year.year(), 2026);
    assert_eq!(current_state.area.id(), "NORTH");
    assert_eq!(current_state.users.len(), 0);
}

#[test]
fn test_get_current_state_after_snapshot_with_user() {
    let mut persistence: SqlitePersistence = create_bootstrapped_persistence();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Create initial empty snapshot
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

    // Register a user (delta event, no snapshot)
    let command2: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("Alice Blue"),
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

    // Create another snapshot to capture the state with the user
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

    // Retrieve current state - should include the user from most recent snapshot
    let current_state: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    assert_eq!(current_state.bid_year.year(), 2026);
    assert_eq!(current_state.area.id(), "NORTH");
    assert_eq!(current_state.users.len(), 1);
    assert_eq!(current_state.users[0].initials.value(), "AB");
}

#[test]
fn test_get_current_state_no_snapshot_returns_error() {
    let mut persistence: SqlitePersistence = create_bootstrapped_persistence();

    // Try to retrieve current state with no users added yet
    // With canonical tables (Phase 7), an empty state is valid (no users yet)
    let result: Result<State, crate::error::PersistenceError> =
        persistence.get_current_state(&BidYear::new(2026), &Area::new("North"));

    // Should succeed with empty user list
    assert!(result.is_ok());
    let state: State = result.unwrap();
    assert_eq!(state.users.len(), 0);
}

#[test]
fn test_get_current_state_is_deterministic() {
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
    persistence.persist_transition(&result1).unwrap();

    // Add a user
    let command2: Command = Command::RegisterUser {
        initials: Initials::new("XY"),
        name: String::from("Xavier Young"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(2).unwrap()),
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
    persistence.persist_transition(&result3).unwrap();

    // Retrieve state multiple times
    let state1: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();
    let state2: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();
    let state3: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    assert_eq!(state1.users.len(), state2.users.len());
    assert_eq!(state2.users.len(), state3.users.len());
    assert_eq!(state1.users[0].initials.value(), "XY");
    assert_eq!(state2.users[0].initials.value(), "XY");
    assert_eq!(state3.users[0].initials.value(), "XY");
}

#[test]
fn test_get_current_state_does_not_mutate() {
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

    // Count events before read
    let timeline_before: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("North"))
        .unwrap();
    let count_before: usize = timeline_before.len();

    // Read current state
    let _current_state: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    // Count events after read
    let timeline_after: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("North"))
        .unwrap();
    let count_after: usize = timeline_after.len();

    assert_eq!(count_before, count_after, "Read operation mutated state");
}

#[test]
fn test_get_current_state_with_multiple_users() {
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
    persistence.persist_transition(&result1).unwrap();

    // Add first user
    let command2: Command = Command::RegisterUser {
        initials: Initials::new("AA"),
        name: String::from("Alice Anderson"),
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

    // Add second user
    let command3: Command = Command::RegisterUser {
        initials: Initials::new("BB"),
        name: String::from("Bob Brown"),
        area: Area::new("North"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(2).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
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

    // Create snapshot with both users
    let command4: Command = Command::Checkpoint;
    let result4: TransitionResult = apply(
        &create_test_metadata(),
        &result3.new_state,
        &BidYear::new(2026),
        command4,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result4).unwrap();

    // Retrieve current state
    let current_state: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    assert_eq!(current_state.users.len(), 2);
    let initials: Vec<String> = current_state
        .users
        .iter()
        .map(|u| u.initials.value().to_string())
        .collect();
    assert!(initials.contains(&String::from("AA")));
    assert!(initials.contains(&String::from("BB")));
}

/// Helper to bootstrap an area and add user to it.
fn bootstrap_area_with_user(
    persistence: &mut SqlitePersistence,
    metadata: &zab_bid::BootstrapMetadata,
    area_name: &str,
    user_initials: &str,
) {
    let area: Area = Area::new(area_name);
    let state: State = State::new(BidYear::new(2026), area.clone());

    // Initial checkpoint
    let cmd1: Command = Command::Checkpoint;
    let res1: TransitionResult = apply(
        metadata,
        &state,
        &BidYear::new(2026),
        cmd1,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&res1).unwrap();

    // Register user
    let cmd2: Command = Command::RegisterUser {
        initials: Initials::new(user_initials),
        name: format!("{area_name} User"),
        area,
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let res2: TransitionResult = apply(
        metadata,
        &res1.new_state,
        &BidYear::new(2026),
        cmd2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&res2).unwrap();

    // Final checkpoint
    let cmd3: Command = Command::Checkpoint;
    let res3: TransitionResult = apply(
        metadata,
        &res2.new_state,
        &BidYear::new(2026),
        cmd3,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&res3).unwrap();
}

/// Helper to create empty area state.
fn create_empty_area_state(
    persistence: &mut SqlitePersistence,
    metadata: &zab_bid::BootstrapMetadata,
    area_name: &str,
) {
    let state: State = State::new(BidYear::new(2026), Area::new(area_name));
    let cmd: Command = Command::Checkpoint;
    let res: TransitionResult = apply(
        metadata,
        &state,
        &BidYear::new(2026),
        cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&res).unwrap();
}

#[test]
fn test_get_current_state_different_areas_isolated() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let mut metadata = zab_bid::BootstrapMetadata::new();

    // Bootstrap bid year
    let create_bid_year_cmd: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let bid_year_result = zab_bid::apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        create_bid_year_cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&bid_year_result).unwrap();
    metadata.bid_years.push(BidYear::new(2026));

    // Bootstrap North area
    let create_north_cmd: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let north_result = zab_bid::apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        create_north_cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&north_result).unwrap();
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("North")));

    // Bootstrap South area
    let create_south_cmd: Command = Command::CreateArea {
        area_id: String::from("South"),
    };
    let south_result = zab_bid::apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        create_south_cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&south_result).unwrap();

    // Create state for North with a user
    bootstrap_area_with_user(&mut persistence, &metadata, "North", "NN");

    // Create state for South (empty)
    create_empty_area_state(&mut persistence, &metadata, "South");

    // Verify North has the user
    let north_current: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();
    assert_eq!(north_current.users.len(), 1);
    assert_eq!(north_current.users[0].initials.value(), "NN");

    // Verify South is empty
    let south_current: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("South"))
        .unwrap();
    assert_eq!(south_current.users.len(), 0);
}
