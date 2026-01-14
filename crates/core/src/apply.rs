// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::command::Command;
use crate::error::CoreError;
use crate::state::{BootstrapMetadata, BootstrapResult, State, TransitionResult};
use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::{
    Area, BidYear, CanonicalBidYear, DomainError, User, validate_bid_year,
    validate_initials_unique, validate_user_fields,
};

/// Applies a bootstrap command to the metadata, producing new metadata and audit event.
///
/// Bootstrap commands (`CreateBidYear`, `CreateArea`) operate on global metadata.
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata (immutable)
/// * `command` - The bootstrap command to apply
/// * `actor` - The actor performing this action
/// * `cause` - The cause or reason for this action
///
/// # Returns
///
/// * `Ok(BootstrapResult)` containing the new metadata and audit event
/// * `Err(CoreError)` if the command is invalid
///
/// # Errors
///
/// Returns an error if:
/// - The command violates domain rules
#[allow(clippy::too_many_lines)]
pub fn apply_bootstrap(
    metadata: &BootstrapMetadata,
    active_bid_year: &BidYear,
    command: Command,
    actor: Actor,
    cause: Cause,
) -> Result<BootstrapResult, CoreError> {
    match command {
        Command::CreateBidYear {
            year,
            start_date,
            num_pay_periods,
        } => {
            // Validate the year is reasonable
            validate_bid_year(year)?;

            // Construct and validate canonical bid year from provided metadata
            let canonical_bid_year: CanonicalBidYear =
                CanonicalBidYear::new(year, start_date, num_pay_periods)
                    .map_err(CoreError::DomainViolation)?;

            let bid_year: BidYear = BidYear::new(year);

            // Check for duplicate
            if metadata.has_bid_year(&bid_year) {
                return Err(CoreError::DomainViolation(DomainError::DuplicateBidYear(
                    year,
                )));
            }

            // Create new metadata with bid year added
            let mut new_metadata: BootstrapMetadata = metadata.clone();
            new_metadata.add_bid_year(bid_year.clone());

            // Create audit event (not scoped to area since this is global)
            let before: StateSnapshot =
                StateSnapshot::new(format!("bid_years_count={}", metadata.bid_years.len()));
            let after: StateSnapshot =
                StateSnapshot::new(format!("bid_years_count={}", new_metadata.bid_years.len()));

            let action: Action = Action::new(
                String::from("CreateBidYear"),
                Some(format!(
                    "Created bid year {year} (start: {start_date}, periods: {num_pay_periods})"
                )),
            );

            // Use a placeholder area for global operations
            let placeholder_area: Area = Area::new("_global");
            let audit_event: AuditEvent = AuditEvent::new(
                actor,
                cause,
                action,
                before,
                after,
                bid_year,
                placeholder_area,
            );

            Ok(BootstrapResult {
                new_metadata,
                audit_event,
                canonical_bid_year: Some(canonical_bid_year),
            })
        }
        Command::CreateArea { area_id } => {
            // Use the active bid year
            let bid_year = active_bid_year;

            // Check if bid year exists
            if !metadata.has_bid_year(bid_year) {
                return Err(CoreError::DomainViolation(DomainError::BidYearNotFound(
                    bid_year.year(),
                )));
            }

            let area: Area = Area::new(&area_id);

            // Check for duplicate
            if metadata.has_area(bid_year, &area) {
                return Err(CoreError::DomainViolation(DomainError::DuplicateArea {
                    bid_year: bid_year.year(),
                    area: area_id,
                }));
            }

            // Create new metadata with area added
            let mut new_metadata: BootstrapMetadata = metadata.clone();
            new_metadata.add_area(bid_year.clone(), area.clone());

            // Create audit event
            let before: StateSnapshot =
                StateSnapshot::new(format!("areas_count={}", metadata.areas.len()));
            let after: StateSnapshot =
                StateSnapshot::new(format!("areas_count={}", new_metadata.areas.len()));

            let action: Action = Action::new(
                String::from("CreateArea"),
                Some(format!(
                    "Created area '{}' in bid year {}",
                    area.id(),
                    bid_year.year()
                )),
            );

            let audit_event: AuditEvent =
                AuditEvent::new(actor, cause, action, before, after, bid_year.clone(), area);

            Ok(BootstrapResult {
                new_metadata,
                audit_event,
                canonical_bid_year: None,
            })
        }
        Command::SetActiveBidYear { year } => {
            // Validate the year is reasonable
            validate_bid_year(year)?;

            let bid_year: BidYear = BidYear::new(year);

            // Check if bid year exists
            if !metadata.has_bid_year(&bid_year) {
                return Err(CoreError::DomainViolation(DomainError::BidYearNotFound(
                    year,
                )));
            }

            // Create new metadata (unchanged, active status is managed in persistence)
            let new_metadata: BootstrapMetadata = metadata.clone();

            // Create audit event
            let before: StateSnapshot = StateSnapshot::new(String::from("active_bid_year_change"));
            let after: StateSnapshot = StateSnapshot::new(format!("active_bid_year={year}"));

            let action: Action = Action::new(
                String::from("SetActiveBidYear"),
                Some(format!("Set bid year {year} as active")),
            );

            // SetActiveBidYear is a bid-year-level operation without an area
            let audit_event: AuditEvent = AuditEvent {
                event_id: None,
                actor,
                cause,
                action,
                before,
                after,
                bid_year: Some(bid_year),
                area: None,
            };

            Ok(BootstrapResult {
                new_metadata,
                audit_event,
                canonical_bid_year: None,
            })
        }
        Command::SetExpectedAreaCount { expected_count } => {
            // Use the active bid year
            let bid_year = active_bid_year;

            // Validate bid year exists
            if !metadata.has_bid_year(bid_year) {
                return Err(CoreError::DomainViolation(DomainError::BidYearNotFound(
                    bid_year.year(),
                )));
            }

            // Validate count is positive
            if expected_count == 0 {
                return Err(CoreError::DomainViolation(
                    DomainError::InvalidExpectedAreaCount {
                        count: expected_count,
                    },
                ));
            }

            // Create new metadata (unchanged, expected counts are managed in persistence)
            let new_metadata: BootstrapMetadata = metadata.clone();

            // Create audit event
            let before: StateSnapshot =
                StateSnapshot::new(String::from("expected_area_count_change"));
            let after: StateSnapshot =
                StateSnapshot::new(format!("expected_area_count={expected_count}"));

            let action: Action = Action::new(
                String::from("SetExpectedAreaCount"),
                Some(format!(
                    "Set expected area count to {expected_count} for bid year {}",
                    bid_year.year()
                )),
            );

            // SetExpectedAreaCount is a bid-year-level operation without an area
            let audit_event: AuditEvent = AuditEvent {
                event_id: None,
                actor,
                cause,
                action,
                before,
                after,
                bid_year: Some(bid_year.clone()),
                area: None,
            };

            Ok(BootstrapResult {
                new_metadata,
                audit_event,
                canonical_bid_year: None,
            })
        }
        Command::SetExpectedUserCount {
            area,
            expected_count,
        } => {
            // Use the active bid year
            let bid_year = active_bid_year;

            // Validate bid year exists
            if !metadata.has_bid_year(bid_year) {
                return Err(CoreError::DomainViolation(DomainError::BidYearNotFound(
                    bid_year.year(),
                )));
            }

            // Validate area exists in bid year
            if !metadata.has_area(bid_year, &area) {
                return Err(CoreError::DomainViolation(DomainError::AreaNotFound {
                    bid_year: bid_year.year(),
                    area: area.id().to_string(),
                }));
            }

            // Validate count is positive
            if expected_count == 0 {
                return Err(CoreError::DomainViolation(
                    DomainError::InvalidExpectedUserCount {
                        count: expected_count,
                    },
                ));
            }

            // Create new metadata (unchanged, expected counts are managed in persistence)
            let new_metadata: BootstrapMetadata = metadata.clone();

            // Create audit event
            let before: StateSnapshot =
                StateSnapshot::new(String::from("expected_user_count_change"));
            let after: StateSnapshot =
                StateSnapshot::new(format!("expected_user_count={expected_count}"));

            let action: Action = Action::new(
                String::from("SetExpectedUserCount"),
                Some(format!(
                    "Set expected user count to {expected_count} for area '{}' in bid year {}",
                    area.id(),
                    bid_year.year()
                )),
            );

            let audit_event: AuditEvent =
                AuditEvent::new(actor, cause, action, before, after, bid_year.clone(), area);

            Ok(BootstrapResult {
                new_metadata,
                audit_event,
                canonical_bid_year: None,
            })
        }
        _ => {
            // Non-bootstrap commands should use apply() instead
            unreachable!("apply_bootstrap called with non-bootstrap command")
        }
    }
}

