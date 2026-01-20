// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::error::DomainError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Represents the lifecycle state of a bid year.
///
/// Phase 25A: Explicit lifecycle states govern what operations are permitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum BidYearLifecycle {
    /// Initial state after creation. Full editing allowed.
    #[default]
    Draft,
    /// Bootstrap phase complete. Ready for canonicalization.
    BootstrapComplete,
    /// Data locked. Canonical tables authoritative.
    Canonicalized,
    /// Bidding rounds in progress.
    BiddingActive,
    /// Bidding finished. System read-only.
    BiddingClosed,
}

impl FromStr for BidYearLifecycle {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(Self::Draft),
            "BootstrapComplete" => Ok(Self::BootstrapComplete),
            "Canonicalized" => Ok(Self::Canonicalized),
            "BiddingActive" => Ok(Self::BiddingActive),
            "BiddingClosed" => Ok(Self::BiddingClosed),
            _ => Err(DomainError::InvalidLifecycleState(s.to_string())),
        }
    }
}

impl std::fmt::Display for BidYearLifecycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl BidYearLifecycle {
    /// Converts this lifecycle state to its string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::BootstrapComplete => "BootstrapComplete",
            Self::Canonicalized => "Canonicalized",
            Self::BiddingActive => "BiddingActive",
            Self::BiddingClosed => "BiddingClosed",
        }
    }

    /// Checks if a transition from this state to another is valid.
    ///
    /// Valid transitions are:
    /// - Draft → `BootstrapComplete`
    /// - `BootstrapComplete` → Canonicalized
    /// - Canonicalized → `BiddingActive`
    /// - `BiddingActive` → `BiddingClosed`
    #[must_use]
    pub const fn can_transition_to(&self, target: Self) -> bool {
        matches!(
            (self, target),
            (Self::Draft, Self::BootstrapComplete)
                | (Self::BootstrapComplete, Self::Canonicalized)
                | (Self::Canonicalized, Self::BiddingActive)
                | (Self::BiddingActive, Self::BiddingClosed)
        )
    }

    /// Returns whether operations are restricted in this lifecycle state.
    ///
    /// Draft and `BootstrapComplete` allow full editing.
    /// Canonicalized and later states restrict editing.
    #[must_use]
    pub const fn is_locked(&self) -> bool {
        matches!(
            self,
            Self::Canonicalized | Self::BiddingActive | Self::BiddingClosed
        )
    }

    /// Returns whether structural changes (area/user creation/deletion) are allowed.
    #[must_use]
    pub const fn allows_structural_changes(&self) -> bool {
        matches!(self, Self::Draft | Self::BootstrapComplete)
    }
}

/// Represents a bid year identifier.
///
/// Phase 23A: A bid year now has a canonical numeric ID (`bid_year_id`)
/// as well as a human-readable year value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidYear {
    /// The canonical numeric identifier assigned by the database.
    /// `None` indicates the bid year has not been persisted yet.
    bid_year_id: Option<i64>,
    /// The year value (e.g., 2026) - used for display only.
    year: u16,
}

// Phase 23A: Custom PartialEq and Hash that ignore bid_year_id
// Two BidYears are equal if they have the same year, regardless of their IDs
impl PartialEq for BidYear {
    fn eq(&self, other: &Self) -> bool {
        self.year == other.year
    }
}

impl Eq for BidYear {}

impl std::hash::Hash for BidYear {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.year.hash(state);
    }
}

impl BidYear {
    /// Creates a new `BidYear` without a persisted ID.
    ///
    /// # Arguments
    ///
    /// * `year` - The year value
    #[must_use]
    pub const fn new(year: u16) -> Self {
        Self {
            bid_year_id: None,
            year,
        }
    }

    /// Creates a `BidYear` with an existing persisted ID.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical numeric identifier
    /// * `year` - The year value
    #[must_use]
    pub const fn with_id(bid_year_id: i64, year: u16) -> Self {
        Self {
            bid_year_id: Some(bid_year_id),
            year,
        }
    }

    /// Returns the canonical numeric identifier if persisted.
    #[must_use]
    pub const fn bid_year_id(&self) -> Option<i64> {
        self.bid_year_id
    }

