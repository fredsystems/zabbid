// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::SqlitePersistence;
use crate::tests::{
    create_test_actor, create_test_cause, create_test_metadata, create_test_operator,
    create_test_pay_periods, create_test_seniority_data, create_test_start_date,
    create_test_start_date_for_year,
};
use zab_bid::{
    BootstrapMetadata, BootstrapResult, Command, State, TransitionResult, apply, apply_bootstrap,
};
use zab_bid_audit::AuditEvent;
use zab_bid_domain::{Area, BidYear, CanonicalBidYear, Crew, Initials, User, UserType};

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
    let bid_year_result: BootstrapResult = apply_bootstrap(
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
    let area_result: BootstrapResult = apply_bootstrap(
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
fn test_persist_bootstrap_bid_year() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let metadata: BootstrapMetadata = BootstrapMetadata::new();

    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let result: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();

    let event_id: i64 = persistence.persist_bootstrap(&result).unwrap();

    assert_eq!(event_id, 1);

    // Verify the event was persisted
    let retrieved_event: AuditEvent = persistence.get_audit_event(event_id).unwrap();
    assert_eq!(retrieved_event.action.name, "CreateBidYear");
}

#[test]
fn test_persist_bootstrap_area() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // First create the bid year
    let create_bid_year_cmd: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let bid_year_result: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        create_bid_year_cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&bid_year_result).unwrap();
    metadata.bid_years.push(BidYear::new(2026));

    // Now create the area
    let command: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let result: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();

    let event_id: i64 = persistence.persist_bootstrap(&result).unwrap();

    assert_eq!(event_id, 2);

    // Verify the event was persisted
    let retrieved_event: AuditEvent = persistence.get_audit_event(event_id).unwrap();
    assert_eq!(retrieved_event.action.name, "CreateArea");
}

#[test]
fn test_get_bootstrap_metadata_empty() {
    let persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();

    assert_eq!(metadata.bid_years.len(), 0);
    assert_eq!(metadata.areas.len(), 0);
}

#[test]
fn test_get_bootstrap_metadata_with_bid_year() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let metadata: BootstrapMetadata = BootstrapMetadata::new();

    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let result: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();

    persistence.persist_bootstrap(&result).unwrap();

    let retrieved_metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();

    assert_eq!(retrieved_metadata.bid_years.len(), 1);
    assert_eq!(retrieved_metadata.bid_years[0].year(), 2026);
    assert_eq!(retrieved_metadata.areas.len(), 0);
}

#[test]
fn test_get_bootstrap_metadata_with_multiple_bid_years() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Create first bid year
    let command1: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let result1: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command1,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result1).unwrap();
    metadata = result1.new_metadata;

    // Create second bid year
    let command2: Command = Command::CreateBidYear {
        year: 2027,
        start_date: create_test_start_date_for_year(2027),
        num_pay_periods: create_test_pay_periods(),
    };
    let result2: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result2).unwrap();

    let retrieved_metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();

    assert_eq!(retrieved_metadata.bid_years.len(), 2);
    assert!(retrieved_metadata.has_bid_year(&BidYear::new(2026)));
    assert!(retrieved_metadata.has_bid_year(&BidYear::new(2027)));
}

#[test]
fn test_get_bootstrap_metadata_with_areas() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Create bid year
    let command1: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let result1: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command1,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result1).unwrap();
    metadata = result1.new_metadata;

    // Create first area
    let command2: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let result2: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result2).unwrap();
    metadata = result2.new_metadata;

    // Create second area
    let command3: Command = Command::CreateArea {
        area_id: String::from("South"),
    };
    let result3: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command3,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result3).unwrap();

    let retrieved_metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();

    assert_eq!(retrieved_metadata.bid_years.len(), 1);
    assert_eq!(retrieved_metadata.areas.len(), 2);
    assert!(retrieved_metadata.has_area(&BidYear::new(2026), &Area::new("North")));
    assert!(retrieved_metadata.has_area(&BidYear::new(2026), &Area::new("South")));
}

#[test]
fn test_get_bootstrap_metadata_ignores_non_bootstrap_events() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Create bid year and area
    let command1: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let result1: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command1,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result1).unwrap();
    metadata = result1.new_metadata;

    let command2: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let result2: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result2).unwrap();

    // Add a regular user registration event
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let user_command: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("Test User"),
        area: Area::new("North"),
        user_type: UserType::parse("CPC").unwrap(),
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };
    let user_result: TransitionResult = apply(
        &create_test_metadata(),
        &state,
        &BidYear::new(2026),
        user_command,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&user_result, false).unwrap();

    // Bootstrap metadata should only include bid year and area, not user
    let retrieved_metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();

    assert_eq!(retrieved_metadata.bid_years.len(), 1);
    assert_eq!(retrieved_metadata.areas.len(), 1);
}

#[test]
fn test_list_bid_years_empty() {
    let persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    let bid_years: Vec<CanonicalBidYear> = persistence.list_bid_years().unwrap();

    assert_eq!(bid_years.len(), 0);
}

