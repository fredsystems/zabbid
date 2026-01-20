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
    ) -> Self {
        Self {
            area_id: Some(area_id),
            area_code: area_code.to_uppercase(),
            area_name,
            is_system_area,
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