    /// Returns the year value.
    #[must_use]
    pub const fn year(&self) -> u16 {
        self.year
    }
}

/// Represents a user's initials.
///
/// Initials are the sole identifier for a user within a bid year.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Initials {
    /// The initials value (exactly 2 characters).
    value: String,
}

impl Initials {
    /// Creates new `Initials`.
    ///
    /// Initials are normalized to uppercase to ensure case-insensitive uniqueness.
    ///
    /// # Arguments
    ///
    /// * `value` - The initials value (will be normalized to uppercase)
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_uppercase(),
        }
    }

    /// Returns the initials value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Represents an area identifier.
///
/// Phase 23A: An area now has a canonical numeric ID (`area_id`)
/// as well as a human-readable area code.
///
/// Phase 25B: Areas may be system-managed (e.g., "No Bid").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct Area {
    /// The canonical numeric identifier assigned by the database.
    /// `None` indicates the area has not been persisted yet.
    area_id: Option<i64>,
    /// The area code (e.g., "North", "South") - used for display only.
    /// Normalized to uppercase for consistency.
    area_code: String,
    /// Optional area name for additional context.
    area_name: Option<String>,
    /// Phase 25B: Whether this is a system-managed area (e.g., "No Bid").
    is_system_area: bool,
    /// Phase 29B: The round group this area references.
    /// Non-system areas must reference exactly one round group.
    /// System areas must not reference any round group (None).
    round_group_id: Option<i64>,
}

// Phase 23A: Custom PartialEq and Hash that ignore area_id
// Two Areas are equal if they have the same area_code, regardless of their IDs
impl PartialEq for Area {
    fn eq(&self, other: &Self) -> bool {
        self.area_code == other.area_code
    }
}

impl Eq for Area {}

impl std::hash::Hash for Area {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.area_code.hash(state);
    }
}

impl Area {
    /// The canonical area code for the No Bid system area.
    pub const NO_BID_AREA_CODE: &'static str = "NO BID";

    /// Creates a new regular (non-system) `Area` without a persisted ID.
    ///
    /// Area codes are normalized to uppercase to ensure case-insensitive uniqueness.
    ///
    /// # Arguments
    ///
    /// * `area_code` - The area code (will be normalized to uppercase)
    #[must_use]
    pub fn new(area_code: &str) -> Self {
        Self {
            area_id: None,
            area_code: area_code.to_uppercase(),
            area_name: None,
            is_system_area: false,
            round_group_id: None,
        }
    }

    /// Creates a new system area (e.g., "No Bid") without a persisted ID.
    ///
    /// # Arguments
    ///
    /// * `area_code` - The area code (will be normalized to uppercase)
    #[must_use]
    pub fn new_system_area(area_code: &str) -> Self {
        Self {
            area_id: None,
            area_code: area_code.to_uppercase(),
            area_name: None,
            is_system_area: true,
            round_group_id: None,
        }
    }

    /// Creates an `Area` with an existing persisted ID.
    ///
    /// # Arguments
    ///
    /// * `area_id` - The canonical numeric identifier
    /// * `area_code` - The area code
    /// * `area_name` - Optional area name
    /// * `is_system_area` - Whether this is a system-managed area
    #[must_use]
    pub fn with_id(
        area_id: i64,
        area_code: &str,
        area_name: Option<String>,
        is_system_area: bool,
        round_group_id: Option<i64>,
    ) -> Self {
        Self {
            area_id: Some(area_id),
            area_code: area_code.to_uppercase(),
            area_name,
            is_system_area,
            round_group_id,
        }
    }

    /// Returns the canonical numeric identifier if persisted.
    #[must_use]
    pub const fn area_id(&self) -> Option<i64> {
        self.area_id
    }

    /// Returns the area code.
    #[must_use]
    pub fn area_code(&self) -> &str {
        &self.area_code
    }

    /// Returns the area name if set.
    #[must_use]
    pub fn area_name(&self) -> Option<&str> {
        self.area_name.as_deref()
    }

    /// Returns whether this is a system-managed area.
    #[must_use]
    pub const fn is_system_area(&self) -> bool {
        self.is_system_area
    }

