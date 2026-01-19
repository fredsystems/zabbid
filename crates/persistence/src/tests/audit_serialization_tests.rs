// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Tests for audit event serialization and persistence.
//!
//! These tests validate that audit events and state snapshots are correctly
//! serialized, persisted, and deserialized. Focus is on integration behavior
//! rather than testing `serde_json` itself.

use crate::SqlitePersistence;
use crate::tests::{
    create_test_actor, create_test_bid_year_and_area, create_test_operator,
    create_test_seniority_data,
};
use zab_bid::{BootstrapMetadata, Command, State, apply};
use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::{Area, BidYear, Crew, Initials, UserType};

#[test]
fn test_persist_audit_event_with_minimal_snapshot() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    // Create a minimal audit event with small snapshots
    let actor = Actor::with_operator(
        String::from("1"),
        String::from("operator"),
        1,
        String::from("testop"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(String::from("test"), String::from("Test operation"));
    let action = Action::new(String::from("TestAction"), None);
    let before = StateSnapshot::new(String::from("{}"));
    let after = StateSnapshot::new(String::from("{}"));

    let event = AuditEvent::new_global(actor, cause, action, before, after);

    // Persist the event
    let event_id = persistence.persist_audit_event(&event).unwrap();

    assert!(event_id > 0, "Should return valid event ID");
}

#[test]
fn test_persist_audit_event_with_large_snapshot() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    // Create a large snapshot (simulating many users)
    let large_json = format!(
        r#"{{"users": [{}]}}"#,
        (0..1000)
            .map(|i| format!(
                r#"{{"id": {i}, "name": "User {i}", "data": "Lorem ipsum dolor sit amet"}}"#
            ))
            .collect::<Vec<_>>()
            .join(",")
    );

    let actor = Actor::with_operator(
        String::from("1"),
        String::from("operator"),
        1,
        String::from("testop"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(
        String::from("large-test"),
        String::from("Large snapshot test"),
    );
    let action = Action::new(String::from("TestLargeSnapshot"), None);
    let before = StateSnapshot::new(String::from("{}"));
    let after = StateSnapshot::new(large_json);

    let event = AuditEvent::new_global(actor, cause, action, before, after);

    // Should successfully persist large snapshots
    let event_id = persistence.persist_audit_event(&event).unwrap();

    assert!(event_id > 0, "Should handle large snapshots");
}

#[test]
fn test_persist_state_snapshot_integration() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    // Create a state with a user
    let state = State::new(BidYear::new(2026), Area::new("NORTH"));

    let mut metadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("NORTH")));

    let cmd = Command::RegisterUser {
        initials: Initials::new("AB"),
        name: String::from("Alice Bob"),
        area: Area::new("NORTH"),
        user_type: UserType::CPC,
        crew: Some(Crew::new(1).unwrap()),
        seniority_data: create_test_seniority_data(),
    };

    let result = apply(
        &metadata,
        &state,
        &BidYear::new(2026),
        cmd,
        create_test_actor(),
        Cause::new(String::from("test"), String::from("Test")),
    )
    .unwrap();

    // Persist the transition (includes snapshot)
    persistence.persist_transition(&result).unwrap();

    // Verify we can read the state back
    let retrieved_state = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("NORTH"))
        .unwrap();

    assert_eq!(retrieved_state.users.len(), 1, "Should have one user");
    assert_eq!(retrieved_state.users[0].initials.value(), "AB");
    assert_eq!(retrieved_state.users[0].name, "Alice Bob");
}

