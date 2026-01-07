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
use zab_bid_domain::{
    Area, BidYear, Crew, DomainError, Initials, SeniorityData, User, validate_initials_unique,
    validate_user_fields,
};

/// A command represents user or system intent as data only.
///
/// Commands are the only way to request state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Register a new user for a bid year.
    RegisterUser {
        /// The bid year.
        bid_year: BidYear,
        /// The user's initials.
        initials: Initials,
        /// The user's name.
        name: String,
        /// The user's area.
        area: Area,
        /// The user's crew.
        crew: Crew,
        /// The user's seniority data.
        seniority_data: SeniorityData,
    },
}

/// The complete system state.
///
/// For Phase 1, this tracks users scoped by bid year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// All registered users.
    pub users: Vec<User>,
}

impl State {
    /// Creates a new empty state.
    #[must_use]
    pub const fn new() -> Self {
        Self { users: Vec::new() }
    }

    /// Converts the state to a snapshot for audit purposes.
    #[must_use]
    pub fn to_snapshot(&self) -> StateSnapshot {
        StateSnapshot::new(format!("users_count={}", self.users.len()))
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
/// This function ensures:
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
        Command::RegisterUser {
            bid_year,
            initials,
            name,
            area,
            crew,
            seniority_data,
        } => {
            // Create the user object
            let user: User = User::new(
                bid_year.clone(),
                initials.clone(),
                name,
                area,
                crew,
                seniority_data,
            );

            // Validate user field constraints
            validate_user_fields(&user)?;

            // Validate initials are unique within the bid year
            validate_initials_unique(&bid_year, &initials, &state.users)?;

            // Capture state before transition
            let before: StateSnapshot = state.to_snapshot();

            // Create new state with the user added
            let mut new_users: Vec<User> = state.users.clone();
            new_users.push(user);
            let new_state: State = State { users: new_users };

            // Capture state after transition
            let after: StateSnapshot = new_state.to_snapshot();

            // Create audit event
            let action: Action = Action::new(
                String::from("RegisterUser"),
                Some(format!(
                    "Registered user with initials '{}' for bid year {}",
                    initials.value(),
                    bid_year.year()
                )),
            );
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
        Actor::new(String::from("admin-123"), String::from("admin"))
    }

    fn create_test_cause() -> Cause {
        Cause::new(String::from("req-456"), String::from("Admin request"))
    }

    fn create_test_seniority_data() -> SeniorityData {
        SeniorityData::new(
            String::from("2019-01-15"),
            String::from("2019-06-01"),
            String::from("2020-01-15"),
            String::from("2020-01-15"),
            Some(42),
        )
    }

    #[test]
    fn test_valid_command_returns_new_state() {
        let state: State = State::new();
        let command: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("ABC")),
            name: String::from("John Doe"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_ok());
        let transition: TransitionResult = result.unwrap();
        assert_eq!(transition.new_state.users.len(), 1);
        assert_eq!(transition.new_state.users[0].initials.value(), "ABC");
        assert_eq!(transition.new_state.users[0].name, "John Doe");
    }

