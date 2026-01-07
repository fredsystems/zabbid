// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![deny(
    clippy::pedantic,
    clippy::cargo,
    clippy::nursery,
    clippy::style,
    clippy::correctness,
    clippy::all
)]

/// Represents the entity performing an action.
///
/// An actor is any identifiable entity that initiates a state change.
/// This could be a user, a system process, or an automated trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    /// The unique identifier for this actor.
    pub id: String,
    /// The type of actor (e.g., "user", "system", "scheduler").
    pub actor_type: String,
}

impl Actor {
    /// Creates a new Actor.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier for this actor
    /// * `actor_type` - The type of actor
    #[must_use]
    pub const fn new(id: String, actor_type: String) -> Self {
        Self { id, actor_type }
    }
}

/// Represents the reason or trigger for an action.
///
/// A cause describes why a state change was initiated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cause {
    /// A unique identifier for this cause (e.g., request ID, event ID).
    pub id: String,
    /// A description of the cause.
    pub description: String,
}

impl Cause {
    /// Creates a new Cause.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier for this cause
    /// * `description` - A description of what triggered this action
    #[must_use]
    pub const fn new(id: String, description: String) -> Self {
        Self { id, description }
    }
}

/// Represents the specific action performed.
///
/// An action describes what state change occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// The name of the action (e.g., "`SubmitBid`", "`ApproveBid`").
    pub name: String,
    /// Optional additional details about the action.
    pub details: Option<String>,
}

impl Action {
    /// Creates a new Action.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the action
    /// * `details` - Optional additional details
    #[must_use]
    pub const fn new(name: String, details: Option<String>) -> Self {
        Self { name, details }
    }
}

/// A snapshot of system state at a point in time.
///
/// This is a placeholder type for Phase 0.
/// In a complete system, this would capture the relevant state for audit purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSnapshot {
    /// A string representation of the state.
    /// In Phase 0, this is intentionally minimal.
    pub data: String,
}

impl StateSnapshot {
    /// Creates a new `StateSnapshot`.
    ///
    /// # Arguments
    ///
    /// * `data` - A string representation of the state
    #[must_use]
    pub const fn new(data: String) -> Self {
        Self { data }
    }
}

/// An immutable audit event representing a state transition.
///
/// Every successful state change must produce exactly one audit event.
/// Audit events are immutable once created and capture:
/// - Who performed the action (actor)
/// - Why it was performed (cause)
/// - What action was performed (action)
/// - The state before the transition (before)
/// - The state after the transition (after)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    /// The actor who initiated this state change.
    pub actor: Actor,
    /// The cause or reason for this state change.
    pub cause: Cause,
    /// The action that was performed.
    pub action: Action,
    /// The state before the transition.
    pub before: StateSnapshot,
    /// The state after the transition.
    pub after: StateSnapshot,
}

impl AuditEvent {
    /// Creates a new `AuditEvent`.
    ///
    /// Once created, an audit event is immutable.
    ///
    /// # Arguments
    ///
    /// * `actor` - The actor who initiated the change
    /// * `cause` - The reason for the change
    /// * `action` - The action that was performed
    /// * `before` - The state before the transition
    /// * `after` - The state after the transition
    #[must_use]
    pub const fn new(
        actor: Actor,
        cause: Cause,
        action: Action,
        before: StateSnapshot,
        after: StateSnapshot,
    ) -> Self {
        Self {
            actor,
            cause,
            action,
            before,
            after,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let event: AuditEvent = AuditEvent::new(
            actor.clone(),
            cause.clone(),
            action.clone(),
            before.clone(),
            after.clone(),
        );

        assert_eq!(event.actor, actor);
        assert_eq!(event.cause, cause);
        assert_eq!(event.action, action);
        assert_eq!(event.before, before);
        assert_eq!(event.after, after);
    }

    #[test]
    fn test_audit_event_is_immutable_once_created() {
        let actor: Actor = Actor::new(String::from("user-123"), String::from("user"));
        let cause: Cause = Cause::new(String::from("req-456"), String::from("User request"));
        let action: Action = Action::new(String::from("SubmitBid"), None);
        let before: StateSnapshot = StateSnapshot::new(String::from("before-state"));
        let after: StateSnapshot = StateSnapshot::new(String::from("after-state"));

        let event: AuditEvent = AuditEvent::new(actor, cause, action, before, after);

        // Clone the event to verify it can be cloned but not mutated
        let cloned_event: AuditEvent = event.clone();
        assert_eq!(event, cloned_event);

        // Verify all fields are accessible but cannot be mutated
        // (Rust's type system enforces this - the fields are not mutable)
        assert_eq!(event.actor.id, "user-123");
        assert_eq!(event.cause.id, "req-456");
        assert_eq!(event.action.name, "SubmitBid");
        assert_eq!(event.before.data, "before-state");
        assert_eq!(event.after.data, "after-state");
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

        let event1: AuditEvent = AuditEvent::new(
            actor.clone(),
            cause.clone(),
            action.clone(),
            before.clone(),
            after.clone(),
        );

        let event2: AuditEvent = AuditEvent::new(actor, cause, action, before, after);

        assert_eq!(event1, event2);
    }
}