#[test]
fn test_audit_event_with_special_characters_in_snapshots() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    // Create snapshots with special characters that need JSON escaping
    let special_json = r#"{"data": "Special chars: \"quotes\", 'apostrophes', \n newlines, \t tabs, unicode: 你好"}"#;

    let actor = Actor::with_operator(
        String::from("1"),
        String::from("operator"),
        1,
        String::from("testop"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(
        String::from("special"),
        String::from("Special characters test"),
    );
    let action = Action::new(String::from("TestSpecialChars"), None);
    let before = StateSnapshot::new(String::from("{}"));
    let after = StateSnapshot::new(String::from(special_json));

    let event = AuditEvent::new_global(actor, cause, action, before, after);

    // Should handle special characters correctly
    let event_id = persistence.persist_audit_event(&event).unwrap();

    assert!(
        event_id > 0,
        "Should handle special characters in snapshots"
    );
}

#[test]
fn test_multiple_audit_events_sequential() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    // Create and persist multiple events
    let mut event_ids = Vec::new();

    for i in 0..10 {
        let actor = Actor::with_operator(
            String::from("1"),
            String::from("operator"),
            1,
            String::from("testop"),
            String::from("Test Operator"),
        );
        let cause = Cause::new(format!("test-{i}"), format!("Test operation {i}"));
        let action = Action::new(format!("Action{i}"), None);
        let before = StateSnapshot::new(format!(r#"{{"step": {i}}}"#));
        let after = StateSnapshot::new(format!(r#"{{"step": {}}}"#, i + 1));

        let event = AuditEvent::new_global(actor, cause, action, before, after);

        let event_id = persistence.persist_audit_event(&event).unwrap();
        event_ids.push(event_id);
    }

    // Verify all event IDs are unique and sequential
    assert_eq!(event_ids.len(), 10, "Should have 10 events");
    for i in 1..event_ids.len() {
        assert!(
            event_ids[i] > event_ids[i - 1],
            "Event IDs should be sequential"
        );
    }
}

#[test]
fn test_audit_event_with_action_details() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    // Create event with detailed action metadata
    let action_details = serde_json::json!({
        "previous_value": "old",
        "new_value": "new",
        "field": "status",
        "reason": "Administrative update"
    });

    let actor = Actor::with_operator(
        String::from("1"),
        String::from("operator"),
        1,
        String::from("testop"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(String::from("update"), String::from("Update operation"));
    let action = Action::new(
        String::from("UpdateStatus"),
        Some(action_details.to_string()),
    );
    let before = StateSnapshot::new(String::from(r#"{"status": "old"}"#));
    let after = StateSnapshot::new(String::from(r#"{"status": "new"}"#));

    let event = AuditEvent::new_global(actor, cause, action, before, after);

    // Should persist action details correctly
    let event_id = persistence.persist_audit_event(&event).unwrap();

    assert!(event_id > 0, "Should handle action details");
}

#[test]
fn test_scoped_audit_event_with_bid_year_and_area() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);
    create_test_bid_year_and_area(&mut persistence, 2026, "NORTH");

    // Create a scoped audit event
    let actor = Actor::with_operator(
        String::from("1"),
        String::from("operator"),
        1,
        String::from("testop"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(String::from("scoped"), String::from("Scoped operation"));
    let action = Action::new(String::from("ScopedAction"), None);
    let before = StateSnapshot::new(String::from("{}"));
    let after = StateSnapshot::new(String::from(r#"{"updated": true}"#));

    let event = AuditEvent::new(
        actor,
        cause,
        action,
        before,
        after,
        BidYear::new(2026),
        Area::new("NORTH"),
    );

    // Should persist scoped events with ID lookups
    let event_id = persistence.persist_audit_event(&event).unwrap();

    assert!(event_id > 0, "Should handle scoped events");
}

#[test]
fn test_empty_snapshots() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    // Create event with empty string snapshots
    let actor = Actor::with_operator(
        String::from("1"),
        String::from("operator"),
        1,
        String::from("testop"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(String::from("empty"), String::from("Empty snapshot test"));
    let action = Action::new(String::from("EmptySnapshot"), None);
    let before = StateSnapshot::new(String::new());
    let after = StateSnapshot::new(String::new());

    let event = AuditEvent::new_global(actor, cause, action, before, after);

    // Should handle empty snapshots
    let event_id = persistence.persist_audit_event(&event).unwrap();

    assert!(event_id > 0, "Should handle empty snapshots");
}
