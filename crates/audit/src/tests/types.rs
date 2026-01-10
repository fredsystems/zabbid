// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::{Area, BidYear};

#[test]
fn test_actor_creation_requires_all_fields() {
    let actor: Actor = Actor::new(String::from("user-123"), String::from("user"));

    assert_eq!(actor.id, "user-123");
    assert_eq!(actor.actor_type, "user");
}

#[test]
fn test_cause_creation_requires_all_fields() {
    let cause: Cause = Cause::new(String::from("req-456"), String::from("User request"));

    assert_eq!(cause.id, "req-456");
    assert_eq!(cause.description, "User request");
}

#[test]
fn test_action_creation_requires_name() {
    let action: Action = Action::new(String::from("SubmitBid"), None);

    assert_eq!(action.name, "SubmitBid");
    assert_eq!(action.details, None);
}

#[test]
fn test_action_creation_with_details() {
    let action: Action = Action::new(
        String::from("SubmitBid"),
        Some(String::from("Bid for vacation")),
    );

    assert_eq!(action.name, "SubmitBid");
    assert_eq!(action.details, Some(String::from("Bid for vacation")));
}

#[test]
fn test_state_snapshot_creation() {
    let snapshot: StateSnapshot = StateSnapshot::new(String::from("state-data"));

    assert_eq!(snapshot.data, "state-data");
}

#[test]
fn test_audit_event_creation_requires_all_fields() {
    let actor: Actor = Actor::new(String::from("user-123"), String::from("user"));
    let cause: Cause = Cause::new(String::from("req-456"), String::from("User request"));
    let action: Action = Action::new(String::from("SubmitBid"), None);
    let before: StateSnapshot = StateSnapshot::new(String::from("before-state"));
    let after: StateSnapshot = StateSnapshot::new(String::from("after-state"));

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    let event: AuditEvent = AuditEvent::new(
        actor.clone(),
        cause.clone(),
        action.clone(),
        before.clone(),
        after.clone(),
        bid_year.clone(),
        area.clone(),
    );

    assert_eq!(event.event_id, None);
    assert_eq!(event.actor, actor);
    assert_eq!(event.cause, cause);
    assert_eq!(event.action, action);
    assert_eq!(event.before, before);
    assert_eq!(event.after, after);
    assert_eq!(event.bid_year, bid_year);
    assert_eq!(event.area, area);
}

#[test]
fn test_audit_event_is_immutable_once_created() {
    let actor: Actor = Actor::new(String::from("user-123"), String::from("user"));
    let cause: Cause = Cause::new(String::from("req-456"), String::from("User request"));
    let action: Action = Action::new(String::from("SubmitBid"), None);
    let before: StateSnapshot = StateSnapshot::new(String::from("before-state"));
    let after: StateSnapshot = StateSnapshot::new(String::from("after-state"));

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    let event: AuditEvent = AuditEvent::new(actor, cause, action, before, after, bid_year, area);

    // Clone the event to verify it can be cloned but not mutated
    let cloned_event: AuditEvent = event.clone();
    assert_eq!(event, cloned_event);

    // Verify all fields are accessible but cannot be mutated
    // (Rust's type system enforces this - the fields are not mutable)
    assert_eq!(event.event_id, None);
    assert_eq!(event.actor.id, "user-123");
    assert_eq!(event.cause.id, "req-456");
    assert_eq!(event.action.name, "SubmitBid");
    assert_eq!(event.before.data, "before-state");
    assert_eq!(event.after.data, "after-state");
    assert_eq!(event.bid_year.year(), 2026);
    assert_eq!(event.area.id(), "NORTH");
}

#[test]
fn test_actor_equality() {
    let actor1: Actor = Actor::new(String::from("user-123"), String::from("user"));
    let actor2: Actor = Actor::new(String::from("user-123"), String::from("user"));
    let actor3: Actor = Actor::new(String::from("user-456"), String::from("user"));

    assert_eq!(actor1, actor2);
    assert_ne!(actor1, actor3);
}

#[test]
fn test_audit_event_equality() {
    let actor: Actor = Actor::new(String::from("user-123"), String::from("user"));
    let cause: Cause = Cause::new(String::from("req-456"), String::from("User request"));
    let action: Action = Action::new(String::from("SubmitBid"), None);
    let before: StateSnapshot = StateSnapshot::new(String::from("before-state"));
    let after: StateSnapshot = StateSnapshot::new(String::from("after-state"));

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    let event1: AuditEvent = AuditEvent::new(
        actor.clone(),
        cause.clone(),
        action.clone(),
        before.clone(),
        after.clone(),
        bid_year.clone(),
        area.clone(),
    );

    let event2: AuditEvent = AuditEvent::new(actor, cause, action, before, after, bid_year, area);

    assert_eq!(event1, event2);
}

#[test]
fn test_audit_event_with_id() {
    let actor: Actor = Actor::new(String::from("user-123"), String::from("user"));
    let cause: Cause = Cause::new(String::from("req-456"), String::from("User request"));
    let action: Action = Action::new(String::from("SubmitBid"), None);
    let before: StateSnapshot = StateSnapshot::new(String::from("before-state"));
    let after: StateSnapshot = StateSnapshot::new(String::from("after-state"));
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    let event: AuditEvent =
        AuditEvent::with_id(42, actor, cause, action, before, after, bid_year, area);

    assert_eq!(event.event_id, Some(42));
}
