// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::error::DomainError;
use serde::{Deserialize, Serialize};

/// Represents a bid year identifier.
///
/// A bid year is the scope within which users are identified and rules are enforced.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BidYear {
    /// The year value (e.g., 2026).
    year: u16,
}

impl BidYear {
    /// Creates a new `BidYear`.
    ///
    /// # Arguments
    ///
    /// * `year` - The year value
    #[must_use]
    pub const fn new(year: u16) -> Self {
        Self { year }
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
/// A user must belong to exactly one area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Area {
    /// The area identifier (e.g., "North", "South").
    id: String,
}

impl Area {
    /// Creates a new `Area`.
    ///
    /// Area identifiers are normalized to uppercase to ensure case-insensitive uniqueness.
    ///
    /// # Arguments
    ///
    /// * `id` - The area identifier (will be normalized to uppercase)
    #[must_use]
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_uppercase(),
        }
    }

    /// Returns the area identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
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
        }
    }
}