    /// Returns the round group ID this area references.
    ///
    /// Non-system areas should have exactly one round group.
    /// System areas should have none.
    #[must_use]
    pub const fn round_group_id(&self) -> Option<i64> {
        self.round_group_id
    }

    /// Legacy method for backward compatibility - returns `area_code`.
    /// This will be removed in Phase 23B when API layer is updated.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.area_code
    }
}

/// Represents a crew identifier.
///
/// Crews are domain constants numbered 1 through 7.
/// A user may have zero or one crew assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crew {
    /// The crew number (1-7).
    number: u8,
}

impl Crew {
    /// Creates a new `Crew`.
    ///
    /// # Arguments
    ///
    /// * `number` - The crew number (must be between 1 and 7 inclusive)
    ///
    /// # Returns
    ///
    /// * `Ok(Crew)` if the number is valid
    /// * `Err(DomainError::InvalidCrew)` if the number is not between 1 and 7
    ///
    /// # Errors
    ///
    /// Returns an error if the crew number is not in the range 1-7.
    pub const fn new(number: u8) -> Result<Self, DomainError> {
        if number >= 1 && number <= 7 {
            Ok(Self { number })
        } else {
            Err(DomainError::InvalidCrew(
                "Crew number must be between 1 and 7",
            ))
        }
    }

    /// Returns the crew number.
    #[must_use]
    pub const fn number(&self) -> u8 {
        self.number
    }
}

/// Represents a user type classification.
///
/// User types are fixed domain constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserType {
    /// Certified Professional Controller
    CPC,
    /// Certified Professional Controller - In Training
    #[serde(rename = "CPC-IT")]
    CpcIt,
    /// Developmental - Radar
    #[serde(rename = "Dev-R")]
    DevR,
    /// Developmental - Tower
    #[serde(rename = "Dev-D")]
    DevD,
}

impl UserType {
    /// Parses a user type from a string.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to parse
    ///
    /// # Returns
    ///
    /// * `Ok(UserType)` if the string is valid
    /// * `Err(DomainError::InvalidUserType)` if the string is not recognized
    ///
    /// # Errors
    ///
    /// Returns an error if the string does not match a valid user type.
    /// Parses a user type from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string does not match a valid user type.
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        match s {
            "CPC" => Ok(Self::CPC),
            "CPC-IT" => Ok(Self::CpcIt),
            "Dev-R" => Ok(Self::DevR),
            "Dev-D" => Ok(Self::DevD),
            _ => Err(DomainError::InvalidUserType(format!(
                "Unknown user type: {s}"
            ))),
        }
    }

    /// Returns the string representation of this user type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::CPC => "CPC",
            Self::CpcIt => "CPC-IT",
            Self::DevR => "Dev-R",
            Self::DevD => "Dev-D",
        }
    }
}

/// Represents seniority-related data for a user.
///
/// This data exists as domain data but must NOT be used for ordering,
/// ranking, or decision-making in Phase 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeniorityData {
    /// Cumulative NATCA bargaining unit date (ISO 8601 date string).
    pub cumulative_natca_bu_date: String,
    /// NATCA bargaining unit date (ISO 8601 date string).
    pub natca_bu_date: String,
    /// Entry on Duty / FAA date (ISO 8601 date string).
    pub eod_faa_date: String,
    /// Service Computation Date (ISO 8601 date string).
    pub service_computation_date: String,
    /// Optional lottery value for tie-breaking (not used in Phase 1).
    pub lottery_value: Option<u32>,
}

impl SeniorityData {
    /// Creates new `SeniorityData`.
    ///
    /// # Arguments
    ///
    /// * `cumulative_natca_bu_date` - Cumulative NATCA bargaining unit date (ISO 8601 date)
    /// * `natca_bu_date` - NATCA bargaining unit date (ISO 8601 date)
    /// * `eod_faa_date` - Entry on Duty / FAA date
    /// * `service_computation_date` - Service Computation Date
    /// * `lottery_value` - Optional lottery value
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        cumulative_natca_bu_date: String,
        natca_bu_date: String,
        eod_faa_date: String,
        service_computation_date: String,
        lottery_value: Option<u32>,
    ) -> Self {
        Self {
            cumulative_natca_bu_date,
            natca_bu_date,
            eod_faa_date,
            service_computation_date,
            lottery_value,
        }
    }
}

