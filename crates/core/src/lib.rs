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

use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::{BidRequest, DomainError, validate_bid};

/// A command represents user or system intent as data only.
///
/// Commands are the only way to request state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Submit a bid for time off.
    SubmitBid {
        /// The employee ID making the bid.
        employee_id: String,
        /// The period being bid for.
        period: String,
        /// The requested days.
        requested_days: Vec<String>,
    },
}

/// Represents a submitted bid in the system state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bid {
    /// The employee ID who submitted the bid.
    pub employee_id: String,
    /// The period being bid for.
    pub period: String,
    /// The requested days.
    pub requested_days: Vec<String>,
}

/// The complete system state.
///
/// This is minimal for Phase 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// All submitted bids.
    pub bids: Vec<Bid>,
}

impl State {
    /// Creates a new empty state.
    #[must_use]
    pub const fn new() -> Self {
        Self { bids: Vec::new() }
    }

    /// Converts the state to a snapshot for audit purposes.
    #[must_use]
    pub fn to_snapshot(&self) -> StateSnapshot {
        StateSnapshot::new(format!("bids_count={}", self.bids.len()))
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// The result of a successful state transition.
///
/// Transitions are atomic: they either succeed completely or fail without side effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionResult {
    /// The new state after the transition.
    pub new_state: State,
    /// The audit event recording this transition.
    pub audit_event: AuditEvent,
}

/// Errors that can occur during state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// A domain rule was violated.
    DomainViolation(DomainError),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DomainViolation(err) => write!(f, "Domain violation: {err}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<DomainError> for CoreError {
    fn from(err: DomainError) -> Self {
        Self::DomainViolation(err)
    }
}

/// Applies a command to the current state, producing a new state and audit event.
///
/// This is the heart of Phase 0. It ensures:
/// - Validation happens via domain rules
/// - New state is produced immutably
/// - Audit events are constructed for every successful transition
/// - Failures do not mutate state
///
/// # Arguments
///
/// * `state` - The current state (immutable)
/// * `command` - The command to apply
/// * `actor` - The actor performing this action
/// * `cause` - The cause or reason for this action
///
/// # Returns
///
/// * `Ok(TransitionResult)` containing the new state and audit event
/// * `Err(CoreError)` if the command is invalid
///
/// # Errors
///
/// Returns an error if:
/// - The command violates domain rules
pub fn apply(
    state: &State,
    command: Command,
    actor: Actor,
    cause: Cause,
) -> Result<TransitionResult, CoreError> {
    match command {
        Command::SubmitBid {
            employee_id,
            period,
            requested_days,
        } => {
            // Create a domain object for validation
            let bid_request: BidRequest = BidRequest {
                employee_id: employee_id.clone(),
                period: period.clone(),
                requested_days: requested_days.clone(),
            };

            // Validate using domain rules
            validate_bid(&bid_request)?;

            // Capture state before transition
            let before: StateSnapshot = state.to_snapshot();

            // Create new state with the bid added
            let mut new_bids: Vec<Bid> = state.bids.clone();
            new_bids.push(Bid {
                employee_id,
                period,
                requested_days,
            });
            let new_state: State = State { bids: new_bids };

            // Capture state after transition
            let after: StateSnapshot = new_state.to_snapshot();

            // Create audit event
            let action: Action = Action::new(String::from("SubmitBid"), None);
            let audit_event: AuditEvent = AuditEvent::new(actor, cause, action, before, after);

            Ok(TransitionResult {
                new_state,
                audit_event,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_actor() -> Actor {
        Actor::new(String::from("user-123"), String::from("user"))
    }

    fn create_test_cause() -> Cause {
        Cause::new(String::from("req-456"), String::from("User request"))
    }

    #[test]
    fn test_valid_command_returns_new_state() {
        let state: State = State::new();
        let command: Command = Command::SubmitBid {
            employee_id: String::from("EMP001"),
            period: String::from("2026-Q1"),
            requested_days: vec![String::from("2026-01-15")],
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_ok());
        let transition: TransitionResult = result.unwrap();
        assert_eq!(transition.new_state.bids.len(), 1);
        assert_eq!(transition.new_state.bids[0].employee_id, "EMP001");
    }

    #[test]
    fn test_valid_command_emits_audit_event() {
        let state: State = State::new();
        let command: Command = Command::SubmitBid {
            employee_id: String::from("EMP001"),
            period: String::from("2026-Q1"),
            requested_days: vec![String::from("2026-01-15")],
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_ok());
        let transition: TransitionResult = result.unwrap();
        assert_eq!(transition.audit_event.action.name, "SubmitBid");
        assert_eq!(transition.audit_event.actor.id, "user-123");
        assert_eq!(transition.audit_event.cause.id, "req-456");
    }

    #[test]
    fn test_audit_event_contains_before_and_after_state() {
        let state: State = State::new();
        let command: Command = Command::SubmitBid {
            employee_id: String::from("EMP001"),
            period: String::from("2026-Q1"),
            requested_days: vec![String::from("2026-01-15")],
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_ok());
        let transition: TransitionResult = result.unwrap();
        assert_eq!(transition.audit_event.before.data, "bids_count=0");
        assert_eq!(transition.audit_event.after.data, "bids_count=1");
    }

    #[test]
    fn test_invalid_command_returns_error() {
        let state: State = State::new();
        let command: Command = Command::SubmitBid {
            employee_id: String::new(), // Invalid: empty employee ID
            period: String::from("2026-Q1"),
            requested_days: vec![String::from("2026-01-15")],
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CoreError::DomainViolation(DomainError::InvalidEmployeeId(_))
        ));
    }

    #[test]
    fn test_invalid_command_does_not_mutate_state() {
        let state: State = State::new();
        let original_bid_count: usize = state.bids.len();

        let command: Command = Command::SubmitBid {
            employee_id: String::new(), // Invalid
            period: String::from("2026-Q1"),
            requested_days: vec![String::from("2026-01-15")],
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_err());
        // Original state is unchanged
        assert_eq!(state.bids.len(), original_bid_count);
    }

    #[test]
    fn test_invalid_command_does_not_emit_audit_event() {
        let state: State = State::new();
        let command: Command = Command::SubmitBid {
            employee_id: String::new(), // Invalid
            period: String::from("2026-Q1"),
            requested_days: vec![String::from("2026-01-15")],
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        // When the result is an error, no audit event is created
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_valid_transitions() {
        let mut state: State = State::new();
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        // First transition
        let command1: Command = Command::SubmitBid {
            employee_id: String::from("EMP001"),
            period: String::from("2026-Q1"),
            requested_days: vec![String::from("2026-01-15")],
        };
        let result1: Result<TransitionResult, CoreError> =
            apply(&state, command1, actor.clone(), cause.clone());
        assert!(result1.is_ok());
        state = result1.unwrap().new_state;
        assert_eq!(state.bids.len(), 1);

        // Second transition
        let command2: Command = Command::SubmitBid {
            employee_id: String::from("EMP002"),
            period: String::from("2026-Q1"),
            requested_days: vec![String::from("2026-01-16")],
        };
        let result2: Result<TransitionResult, CoreError> = apply(&state, command2, actor, cause);
        assert!(result2.is_ok());
        state = result2.unwrap().new_state;
        assert_eq!(state.bids.len(), 2);
    }
}