    #[test]
    fn test_valid_command_emits_audit_event() {
        let state: State = State::new();
        let command: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("ABC")),
            name: String::from("John Doe"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_ok());
        let transition: TransitionResult = result.unwrap();
        assert_eq!(transition.audit_event.action.name, "RegisterUser");
        assert_eq!(transition.audit_event.actor.id, "admin-123");
        assert_eq!(transition.audit_event.cause.id, "req-456");
        assert!(
            transition
                .audit_event
                .action
                .details
                .as_ref()
                .unwrap()
                .contains("ABC")
        );
    }

    #[test]
    fn test_audit_event_contains_before_and_after_state() {
        let state: State = State::new();
        let command: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("ABC")),
            name: String::from("John Doe"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_ok());
        let transition: TransitionResult = result.unwrap();
        assert_eq!(transition.audit_event.before.data, "users_count=0");
        assert_eq!(transition.audit_event.after.data, "users_count=1");
    }

    #[test]
    fn test_duplicate_initials_returns_error() {
        let mut state: State = State::new();

        // First user
        let command1: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("ABC")),
            name: String::from("John Doe"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result1: Result<TransitionResult, CoreError> =
            apply(&state, command1, actor.clone(), cause.clone());
        assert!(result1.is_ok());
        state = result1.unwrap().new_state;

        // Second user with same initials in same bid year
        let command2: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("ABC")), // Duplicate!
            name: String::from("Jane Smith"),
            area: Area::new(String::from("South")),
            crew: Crew::new(String::from("B")),
            seniority_data: create_test_seniority_data(),
        };

        let result2: Result<TransitionResult, CoreError> = apply(&state, command2, actor, cause);

        assert!(result2.is_err());
        assert!(matches!(
            result2.unwrap_err(),
            CoreError::DomainViolation(DomainError::DuplicateInitials { .. })
        ));
    }

    #[test]
    fn test_duplicate_initials_in_different_bid_years_allowed() {
        let mut state: State = State::new();

        // User in 2026
        let command1: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("ABC")),
            name: String::from("John Doe"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result1: Result<TransitionResult, CoreError> =
            apply(&state, command1, actor.clone(), cause.clone());
        assert!(result1.is_ok());
        state = result1.unwrap().new_state;

        // User with same initials in 2027 (different bid year)
        let command2: Command = Command::RegisterUser {
            bid_year: BidYear::new(2027),
            initials: Initials::new(String::from("ABC")), // Same initials, different bid year
            name: String::from("Jane Smith"),
            area: Area::new(String::from("South")),
            crew: Crew::new(String::from("B")),
            seniority_data: create_test_seniority_data(),
        };

        let result2: Result<TransitionResult, CoreError> = apply(&state, command2, actor, cause);

        assert!(result2.is_ok());
        let transition: TransitionResult = result2.unwrap();
        assert_eq!(transition.new_state.users.len(), 2);
    }

    #[test]
    fn test_invalid_command_with_empty_initials_returns_error() {
        let state: State = State::new();
        let command: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::new()), // Invalid: empty
            name: String::from("John Doe"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CoreError::DomainViolation(DomainError::InvalidInitials(_))
        ));
    }

    #[test]
    fn test_invalid_command_with_empty_name_returns_error() {
        let state: State = State::new();
        let command: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("ABC")),
            name: String::new(), // Invalid: empty
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CoreError::DomainViolation(DomainError::InvalidName(_))
        ));
    }

    #[test]
    fn test_invalid_command_with_empty_area_returns_error() {
        let state: State = State::new();
        let command: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("ABC")),
            name: String::from("John Doe"),
            area: Area::new(String::new()), // Invalid: empty
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CoreError::DomainViolation(DomainError::InvalidArea(_))
        ));
    }

    #[test]
    fn test_invalid_command_with_empty_crew_returns_error() {
        let state: State = State::new();
        let command: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("ABC")),
            name: String::from("John Doe"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::new()), // Invalid: empty
            seniority_data: create_test_seniority_data(),
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CoreError::DomainViolation(DomainError::InvalidCrew(_))
        ));
    }

    #[test]
    fn test_invalid_command_does_not_mutate_state() {
        let state: State = State::new();
        let original_user_count: usize = state.users.len();

        let command: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::new()), // Invalid
            name: String::from("John Doe"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, CoreError> = apply(&state, command, actor, cause);

        assert!(result.is_err());
        // Original state is unchanged
        assert_eq!(state.users.len(), original_user_count);
    }

    #[test]
    fn test_invalid_command_does_not_emit_audit_event() {
        let state: State = State::new();
        let command: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::new()), // Invalid
            name: String::from("John Doe"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
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

        // First user
        let command1: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("ABC")),
            name: String::from("John Doe"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let result1: Result<TransitionResult, CoreError> =
            apply(&state, command1, actor.clone(), cause.clone());
        assert!(result1.is_ok());
        state = result1.unwrap().new_state;
        assert_eq!(state.users.len(), 1);

        // Second user with different initials
        let command2: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("XYZ")),
            name: String::from("Jane Smith"),
            area: Area::new(String::from("South")),
            crew: Crew::new(String::from("B")),
            seniority_data: create_test_seniority_data(),
        };
        let result2: Result<TransitionResult, CoreError> =
            apply(&state, command2, actor.clone(), cause.clone());
        assert!(result2.is_ok());
        state = result2.unwrap().new_state;
        assert_eq!(state.users.len(), 2);

        // Third user in different bid year
        let command3: Command = Command::RegisterUser {
            bid_year: BidYear::new(2027),
            initials: Initials::new(String::from("DEF")),
            name: String::from("Bob Johnson"),
            area: Area::new(String::from("East")),
            crew: Crew::new(String::from("C")),
            seniority_data: create_test_seniority_data(),
        };
        let result3: Result<TransitionResult, CoreError> = apply(&state, command3, actor, cause);
        assert!(result3.is_ok());
        state = result3.unwrap().new_state;
        assert_eq!(state.users.len(), 3);
    }

    #[test]
    fn test_failed_duplicate_initials_transition_does_not_mutate_state() {
        let mut state: State = State::new();

        // First user
        let command1: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("ABC")),
            name: String::from("John Doe"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let result1: Result<TransitionResult, CoreError> =
            apply(&state, command1, actor.clone(), cause.clone());
        assert!(result1.is_ok());
        state = result1.unwrap().new_state;
        let user_count_before_failed_transition: usize = state.users.len();

        // Attempt to add duplicate
        let command2: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("ABC")), // Duplicate
            name: String::from("Jane Smith"),
            area: Area::new(String::from("South")),
            crew: Crew::new(String::from("B")),
            seniority_data: create_test_seniority_data(),
        };

        let result2: Result<TransitionResult, CoreError> = apply(&state, command2, actor, cause);

        assert!(result2.is_err());
        // State should remain unchanged
        assert_eq!(state.users.len(), user_count_before_failed_transition);
    }
}