/// Represents a user within a bid year.
///
/// Users are scoped to a single bid year.
/// `user_id` is the canonical internal identifier.
/// Initials remain unique per bid year but are no longer the primary identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// Canonical internal identifier (opaque, stable, immutable).
    /// Optional to support creation before persistence.
    pub user_id: Option<i64>,
    /// The bid year this user belongs to.
    pub bid_year: BidYear,
    /// The user's initials (unique per bid year, but not the canonical identifier).
    pub initials: Initials,
    /// The user's name (informational, not unique).
    pub name: String,
    /// The area this user belongs to.
    pub area: Area,
    /// The user's type classification.
    pub user_type: UserType,
    /// The crew this user belongs to (optional).
    pub crew: Option<Crew>,
    /// Seniority-related data (informational only in Phase 1).
    pub seniority_data: SeniorityData,
    /// Phase 29A: Whether this user is excluded from bidding.
    /// If true, user is excluded from bid order derivation and does not receive a bid window.
    pub excluded_from_bidding: bool,
    /// Phase 29A: Whether this user is excluded from leave calculation.
    /// If true, user does not count toward area leave capacity or maximum bid slots.
    /// Directional invariant: `excluded_from_leave_calculation` => `excluded_from_bidding`
    pub excluded_from_leave_calculation: bool,
}

impl User {
    /// Creates a new `User` without a persisted `user_id`.
    ///
    /// The `user_id` will be assigned by the persistence layer upon first save.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `initials` - The user's initials
    /// * `name` - The user's name
    /// * `area` - The user's area
    /// * `user_type` - The user's type classification
    /// * `crew` - The user's crew (optional)
    /// * `seniority_data` - The user's seniority data
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        bid_year: BidYear,
        initials: Initials,
        name: String,
        area: Area,
        user_type: UserType,
        crew: Option<Crew>,
        seniority_data: SeniorityData,
        excluded_from_bidding: bool,
        excluded_from_leave_calculation: bool,
    ) -> Self {
        Self {
            user_id: None,
            bid_year,
            initials,
            name,
            area,
            user_type,
            crew,
            seniority_data,
            excluded_from_bidding,
            excluded_from_leave_calculation,
        }
    }

    /// Creates a `User` with an existing `user_id` (from persistence).
    ///
    /// # Arguments
    ///
    /// * `user_id` - The canonical internal identifier
    /// * `bid_year` - The bid year
    /// * `initials` - The user's initials
    /// * `name` - The user's name
    /// * `area` - The user's area
    /// * `user_type` - The user's type classification
    /// * `crew` - The user's crew (optional)
    /// * `seniority_data` - The user's seniority data
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn with_id(
        user_id: i64,
        bid_year: BidYear,
        initials: Initials,
        name: String,
        area: Area,
        user_type: UserType,
        crew: Option<Crew>,
        seniority_data: SeniorityData,
        excluded_from_bidding: bool,
        excluded_from_leave_calculation: bool,
    ) -> Self {
        Self {
            user_id: Some(user_id),
            bid_year,
            initials,
            name,
            area,
            user_type,
            crew,
            seniority_data,
            excluded_from_bidding,
            excluded_from_leave_calculation,
        }
    }

    /// Validates the directional invariant for participation flags.
    ///
    /// Validates the participation flag directional invariant.
    ///
    /// # Invariant
    ///
    /// `excluded_from_leave_calculation == true` ⇒ `excluded_from_bidding == true`
    ///
    /// A user may never be included in bidding while excluded from leave calculation.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the invariant holds.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::ParticipationFlagViolation` if `excluded_from_leave_calculation`
    /// is true but `excluded_from_bidding` is false.
    pub fn validate_participation_flags(&self) -> Result<(), DomainError> {
        if self.excluded_from_leave_calculation && !self.excluded_from_bidding {
            return Err(DomainError::ParticipationFlagViolation {
                user_initials: self.initials.value().to_owned(),
                reason: String::from(
                    "User excluded from leave calculation must also be excluded from bidding",
                ),
            });
        }
        Ok(())
    }
}

// ============================================================================
// Phase 29B: Round Groups and Rounds
// ============================================================================

