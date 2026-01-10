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

#[cfg(test)]
mod tests;

use zab_bid_domain::{Area, BidYear};

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
/// - The bid year scope (`bid_year`)
/// - The area scope (`area`)
/// - An optional event ID assigned by persistence (`event_id`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    /// Optional event ID assigned when persisted.
    /// None when created in-memory, Some(id) after persistence.
    pub event_id: Option<i64>,
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
    /// The bid year this event is scoped to.
    pub bid_year: BidYear,
    /// The area this event is scoped to.
    pub area: Area,
}

impl AuditEvent {
    /// Creates a new `AuditEvent` without a persisted event ID.
    ///
    /// Once created, an audit event is immutable.
    /// The `event_id` will be None until the event is persisted.
    ///
    /// # Arguments
    ///
    /// * `actor` - The actor who initiated the change
    /// * `cause` - The reason for the change
    /// * `action` - The action that was performed
    /// * `before` - The state before the transition
    /// * `after` - The state after the transition
    /// * `bid_year` - The bid year this event is scoped to
    /// * `area` - The area this event is scoped to
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        actor: Actor,
        cause: Cause,
        action: Action,
        before: StateSnapshot,
        after: StateSnapshot,
        bid_year: BidYear,
        area: Area,
    ) -> Self {
        Self {
            event_id: None,
            actor,
            cause,
            action,
            before,
            after,
            bid_year,
            area,
        }
    }

    /// Creates a new `AuditEvent` with a persisted event ID.
    ///
    /// This is typically used when reconstructing events from storage.
    ///
    /// # Arguments
    ///
    /// * `event_id` - The unique event ID from persistence
    /// * `actor` - The actor who initiated the change
    /// * `cause` - The reason for the change
    /// * `action` - The action that was performed
    /// * `before` - The state before the transition
    /// * `after` - The state after the transition
    /// * `bid_year` - The bid year this event is scoped to
    /// * `area` - The area this event is scoped to
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn with_id(
        event_id: i64,
        actor: Actor,
        cause: Cause,
        action: Action,
        before: StateSnapshot,
        after: StateSnapshot,
        bid_year: BidYear,
        area: Area,
    ) -> Self {
        Self {
            event_id: Some(event_id),
            actor,
            cause,
            action,
            before,
            after,
            bid_year,
            area,
        }
    }
}
