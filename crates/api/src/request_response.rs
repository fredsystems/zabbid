// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! API request and response data transfer objects.

use time::Date;

/// API request to create a new bid year with canonical metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBidYearRequest {
    /// The year value (e.g., 2026).
    pub year: u16,
    /// The start date of the bid year.
    pub start_date: Date,
    /// The number of pay periods (must be 26 or 27).
    pub num_pay_periods: u8,
}

/// API response for a successful bid year creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBidYearResponse {
    /// The created bid year.
    pub year: u16,
    /// The start date of the bid year.
    pub start_date: Date,
    /// The number of pay periods.
    pub num_pay_periods: u8,
    /// The derived end date of the bid year (inclusive).
    pub end_date: Date,
    /// A success message.
    pub message: String,
}

/// API request to create a new area within a bid year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAreaRequest {
    /// The bid year this area belongs to.
    pub bid_year: u16,
    /// The area identifier.
    pub area_id: String,
}

/// API response for a successful area creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAreaResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The area identifier.
    pub area_id: String,
    /// A success message.
    pub message: String,
}

/// API request to register a new user for a bid year.
///
/// This DTO is distinct from domain types and represents the API contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterUserRequest {
    /// The bid year (e.g., 2026).
    pub bid_year: u16,
    /// The user's initials.
    pub initials: String,
    /// The user's name.
    pub name: String,
    /// The user's area identifier.
    pub area: String,
    /// The user's type classification (CPC, CPC-IT, Dev-R, Dev-D).
    pub user_type: String,
    /// The user's crew number (1-7, optional).
    pub crew: Option<u8>,
    /// Cumulative NATCA bargaining unit date (ISO 8601).
    pub cumulative_natca_bu_date: String,
    /// NATCA bargaining unit date (ISO 8601).
    pub natca_bu_date: String,
    /// Entry on Duty / FAA date (ISO 8601).
    pub eod_faa_date: String,
    /// Service Computation Date (ISO 8601).
    pub service_computation_date: String,
    /// Optional lottery value.
    pub lottery_value: Option<u32>,
}

/// API response for a successful user registration.
///
/// This DTO is distinct from domain types and represents the API contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterUserResponse {
    /// The bid year the user was registered for.
    pub bid_year: u16,
    /// The user's initials.
    pub initials: String,
    /// The user's name.
    pub name: String,
    /// A success message.
    pub message: String,
}

/// Canonical bid year information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidYearInfo {
    /// The year value.
    pub year: u16,
    /// The start date of the bid year.
    pub start_date: Date,
    /// The number of pay periods.
    pub num_pay_periods: u8,
    /// The derived end date of the bid year (inclusive).
    pub end_date: Date,
    /// The number of areas in this bid year.
    pub area_count: usize,
    /// The total number of users across all areas in this bid year.
    pub total_user_count: usize,
}

/// API response for listing bid years.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListBidYearsResponse {
    /// The list of bid years with canonical metadata.
    pub bid_years: Vec<BidYearInfo>,
}

/// API request to list areas for a bid year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAreasRequest {
    /// The bid year to list areas for.
    pub bid_year: u16,
}

/// Information about a single area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaInfo {
    /// The area identifier.
    pub area_id: String,
    /// The number of users in this area.
    pub user_count: usize,
}

/// API response for listing areas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAreasResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The list of areas with metadata.
    pub areas: Vec<AreaInfo>,
}

/// API request to list users for a bid year and area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListUsersRequest {
    /// The bid year.
    pub bid_year: u16,
    /// The area identifier.
    pub area: String,
}

/// API response for listing users.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListUsersResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The area identifier.
    pub area: String,
    /// The list of users.
    pub users: Vec<UserInfo>,
}

/// User information for listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserInfo {
    /// The user's initials.
    pub initials: String,
    /// The user's name.
    pub name: String,
    /// The user's crew (optional).
    pub crew: Option<u8>,
    /// The user's type classification (CPC, CPC-IT, Dev-R, Dev-D).
    pub user_type: String,
    /// Total hours earned (from Phase 9, post-rounding).
    pub earned_hours: u16,
    /// Total days earned.
    pub earned_days: u16,
    /// Remaining hours available (may be negative if overdrawn).
    pub remaining_hours: i32,
    /// Remaining days available (may be negative if overdrawn).
    pub remaining_days: i32,
    /// Whether all leave has been exhausted.
    pub is_exhausted: bool,
    /// Whether leave balance is overdrawn.
    pub is_overdrawn: bool,
}

/// Bootstrap status summary for a single bid year.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BidYearStatusInfo {
    /// The year value.
    pub year: u16,
    /// The number of areas in this bid year.
    pub area_count: usize,
    /// The total number of users across all areas.
    pub total_user_count: usize,
}

/// Area summary for bootstrap status.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AreaStatusInfo {
    /// The bid year this area belongs to.
    pub bid_year: u16,
    /// The area identifier.
    pub area_id: String,
    /// The number of users in this area.
    pub user_count: usize,
}

