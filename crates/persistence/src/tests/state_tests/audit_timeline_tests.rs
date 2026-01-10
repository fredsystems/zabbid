// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::SqlitePersistence;
use crate::tests::{create_test_actor, create_test_cause, create_test_metadata};
use zab_bid::{Command, State, TransitionResult, apply};
use zab_bid_audit::AuditEvent;
use zab_bid_domain::{Area, BidYear};

/// Creates a fully bootstrapped test persistence instance with bid year 2026 and area "North".
fn create_bootstrapped_persistence() -> SqlitePersistence {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    let mut metadata = zab_bid::BootstrapMetadata::new();

    // Bootstrap bid year
    let create_bid_year_cmd: Command = Command::CreateBidYear { year: 2026 };
    let bid_year_result = zab_bid::apply_bootstrap(
        &metadata,
        create_bid_year_cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&bid_year_result).unwrap();
    metadata.bid_years.push(BidYear::new(2026));

    // Bootstrap area
    let create_area_cmd: Command = Command::CreateArea {
        bid_year: BidYear::new(2026),
        area_id: String::from("North"),
    };
    let area_result = zab_bid::apply_bootstrap(
        &metadata,
        create_area_cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_bootstrap(&area_result).unwrap();

    persistence
}

#[test]
fn test_get_audit_timeline_returns_events_in_order() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Create multiple events
    let command1: Command = Command::Checkpoint;
    let result1: TransitionResult = apply(
        &create_test_metadata(),
        &state,
        command1,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result1, true).unwrap();

    let command2: Command = Command::Finalize;
    let result2: TransitionResult = apply(
        &create_test_metadata(),
        &result1.new_state,
        command2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result2, true).unwrap();

    let command3: Command = Command::RollbackToEventId { target_event_id: 1 };
    let result3: TransitionResult = apply(
        &create_test_metadata(),
        &result2.new_state,
        command3,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result3, true).unwrap();

    // Retrieve timeline
    let timeline: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    assert_eq!(timeline.len(), 3);
    assert_eq!(timeline[0].action.name, "Checkpoint");
    assert_eq!(timeline[1].action.name, "Finalize");
    assert_eq!(timeline[2].action.name, "Rollback");

    // Verify event IDs are in ascending order
    assert!(timeline[0].event_id.unwrap() < timeline[1].event_id.unwrap());
    assert!(timeline[1].event_id.unwrap() < timeline[2].event_id.unwrap());
}

#[test]
fn test_get_audit_timeline_empty_for_nonexistent_scope() {
    let persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();

    // Retrieve timeline for non-existent scope
    let timeline: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("South"))
        .unwrap();

    assert_eq!(timeline.len(), 0);
}

#[test]
fn test_get_audit_timeline_includes_rollback_events() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Create checkpoint
    let command1: Command = Command::Checkpoint;
    let result1: TransitionResult = apply(
        &create_test_metadata(),
        &state,
        command1,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    let event_id1: i64 = persistence.persist_transition(&result1, true).unwrap();

    // Create rollback
    let command2: Command = Command::RollbackToEventId {
        target_event_id: event_id1,
    };
    let result2: TransitionResult = apply(
        &create_test_metadata(),
        &result1.new_state,
        command2,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result2, true).unwrap();

    // Retrieve timeline
    let timeline: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[0].action.name, "Checkpoint");
    assert_eq!(timeline[1].action.name, "Rollback");
}

#[test]
fn test_get_audit_timeline_does_not_mutate() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Create events
    let command: Command = Command::Checkpoint;
    let result: TransitionResult = apply(
        &create_test_metadata(),
        &state,
        command,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result, true).unwrap();

    // Retrieve timeline
    let timeline1: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    // Retrieve again
    let timeline2: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    // Should be identical
    assert_eq!(timeline1.len(), timeline2.len());
    assert_eq!(timeline1.len(), 1);
}

#[test]
fn test_read_operations_are_side_effect_free() {
    let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));

    // Create initial snapshot
    let command: Command = Command::Checkpoint;
    let result: TransitionResult = apply(
        &create_test_metadata(),
        &state,
        command,
        create_test_actor(),
        create_test_cause(),
    )
    .unwrap();
    persistence.persist_transition(&result, true).unwrap();

    // Capture initial event count
    let initial_timeline: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("North"))
        .unwrap();
    let initial_count: usize = initial_timeline.len();

    // Perform multiple read operations
    let _current1: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    let _current2: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    let _timeline1: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    let _timeline2: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    let timestamp: String = String::from("9999-12-31 23:59:59");
    let _historical1: State = persistence
        .get_historical_state(&BidYear::new(2026), &Area::new("North"), &timestamp)
        .unwrap();

    let _historical2: State = persistence
        .get_historical_state(&BidYear::new(2026), &Area::new("North"), &timestamp)
        .unwrap();

    // Verify no new events were created
    let final_timeline: Vec<AuditEvent> = persistence
        .get_audit_timeline(&BidYear::new(2026), &Area::new("North"))
        .unwrap();

    assert_eq!(final_timeline.len(), initial_count);
}