/// Applies a command to the state, producing a new state and audit event.
///
/// Commands are validated and applied atomically. Either they succeed completely
/// or they fail without side effects.
///
/// # Arguments
///
/// * `metadata` - The bootstrap metadata (immutable)
/// * `state` - The current state (immutable)
/// * `active_bid_year` - The active bid year (must be validated by caller)
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
/// - The user already exists (for `RegisterUser`)
#[allow(clippy::too_many_lines)]
pub fn apply(
    metadata: &BootstrapMetadata,
    state: &State,
    active_bid_year: &BidYear,
    command: Command,
    actor: Actor,
    cause: Cause,
) -> Result<TransitionResult, CoreError> {
    match command {
        Command::RegisterUser {
            initials,
            name,
            area,
            user_type,
            crew,
            seniority_data,
        } => {
            // Use the active bid year
            let bid_year = active_bid_year;

            // Validate bid year exists
            if !metadata.has_bid_year(bid_year) {
                return Err(CoreError::DomainViolation(DomainError::BidYearNotFound(
                    bid_year.year(),
                )));
            }

            // Validate area exists in bid year
            if !metadata.has_area(bid_year, &area) {
                return Err(CoreError::DomainViolation(DomainError::AreaNotFound {
                    bid_year: bid_year.year(),
                    area: area.id().to_string(),
                }));
            }

            // Create the user object
            let user: User = User::new(
                bid_year.clone(),
                initials.clone(),
                name,
                area,
                user_type,
                crew,
                seniority_data,
            );

            // Validate user field constraints
            validate_user_fields(&user)?;

            // Validate initials are unique within the bid year
            validate_initials_unique(bid_year, &initials, &state.users)?;

            // Capture state before transition
            let before: StateSnapshot = state.to_snapshot();

            // Create new state with the user added
            let mut new_users: Vec<User> = state.users.clone();
            new_users.push(user);
            let new_state: State = State {
                bid_year: state.bid_year.clone(),
                area: state.area.clone(),
                users: new_users,
            };

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
            let audit_event: AuditEvent = AuditEvent::new(
                actor,
                cause,
                action,
                before,
                after,
                state.bid_year.clone(),
                state.area.clone(),
            );

            Ok(TransitionResult {
                new_state,
                audit_event,
            })
        }
        Command::Checkpoint => {
            // Checkpoint creates a snapshot without changing state
            let before: StateSnapshot = state.to_snapshot();
            let after: StateSnapshot = state.to_snapshot();

            let action: Action = Action::new(
                String::from("Checkpoint"),
                Some(String::from("Explicit checkpoint created")),
            );

            let audit_event: AuditEvent = AuditEvent::new(
                actor,
                cause,
                action,
                before,
                after,
                state.bid_year.clone(),
                state.area.clone(),
            );

            Ok(TransitionResult {
                new_state: state.clone(),
                audit_event,
            })
        }
        Command::Finalize => {
            // Finalize marks a milestone without changing state
            let before: StateSnapshot = state.to_snapshot();
            let after: StateSnapshot = state.to_snapshot();

            let action: Action = Action::new(
                String::from("Finalize"),
                Some(String::from("Milestone finalized")),
            );

            let audit_event: AuditEvent = AuditEvent::new(
                actor,
                cause,
                action,
                before,
                after,
                state.bid_year.clone(),
                state.area.clone(),
            );

            Ok(TransitionResult {
                new_state: state.clone(),
                audit_event,
            })
        }
        Command::RollbackToEventId { target_event_id } => {
            // Rollback creates a new audit event that references a prior event
            // The actual state reconstruction from the target event would be done
            // by the persistence layer when replaying events
            // For now, this just creates the rollback audit event
            let before: StateSnapshot = state.to_snapshot();
            let after: StateSnapshot = state.to_snapshot();

            let action: Action = Action::new(
                String::from("Rollback"),
                Some(format!("Rolled back to event ID {target_event_id}")),
            );

            let audit_event: AuditEvent = AuditEvent::new(
                actor,
                cause,
                action,
                before,
                after,
                state.bid_year.clone(),
                state.area.clone(),
            );

            Ok(TransitionResult {
                new_state: state.clone(),
                audit_event,
            })
        }
        Command::UpdateUser {
            initials,
            name,
            area,
            user_type,
            crew,
            seniority_data,
        } => {
            // Use the active bid year
            let bid_year = active_bid_year;

            // Validate bid year exists
            if !metadata.has_bid_year(bid_year) {
                return Err(CoreError::DomainViolation(DomainError::BidYearNotFound(
                    bid_year.year(),
                )));
            }

            // Validate area exists in bid year
            if !metadata.has_area(bid_year, &area) {
                return Err(CoreError::DomainViolation(DomainError::AreaNotFound {
                    bid_year: bid_year.year(),
                    area: area.id().to_string(),
                }));
            }

            // Find the user to update
            let user_index: Option<usize> = state
                .users
                .iter()
                .position(|u| u.initials == initials && &u.bid_year == bid_year);

            let user_index: usize = user_index.ok_or_else(|| {
                CoreError::DomainViolation(DomainError::UserNotFound {
                    bid_year: bid_year.year(),
                    area: area.id().to_string(),
                    initials: initials.value().to_string(),
                })
            })?;

            // Create the updated user object
            let updated_user: User = User::new(
                bid_year.clone(),
                initials.clone(),
                name,
                area,
                user_type,
                crew,
                seniority_data,
            );

            // Validate user field constraints
            validate_user_fields(&updated_user)?;

            // Capture state before transition
            let before: StateSnapshot = state.to_snapshot();

            // Create new state with the user updated
            let mut new_users: Vec<User> = state.users.clone();
            new_users[user_index] = updated_user;
            let new_state: State = State {
                bid_year: state.bid_year.clone(),
                area: state.area.clone(),
                users: new_users,
            };

            // Capture state after transition
            let after: StateSnapshot = new_state.to_snapshot();

            // Create audit event
            let action: Action = Action::new(
                String::from("UpdateUser"),
                Some(format!(
                    "Updated user with initials '{}' for bid year {}",
                    initials.value(),
                    bid_year.year()
                )),
            );
            let audit_event: AuditEvent = AuditEvent::new(
                actor,
                cause,
                action,
                before,
                after,
                state.bid_year.clone(),
                state.area.clone(),
            );

            Ok(TransitionResult {
                new_state,
                audit_event,
            })
        }
        Command::CreateBidYear { .. }
        | Command::CreateArea { .. }
        | Command::SetActiveBidYear { .. }
        | Command::SetExpectedAreaCount { .. }
        | Command::SetExpectedUserCount { .. } => {
            // Bootstrap commands should use apply_bootstrap() instead
            unreachable!("apply called with bootstrap command")
        }
    }
}