/// API response for bootstrap status.
///
/// Provides a comprehensive summary of the system state for operators.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BootstrapStatusResponse {
    /// Summary of all bid years with counts.
    pub bid_years: Vec<BidYearStatusInfo>,
    /// Summary of all areas with counts.
    pub areas: Vec<AreaStatusInfo>,
}

/// API request to get leave availability for a user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetLeaveAvailabilityRequest {
    /// The bid year.
    pub bid_year: u16,
    /// The area identifier.
    pub area: String,
    /// The user's initials.
    pub initials: String,
}

/// API response for leave availability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetLeaveAvailabilityResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The user's initials.
    pub initials: String,
    /// Total hours earned (from Phase 9, post-rounding).
    pub earned_hours: u16,
    /// Total days earned.
    pub earned_days: u16,
    /// Total hours used.
    pub used_hours: u16,
    /// Remaining hours available (may be negative if overdrawn).
    pub remaining_hours: i32,
    /// Remaining days available (may be negative if overdrawn).
    pub remaining_days: i32,
    /// Whether all leave has been exhausted.
    pub is_exhausted: bool,
    /// Whether leave balance is overdrawn.
    pub is_overdrawn: bool,
    /// Human-readable explanation of the calculation.
    pub explanation: String,
}

// ========================================================================
// Authentication Request/Response Types (Phase 14)
// ========================================================================

/// API request to log in and create a session.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LoginRequest {
    /// The operator login name.
    pub login_name: String,
    /// The operator password.
    pub password: String,
}

/// API response for successful login.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LoginResponse {
    /// The session token (opaque).
    pub session_token: String,
    /// The operator's login name.
    pub login_name: String,
    /// The operator's display name.
    pub display_name: String,
    /// The operator's role.
    pub role: String,
    /// Session expiration timestamp (ISO 8601).
    pub expires_at: String,
}

/// API response for the "who am I" endpoint.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WhoAmIResponse {
    /// The operator's login name.
    pub login_name: String,
    /// The operator's display name.
    pub display_name: String,
    /// The operator's role.
    pub role: String,
    /// Whether the operator is disabled.
    pub is_disabled: bool,
}

/// API request to create a new operator.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CreateOperatorRequest {
    /// The operator login name.
    pub login_name: String,
    /// The operator display name.
    pub display_name: String,
    /// The operator role (Admin or Bidder).
    pub role: String,
}

/// API response for successful operator creation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CreateOperatorResponse {
    /// The operator ID.
    pub operator_id: i64,
    /// The operator login name.
    pub login_name: String,
    /// The operator display name.
    pub display_name: String,
    /// The operator role.
    pub role: String,
}

/// Operator information for listing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OperatorInfo {
    /// The operator ID.
    pub operator_id: i64,
    /// The operator login name.
    pub login_name: String,
    /// The operator display name.
    pub display_name: String,
    /// The operator role.
    pub role: String,
    /// Whether the operator is disabled.
    pub is_disabled: bool,
    /// Created timestamp (ISO 8601).
    pub created_at: String,
    /// Last login timestamp (ISO 8601, optional).
    pub last_login_at: Option<String>,
}

/// API response for listing operators.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ListOperatorsResponse {
    /// The list of operators.
    pub operators: Vec<OperatorInfo>,
}

/// API request for disabling an operator.
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DisableOperatorRequest {
    /// The operator ID to disable.
    pub operator_id: i64,
}

/// API response for disabling an operator.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DisableOperatorResponse {
    /// Confirmation message.
    pub message: String,
}

/// API request for re-enabling an operator.
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnableOperatorRequest {
    /// The operator ID to re-enable.
    pub operator_id: i64,
}

/// API response for re-enabling an operator.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnableOperatorResponse {
    /// Confirmation message.
    pub message: String,
}

/// API request for deleting an operator.
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeleteOperatorRequest {
    /// The operator ID to delete.
    pub operator_id: i64,
}

/// API response for deleting an operator.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeleteOperatorResponse {
    /// Confirmation message.
    pub message: String,
}

/// API response for checking bootstrap status.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BootstrapAuthStatusResponse {
    /// Whether the system is in bootstrap mode (no operators exist).
    pub is_bootstrap_mode: bool,
}

/// API request for bootstrap login with hardcoded credentials.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BootstrapLoginRequest {
    /// Username (must be "admin" in bootstrap mode).
    pub username: String,
    /// Password (must be "admin" in bootstrap mode).
    pub password: String,
}

/// API response for successful bootstrap login.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BootstrapLoginResponse {
    /// Bootstrap session token (temporary, not a real operator session).
    pub bootstrap_token: String,
    /// Indicates this is a bootstrap session.
    pub is_bootstrap: bool,
}

/// API request to create the first admin operator during bootstrap.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CreateFirstAdminRequest {
    /// The new admin login name.
    pub login_name: String,
    /// The new admin display name.
    pub display_name: String,
    /// The password for the new admin.
    pub password: String,
}

/// API response for successful first admin creation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CreateFirstAdminResponse {
    /// The operator ID.
    pub operator_id: i64,
    /// The operator login name.
    pub login_name: String,
    /// The operator display name.
    pub display_name: String,
    /// Success message.
    pub message: String,
}
