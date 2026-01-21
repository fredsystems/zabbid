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
    clippy::all,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::unwrap_used,
    clippy::expect_used
)]

mod bid_order;
mod bid_status;
mod bid_window;
mod bid_year;
mod error;
mod leave_accrual;
mod leave_availability;
mod readiness;
mod types;
mod validation;

#[cfg(test)]
mod tests;

pub use bid_order::{BidOrderPosition, SeniorityInputs, compute_bid_order};
pub use bid_status::{BidStatus, UserBidStatus};
pub use bid_window::{BidWindow, calculate_bid_windows};
pub use readiness::{
    count_participation_flag_violations, count_seniority_conflicts, count_unreviewed_no_bid_users,
    evaluate_area_readiness,
};

// Re-export public types
pub use bid_year::{CanonicalBidYear, PayPeriod};
pub use error::DomainError;
pub use leave_accrual::{
    AccrualReason, LeaveAccrualResult, PayPeriodAccrual, calculate_leave_accrual,
};
pub use leave_availability::{LeaveAvailabilityResult, LeaveUsage, calculate_leave_availability};
pub use types::{
    Area, BidSchedule, BidYear, BidYearLifecycle, BidYearReadiness, Crew, Initials,
    ReadinessDetails, Round, RoundGroup, SeniorityData, User, UserType,
};
pub use validation::{validate_bid_year, validate_initials_unique, validate_user_fields};
