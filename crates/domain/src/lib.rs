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

mod bid_year;
mod error;
mod leave_accrual;
mod leave_availability;
mod types;
mod validation;

#[cfg(test)]
mod tests;

// Re-export public types
pub use bid_year::{CanonicalBidYear, PayPeriod};
pub use error::DomainError;
pub use leave_accrual::{
    AccrualReason, LeaveAccrualResult, PayPeriodAccrual, calculate_leave_accrual,
};
pub use leave_availability::{LeaveAvailabilityResult, LeaveUsage, calculate_leave_availability};
pub use types::{
    Area, BidYear, BidYearLifecycle, Crew, Initials, Round, RoundGroup, SeniorityData, User,
    UserType,
};
pub use validation::{validate_bid_year, validate_initials_unique, validate_user_fields};
