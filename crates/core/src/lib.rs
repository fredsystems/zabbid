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

use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::{
    Area, BidYear, Crew, DomainError, Initials, SeniorityData, User, UserType, validate_bid_year,
    validate_initials_unique, validate_user_fields,
};

/// A command represents user or system intent as data only.
///
/// Commands are the only way to request state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Create a new bid year.
    CreateBidYear {
        /// The year value.
        year: u16,
    },
    /// Create a new area within a bid year.
    CreateArea {
        /// The bid year this area belongs to.
        bid_year: BidYear,
        /// The area identifier.
        area_id: String,
    },
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
        /// The user's type classification.
        user_type: UserType,
        /// The user's crew (optional).
        crew: Option<Crew>,
        /// The user's seniority data.
        seniority_data: SeniorityData,
    },
    /// Create an explicit checkpoint, triggering a full state snapshot.
    Checkpoint,
    /// Mark a milestone as finalized, triggering a full state snapshot.
    Finalize,
    /// Rollback to a specific event ID, establishing it as authoritative going forward.
    /// This creates a new audit event and triggers a full state snapshot.
    RollbackToEventId {
        /// The event ID to rollback to.
        /// Must be within the same `(bid_year, area)` scope.
        target_event_id: i64,
    },
}

/// Bootstrap metadata tracking which bid years and areas exist.
///
/// This is separate from the scoped State and represents global system metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapMetadata {
    /// All valid bid years that have been created.
    pub bid_years: Vec<BidYear>,
    /// All valid areas per bid year.
    pub areas: Vec<(BidYear, Area)>,
}

impl BootstrapMetadata {
    /// Creates a new empty bootstrap metadata.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bid_years: Vec::new(),
            areas: Vec::new(),
        }
    }

    /// Checks if a bid year exists.
    #[must_use]
    pub fn has_bid_year(&self, bid_year: &BidYear) -> bool {
        self.bid_years.contains(bid_year)
    }

    /// Checks if an area exists in a bid year.
    #[must_use]
    pub fn has_area(&self, bid_year: &BidYear, area: &Area) -> bool {
        self.areas.iter().any(|(y, a)| y == bid_year && a == area)
    }

    /// Adds a bid year.
    fn add_bid_year(&mut self, bid_year: BidYear) {
        self.bid_years.push(bid_year);
    }

    /// Adds an area to a bid year.
    fn add_area(&mut self, bid_year: BidYear, area: Area) {
        self.areas.push((bid_year, area));
    }
}

impl Default for BootstrapMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates that a bid year exists in the metadata.
///
/// This is a read-only validation that does not create audit events.
///
/// # Arguments
///
/// * `metadata` - The bootstrap metadata to check
/// * `bid_year` - The bid year to validate
///
/// # Returns
///
/// * `Ok(())` if the bid year exists
/// * `Err(DomainError::BidYearNotFound)` if the bid year does not exist
///
/// # Errors
///
/// Returns an error if the bid year has not been created.
pub fn validate_bid_year_exists(
    metadata: &BootstrapMetadata,
    bid_year: &BidYear,
) -> Result<(), DomainError> {
    if !metadata.has_bid_year(bid_year) {
        return Err(DomainError::BidYearNotFound(bid_year.year()));
    }
    Ok(())
}

/// Validates that an area exists in the specified bid year.
///
/// This is a read-only validation that does not create audit events.
/// This function also validates that the bid year exists.
///
/// # Arguments
///
/// * `metadata` - The bootstrap metadata to check
/// * `bid_year` - The bid year to check within
/// * `area` - The area to validate
///
/// # Returns
///
/// * `Ok(())` if both the bid year and area exist
/// * `Err(DomainError::BidYearNotFound)` if the bid year does not exist
/// * `Err(DomainError::AreaNotFound)` if the area does not exist in the bid year
///
/// # Errors
///
/// Returns an error if:
/// - The bid year has not been created
/// - The area has not been created in the bid year
pub fn validate_area_exists(
    metadata: &BootstrapMetadata,
    bid_year: &BidYear,
    area: &Area,
) -> Result<(), DomainError> {
    // First validate bid year exists
    validate_bid_year_exists(metadata, bid_year)?;

    // Then validate area exists in that bid year
    if !metadata.has_area(bid_year, area) {
        return Err(DomainError::AreaNotFound {
            bid_year: bid_year.year(),
            area: area.id().to_string(),
        });
    }
    Ok(())
}