/// A reusable configuration set for rounds.
///
/// Round groups define common rule sets that can be applied to multiple rounds
/// across different areas within a bid year.
#[allow(dead_code)]
#[allow(clippy::struct_field_names)]
pub struct RoundGroup {
    /// The canonical numeric identifier assigned by the database.
    /// `None` indicates the round group has not been persisted yet.
    round_group_id: Option<i64>,
    /// The bid year this round group belongs to.
    bid_year: BidYear,
    /// The name of this round group (unique within the bid year).
    name: String,
    /// Whether editing is enabled for this round group.
    /// When false, rounds using this group should not be modifiable.
    editing_enabled: bool,
}

#[allow(dead_code)]
impl RoundGroup {
    /// Creates a new `RoundGroup` without a persisted ID.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year this round group belongs to
    /// * `name` - The name of this round group
    /// * `editing_enabled` - Whether editing is enabled
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(bid_year: BidYear, name: String, editing_enabled: bool) -> Self {
        Self {
            round_group_id: None,
            bid_year,
            name,
            editing_enabled,
        }
    }

    /// Creates a `RoundGroup` with an existing persisted ID.
    ///
    /// # Arguments
    ///
    /// * `round_group_id` - The canonical numeric identifier
    /// * `bid_year` - The bid year this round group belongs to
    /// * `name` - The name of this round group
    /// * `editing_enabled` - Whether editing is enabled
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_id(
        round_group_id: i64,
        bid_year: BidYear,
        name: String,
        editing_enabled: bool,
    ) -> Self {
        Self {
            round_group_id: Some(round_group_id),
            bid_year,
            name,
            editing_enabled,
        }
    }

    /// Returns the canonical numeric identifier if persisted.
    #[must_use]
    pub const fn round_group_id(&self) -> Option<i64> {
        self.round_group_id
    }

    /// Returns the bid year this round group belongs to.
    #[must_use]
    pub const fn bid_year(&self) -> &BidYear {
        &self.bid_year
    }

    /// Returns the name of this round group.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns whether editing is enabled for this round group.
    #[must_use]
    pub const fn editing_enabled(&self) -> bool {
        self.editing_enabled
    }

    /// Validates the round group configuration constraints.
    ///
    /// Ensures that:
    /// - `name` is not empty
    ///
    /// # Returns
    ///
    /// `Ok(())` if all constraints are satisfied.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::InvalidRoundConfiguration` if any constraint is violated.
    #[allow(dead_code)]
    pub fn validate_constraints(&self) -> Result<(), crate::error::DomainError> {
        if self.name.trim().is_empty() {
            return Err(crate::error::DomainError::InvalidRoundConfiguration {
                reason: String::from("round group name cannot be empty"),
            });
        }
        Ok(())
    }
}

/// A bidding round within a round group.
///
/// Rounds carry all bidding rule fields and belong to round groups.
/// Round groups are reusable collections of rounds that areas reference.
#[allow(dead_code)]
#[allow(clippy::struct_field_names)]
pub struct Round {
    /// The canonical numeric identifier assigned by the database.
    /// `None` indicates the round has not been persisted yet.
    round_id: Option<i64>,
    /// The round group this round belongs to.
    round_group: RoundGroup,
    /// The round number (unique within the round group).
    /// Determines the order in which rounds are executed.
    round_number: u32,
    /// The display name for this round.
    name: String,
    /// Maximum number of slots that can be bid per day.
    slots_per_day: u32,
    /// Maximum number of groups (consecutive day sequences) that can be bid.
    max_groups: u32,
    /// Maximum total hours that can be bid.
    max_total_hours: u32,
    /// Whether holidays are included in bid groups.
    /// If false, holidays do not count toward group length.
    include_holidays: bool,
    /// Whether overbidding is allowed.
    /// If true, accrued leave limits are ignored (round limits still apply).
    /// Typically used for carryover rounds.
    allow_overbid: bool,
}

