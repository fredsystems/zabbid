// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::types::{BidYear, Initials};

/// Errors that can occur during domain validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// User initials are not unique within the bid year.
    DuplicateInitials {
        /// The bid year in which the duplicate was found.
        bid_year: BidYear,
        /// The duplicate initials.
        initials: Initials,
    },
    /// User initials are empty or invalid.
    InvalidInitials(String),
    /// User name is empty or invalid.
    InvalidName(String),
    /// Area identifier is empty or invalid.
    InvalidArea(String),
    /// Crew identifier is empty or invalid.
    InvalidCrew(&'static str),
    /// User type is invalid.
    InvalidUserType(String),
    /// Bid year does not exist.
    BidYearNotFound(u16),
    /// Area does not exist in the specified bid year.
    AreaNotFound {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
    },
    /// Bid year already exists.
    DuplicateBidYear(u16),
    /// Area already exists in the bid year.
    DuplicateArea {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
    },
    /// Invalid bid year value.
    InvalidBidYear(String),
    /// Invalid pay period count.
    InvalidPayPeriodCount {
        /// The invalid count value.
        count: u8,
    },
    /// Invalid pay period index.
    InvalidPayPeriodIndex {
        /// The invalid index.
        index: u8,
        /// The maximum valid index.
        max: u8,
    },
    /// Date arithmetic overflow.
    DateArithmeticOverflow {
        /// Description of the operation that failed.
        operation: String,
    },
    /// Bid year start date must be a Sunday.
    InvalidStartDateWeekday {
        /// The invalid start date.
        start_date: time::Date,
        /// The actual weekday.
        weekday: time::Weekday,
    },
    /// Bid year start date must be in January.
    InvalidStartDateMonth {
        /// The invalid start date.
        start_date: time::Date,
        /// The actual month.
        month: time::Month,
    },
    /// Service computation date is missing or invalid.
    InvalidServiceComputationDate {
        /// Description of the validation error.
        reason: String,
    },
    /// Failed to parse date from string.
    DateParseError {
        /// The invalid date string.
        date_string: String,
        /// The parsing error message.
        error: String,
    },
    /// User not found.
    UserNotFound {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
        /// The user's initials.
        initials: String,
    },
    /// Multiple bid years cannot be active simultaneously.
    MultipleBidYearsActive {
        /// The currently active year.
        current_active: u16,
        /// The year attempting to become active.
        requested_active: u16,
    },
    /// No active bid year is set.
    NoActiveBidYear,
    /// Expected area count must be positive.
    InvalidExpectedAreaCount {
        /// The invalid count value.
        count: u32,
    },
    /// Expected user count must be positive.
    InvalidExpectedUserCount {
        /// The invalid count value.
        count: u32,
    },
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateInitials { bid_year, initials } => {
                write!(
                    f,
                    "User with initials '{}' already exists in bid year {}",
                    initials.value(),
                    bid_year.year()
                )
            }
            Self::InvalidInitials(msg) => write!(f, "Invalid initials: {msg}"),
            Self::InvalidName(msg) => write!(f, "Invalid name: {msg}"),
            Self::InvalidArea(msg) => write!(f, "Invalid area: {msg}"),
            Self::InvalidCrew(msg) => write!(f, "Invalid crew: {msg}"),
            Self::InvalidUserType(msg) => write!(f, "Invalid user type: {msg}"),
            Self::BidYearNotFound(year) => write!(f, "Bid year {year} not found"),
            Self::AreaNotFound { bid_year, area } => {
                write!(f, "Area '{area}' not found in bid year {bid_year}")
            }
            Self::DuplicateBidYear(year) => write!(f, "Bid year {year} already exists"),
            Self::DuplicateArea { bid_year, area } => {
                write!(f, "Area '{area}' already exists in bid year {bid_year}")
            }
            Self::InvalidBidYear(msg) => write!(f, "Invalid bid year: {msg}"),
            Self::InvalidPayPeriodCount { count } => {
                write!(
                    f,
                    "Invalid pay period count: {count}. Must be exactly 26 or 27"
                )
            }
            Self::InvalidPayPeriodIndex { index, max } => {
                write!(
                    f,
                    "Invalid pay period index: {index}. Must be between 1 and {max}"
                )
            }
            Self::DateArithmeticOverflow { operation } => {
                write!(f, "Date arithmetic overflow while {operation}")
            }
            Self::InvalidStartDateWeekday {
                start_date,
                weekday,
            } => {
                write!(
                    f,
                    "Bid year start date must be a Sunday, but {start_date} is a {weekday}"
                )
            }
            Self::InvalidStartDateMonth { start_date, month } => {
                write!(
                    f,
                    "Bid year start date must be in January, but {start_date} is in {month}"
                )
            }
            Self::InvalidServiceComputationDate { reason } => {
                write!(f, "Invalid service computation date: {reason}")
            }
            Self::DateParseError { date_string, error } => {
                write!(f, "Failed to parse date '{date_string}': {error}")
            }
            Self::UserNotFound {
                bid_year,
                area,
                initials,
            } => {
                write!(
                    f,
                    "User with initials '{initials}' not found in area '{area}' for bid year {bid_year}"
                )
            }
            Self::MultipleBidYearsActive {
                current_active,
                requested_active,
            } => {
                write!(
                    f,
                    "Cannot set bid year {requested_active} as active: bid year {current_active} is already active"
                )
            }
            Self::NoActiveBidYear => {
                write!(f, "No active bid year is currently set")
            }
            Self::InvalidExpectedAreaCount { count } => {
                write!(
                    f,
                    "Invalid expected area count: {count}. Must be greater than 0"
                )
            }
            Self::InvalidExpectedUserCount { count } => {
                write!(
                    f,
                    "Invalid expected user count: {count}. Must be greater than 0"
                )
            }
        }
    }
}

impl std::error::Error for DomainError {}