/// The complete system state scoped to a single `(bid_year, area)` pair.
///
/// State is now scoped to one bid year and one area combination.
/// This enables proper persistence and audit scoping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// The bid year this state is scoped to.
    pub bid_year: BidYear,
    /// The area this state is scoped to.
    pub area: Area,
    /// All registered users for this `(bid_year, area)`.
    pub users: Vec<User>,
}

impl State {
    /// Creates a new empty state for a given bid year and area.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year this state is scoped to
    /// * `area` - The area this state is scoped to
    #[must_use]
    pub const fn new(bid_year: BidYear, area: Area) -> Self {
        Self {
            bid_year,
            area,
            users: Vec::new(),
        }
    }

    /// Converts the state to a snapshot for audit purposes.
    #[must_use]
    pub fn to_snapshot(&self) -> StateSnapshot {
        StateSnapshot::new(format!(
            "bid_year={},area={},users_count={}",
            self.bid_year.year(),
            self.area.id(),
            self.users.len()
        ))
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

/// The result of a bootstrap operation.
///
/// Bootstrap operations modify metadata, not scoped state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapResult {
    /// The new bootstrap metadata after the operation.
    pub new_metadata: BootstrapMetadata,
    /// The audit event recording this operation.
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
    command: Command,
    actor: Actor,
    cause: Cause,
) -> Result<BootstrapResult, CoreError> {
    match command {
        Command::CreateBidYear { year } => {
            // Validate the year is reasonable
            validate_bid_year(year)?;

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
                Some(format!("Created bid year {year}")),
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
            })
        }
        Command::CreateArea { bid_year, area_id } => {
            // Check if bid year exists
            if !metadata.has_bid_year(&bid_year) {
                return Err(CoreError::DomainViolation(DomainError::BidYearNotFound(
                    bid_year.year(),
                )));
            }

            let area: Area = Area::new(&area_id);

            // Check for duplicate
            if metadata.has_area(&bid_year, &area) {
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
                AuditEvent::new(actor, cause, action, before, after, bid_year, area);

            Ok(BootstrapResult {
                new_metadata,
                audit_event,
            })
        }
        _ => {
            // Non-bootstrap commands should use apply() instead
            unreachable!("apply_bootstrap called with non-bootstrap command")
        }
    }
}

/// Applies a command to the current state, producing a new state and audit event.
///
/// This function handles user-scoped commands within a (`bid_year`, `area`) scope.
///
/// # Arguments
///
/// * `metadata` - The bootstrap metadata (for validation)
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
/// - The bid year or area does not exist
#[allow(clippy::too_many_lines)]
pub fn apply(
    metadata: &BootstrapMetadata,
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
            user_type,
            crew,
            seniority_data,
        } => {
            // Validate bid year exists
            if !metadata.has_bid_year(&bid_year) {
                return Err(CoreError::DomainViolation(DomainError::BidYearNotFound(
                    bid_year.year(),
                )));
            }

            // Validate area exists in bid year
            if !metadata.has_area(&bid_year, &area) {
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
            validate_initials_unique(&bid_year, &initials, &state.users)?;

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
        Command::CreateBidYear { .. } | Command::CreateArea { .. } => {
            // Bootstrap commands should use apply_bootstrap() instead
            unreachable!("apply called with bootstrap command")
        }
    }
}