#[test]
fn test_list_bid_years() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Create first bid year
    let command1: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let result1: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command1,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result1).unwrap();
    metadata = result1.new_metadata;

    // Create second bid year
    let command2: Command = Command::CreateBidYear {
        year: 2027,
        start_date: create_test_start_date_for_year(2027),
        num_pay_periods: create_test_pay_periods(),
    };
    let result2: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result2).unwrap();

    let bid_years: Vec<CanonicalBidYear> = persistence.list_bid_years().unwrap();

    assert_eq!(bid_years.len(), 2);
    assert!(bid_years.iter().any(|by| by.year() == 2026));
    assert!(bid_years.iter().any(|by| by.year() == 2027));
}

#[test]
fn test_list_areas_empty() {
    let persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    let areas: Vec<Area> = persistence.list_areas(&BidYear::new(2026)).unwrap();

    assert_eq!(areas.len(), 0);
}

#[test]
fn test_list_areas_for_bid_year() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Create bid year
    let command1: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let result1: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command1,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result1).unwrap();
    metadata = result1.new_metadata;

    // Create areas
    let command2: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let result2: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result2).unwrap();
    metadata = result2.new_metadata;

    let command3: Command = Command::CreateArea {
        area_id: String::from("South"),
    };
    let result3: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command3,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result3).unwrap();

    let areas: Vec<Area> = persistence.list_areas(&BidYear::new(2026)).unwrap();

    assert_eq!(areas.len(), 2);
    assert!(areas.iter().any(|a| a.id() == "NORTH"));
    assert!(areas.iter().any(|a| a.id() == "SOUTH"));
}

#[test]
#[ignore = "Phase 19: Multiple bid years are no longer supported - all operations target the active bid year"]
fn test_list_areas_isolated_by_bid_year() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Create two bid years
    let command1: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let result1: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command1,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result1).unwrap();
    metadata = result1.new_metadata;

    let command2: Command = Command::CreateBidYear {
        year: 2027,
        start_date: create_test_start_date_for_year(2027),
        num_pay_periods: create_test_pay_periods(),
    };
    let result2: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result2).unwrap();
    metadata = result2.new_metadata;

    // Create area in 2026
    let command3: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let result3: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command3,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result3).unwrap();
    metadata = result3.new_metadata;

    // Create area in 2027
    let command4: Command = Command::CreateArea {
        area_id: String::from("South"),
    };
    let result4: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command4,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result4).unwrap();

    let areas_2026: Vec<Area> = persistence.list_areas(&BidYear::new(2026)).unwrap();
    let areas_2027: Vec<Area> = persistence.list_areas(&BidYear::new(2027)).unwrap();

    assert_eq!(areas_2026.len(), 1);
    assert_eq!(areas_2026[0].id(), "NORTH");

    assert_eq!(areas_2027.len(), 1);
    assert_eq!(areas_2027[0].id(), "SOUTH");
}

#[test]
fn test_bootstrap_persistence_is_deterministic() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Create bid year and area
    let command1: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let result1: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command1,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result1).unwrap();
    metadata = result1.new_metadata;

    let command2: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let result2: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result2).unwrap();

    // Query metadata multiple times
    let metadata1: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let metadata2: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let metadata3: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();

    assert_eq!(metadata1.bid_years.len(), metadata2.bid_years.len());
    assert_eq!(metadata2.bid_years.len(), metadata3.bid_years.len());
    assert_eq!(metadata1.areas.len(), metadata2.areas.len());
    assert_eq!(metadata2.areas.len(), metadata3.areas.len());
}

#[test]
fn test_bootstrap_read_operations_do_not_mutate() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let metadata: BootstrapMetadata = BootstrapMetadata::new();

    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let result: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result).unwrap();

    // Get initial event count
    let initial_count: i64 = persistence
        .conn
        .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
        .unwrap();

    // Perform multiple reads
    let _metadata1: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let _bid_years1: Vec<CanonicalBidYear> = persistence.list_bid_years().unwrap();
    let _areas1: Vec<Area> = persistence.list_areas(&BidYear::new(2026)).unwrap();
    let _metadata2: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let _bid_years2: Vec<CanonicalBidYear> = persistence.list_bid_years().unwrap();
    let _areas2: Vec<Area> = persistence.list_areas(&BidYear::new(2026)).unwrap();

    // Verify event count unchanged
    let final_count: i64 = persistence
        .conn
        .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
        .unwrap();

    assert_eq!(initial_count, final_count);
}

#[test]
fn test_create_area_creates_initial_snapshot() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Create bid year
    let command1: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let result1: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command1,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result1).unwrap();
    metadata = result1.new_metadata;

    // Create area
    let command2: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let result2: BootstrapResult = apply_bootstrap(
        &metadata,
        &BidYear::new(2026),
        command2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&result2).unwrap();

    // Verify we can get_current_state for this area (should not fail)
    let state: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    assert_eq!(state.bid_year.year(), 2026);
    assert_eq!(state.area.id(), "NORTH");
    assert_eq!(state.users.len(), 0);
}

#[test]
fn test_list_users() {
    let mut persistence = create_bootstrapped_persistence();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Register a user
    let command: Command = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("Alice Blue"),
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
    persistence.persist_transition(&result, false).unwrap();

    // List users
    let users: Vec<User> = persistence
        .list_users(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].initials.value(), "AB");
    assert_eq!(users[0].name, "Alice Blue");
}