#[allow(dead_code)]
impl Round {
    /// Creates a new `Round` without a persisted ID.
    ///
    /// # Arguments
    ///
    /// * `round_group` - The round group this round belongs to
    /// * `round_number` - The round number (unique within round group)
    /// * `name` - The display name for this round
    /// * `slots_per_day` - Maximum slots per day
    /// * `max_groups` - Maximum number of groups
    /// * `max_total_hours` - Maximum total hours
    /// * `include_holidays` - Whether holidays are included
    /// * `allow_overbid` - Whether overbidding is allowed
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(
        round_group: RoundGroup,
        round_number: u32,
        name: String,
        slots_per_day: u32,
        max_groups: u32,
        max_total_hours: u32,
        include_holidays: bool,
        allow_overbid: bool,
    ) -> Self {
        Self {
            round_id: None,
            round_group,
            round_number,
            name,
            slots_per_day,
            max_groups,
            max_total_hours,
            include_holidays,
            allow_overbid,
        }
    }

    /// Creates a `Round` with an existing persisted ID.
    ///
    /// # Arguments
    ///
    /// * `round_id` - The canonical numeric identifier
    /// * `round_group` - The round group this round belongs to
    /// * `round_number` - The round number (unique within round group)
    /// * `name` - The display name for this round
    /// * `slots_per_day` - Maximum slots per day
    /// * `max_groups` - Maximum number of groups
    /// * `max_total_hours` - Maximum total hours
    /// * `include_holidays` - Whether holidays are included
    /// * `allow_overbid` - Whether overbidding is allowed
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_id(
        round_id: i64,
        round_group: RoundGroup,
        round_number: u32,
        name: String,
        slots_per_day: u32,
        max_groups: u32,
        max_total_hours: u32,
        include_holidays: bool,
        allow_overbid: bool,
    ) -> Self {
        Self {
            round_id: Some(round_id),
            round_group,
            round_number,
            name,
            slots_per_day,
            max_groups,
            max_total_hours,
            include_holidays,
            allow_overbid,
        }
    }

    /// Returns the canonical numeric identifier if persisted.
    #[must_use]
    pub const fn round_id(&self) -> Option<i64> {
        self.round_id
    }

    /// Returns the round group this round belongs to.
    #[must_use]
    pub const fn round_group(&self) -> &RoundGroup {
        &self.round_group
    }

    /// Returns the round number.
    #[must_use]
    pub const fn round_number(&self) -> u32 {
        self.round_number
    }

    /// Returns the display name for this round.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the maximum number of slots per day.
    #[must_use]
    pub const fn slots_per_day(&self) -> u32 {
        self.slots_per_day
    }

    /// Returns the maximum number of groups.
    #[must_use]
    pub const fn max_groups(&self) -> u32 {
        self.max_groups
    }

    /// Returns the maximum total hours.
    #[must_use]
    pub const fn max_total_hours(&self) -> u32 {
        self.max_total_hours
    }

    /// Returns whether holidays are included in groups.
    #[must_use]
    pub const fn include_holidays(&self) -> bool {
        self.include_holidays
    }

    /// Returns whether overbidding is allowed.
    #[must_use]
    pub const fn allow_overbid(&self) -> bool {
        self.allow_overbid
    }

    /// Validates the round configuration constraints.
    ///
    /// Ensures that:
    /// - `slots_per_day` is greater than 0
    /// - `max_groups` is greater than 0
    /// - `max_total_hours` is greater than 0
    /// - `name` is not empty
    ///
    /// # Returns
    ///
    /// `Ok(())` if all constraints are satisfied.
    ///
    /// # Errors
    ///
    /// Returns `DomainError::InvalidRoundConfiguration` if any constraint is violated.
    #[allow(dead_code)]
    pub fn validate_constraints(&self) -> Result<(), crate::error::DomainError> {
        if self.slots_per_day == 0 {
            return Err(crate::error::DomainError::InvalidRoundConfiguration {
                reason: String::from("slots_per_day must be greater than 0"),
            });
        }
        if self.max_groups == 0 {
            return Err(crate::error::DomainError::InvalidRoundConfiguration {
                reason: String::from("max_groups must be greater than 0"),
            });
        }
        if self.max_total_hours == 0 {
            return Err(crate::error::DomainError::InvalidRoundConfiguration {
                reason: String::from("max_total_hours must be greater than 0"),
            });
        }
        if self.name.trim().is_empty() {
            return Err(crate::error::DomainError::InvalidRoundConfiguration {
                reason: String::from("name cannot be empty"),
            });
        }
        Ok(())
    }
}
