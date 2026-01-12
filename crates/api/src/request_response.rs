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
    /// The operator password.
    pub password: String,
    /// The password confirmation.
    pub password_confirmation: String,
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

/// API request to change an operator's own password.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChangePasswordRequest {
    /// The current password.
    pub current_password: String,
    /// The new password.
    pub new_password: String,
    /// The new password confirmation.
    pub new_password_confirmation: String,
}

/// API response for successful password change.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChangePasswordResponse {
    /// Success message.
    pub message: String,
}

/// API request to reset another operator's password (admin only).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResetPasswordRequest {
    /// The operator ID whose password should be reset.
    pub operator_id: i64,
    /// The new password.
    pub new_password: String,
    /// The new password confirmation.
    pub new_password_confirmation: String,
}

/// API response for successful password reset.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResetPasswordResponse {
    /// Success message.
    pub message: String,
    /// The operator ID whose password was reset.
    pub operator_id: i64,
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
    /// The password confirmation.
    pub password_confirmation: String,
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

// ========================================================================
// Phase 18: Bootstrap Workflow Completion Request/Response Types
// ========================================================================

/// API request to set the active bid year.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetActiveBidYearRequest {
    /// The year to mark as active.
    pub year: u16,
}

/// API response for setting the active bid year.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetActiveBidYearResponse {
    /// The year that was set as active.
    pub year: u16,
    /// Success message.
    pub message: String,
}

/// API response for getting the active bid year.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GetActiveBidYearResponse {
    /// The currently active year, if any.
    pub year: Option<u16>,
}

/// API request to set the expected area count for the active bid year.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetExpectedAreaCountRequest {
    /// The expected number of areas.
    pub expected_count: u32,
}

/// API response for setting the expected area count.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetExpectedAreaCountResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The expected area count that was set.
    pub expected_count: u32,
    /// Success message.
    pub message: String,
}

/// API request to set the expected user count for an area in the active bid year.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetExpectedUserCountRequest {
    /// The area identifier.
    pub area: String,
    /// The expected number of users.
    pub expected_count: u32,
}

/// API response for setting the expected user count.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetExpectedUserCountResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The area identifier.
    pub area: String,
    /// The expected user count that was set.
    pub expected_count: u32,
    /// Success message.
    pub message: String,
}

/// API request to update an existing user in the active bid year.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UpdateUserRequest {
    /// The user's initials (identifier, cannot be changed).
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

/// API response for successful user update.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UpdateUserResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The user's initials.
    pub initials: String,
    /// The user's name.
    pub name: String,
    /// Success message.
    pub message: String,
}

/// Blocking reason for bootstrap incompleteness.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BlockingReason {
    /// No active bid year is set.
    NoActiveBidYear,
    /// Expected area count not set.
    ExpectedAreaCountNotSet {
        /// The bid year.
        bid_year: u16,
    },
    /// Actual area count does not match expected.
    AreaCountMismatch {
        /// The bid year.
        bid_year: u16,
        /// Expected count.
        expected: u32,
        /// Actual count.
        actual: usize,
    },
    /// Expected user count not set for an area.
    ExpectedUserCountNotSet {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
    },
    /// Actual user count does not match expected for an area.
    UserCountMismatch {
        /// The bid year.
        bid_year: u16,
        /// The area identifier.
        area: String,
        /// Expected count.
        expected: u32,
        /// Actual count.
        actual: usize,
    },
}

/// Completeness status for a bid year.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BidYearCompletenessInfo {
    /// The bid year.
    pub year: u16,
    /// Whether this bid year is active.
    pub is_active: bool,
    /// Expected area count, if set.
    pub expected_area_count: Option<u32>,
    /// Actual area count.
    pub actual_area_count: usize,
    /// Whether the bid year is complete.
    pub is_complete: bool,
    /// Blocking reasons preventing completeness.
    pub blocking_reasons: Vec<BlockingReason>,
}

/// Completeness status for an area.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AreaCompletenessInfo {
    /// The bid year.
    pub bid_year: u16,
    /// The area identifier.
    pub area: String,
    /// Expected user count, if set.
    pub expected_user_count: Option<u32>,
    /// Actual user count.
    pub actual_user_count: usize,
    /// Whether the area is complete.
    pub is_complete: bool,
    /// Blocking reasons preventing completeness.
    pub blocking_reasons: Vec<BlockingReason>,
}

/// API response for bootstrap completeness status.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GetBootstrapCompletenessResponse {
    /// The currently active bid year, if any.
    pub active_bid_year: Option<u16>,
    /// Completeness information for all bid years.
    pub bid_years: Vec<BidYearCompletenessInfo>,
    /// Completeness information for all areas.
    pub areas: Vec<AreaCompletenessInfo>,
    /// Whether the system is ready for bidding.
    pub is_ready_for_bidding: bool,
    /// Top-level blocking reasons.
    pub blocking_reasons: Vec<BlockingReason>,
}

/// A single row from a CSV import preview.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CsvUserRow {
    /// The row index (0-based).
    pub row_index: usize,
    /// The user's initials.
    pub initials: String,
    /// The user's name.
    pub name: String,
    /// The user's area identifier.
    pub area: String,
    /// The user's type classification.
    pub user_type: String,
    /// The user's crew number (optional).
    pub crew: Option<u8>,
    /// Cumulative NATCA bargaining unit date.
    pub cumulative_natca_bu_date: String,
    /// NATCA bargaining unit date.
    pub natca_bu_date: String,
    /// Entry on Duty / FAA date.
    pub eod_faa_date: String,
    /// Service Computation Date.
    pub service_computation_date: String,
    /// Optional lottery value.
    pub lottery_value: Option<u32>,
    /// Whether this row is valid.
    pub is_valid: bool,
    /// Validation error message, if invalid.
    pub validation_error: Option<String>,
}

/// API request to preview CSV user data.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PreviewCsvRequest {
    /// The bid year these users will be imported into.
    pub bid_year: u16,
    /// The CSV content as a string.
    pub csv_content: String,
}

/// API response for CSV preview.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PreviewCsvResponse {
    /// The bid year.
    pub bid_year: u16,
    /// Parsed rows with validation status.
    pub rows: Vec<CsvUserRow>,
    /// Total number of rows.
    pub total_rows: usize,
    /// Number of valid rows.
    pub valid_rows: usize,
    /// Number of invalid rows.
    pub invalid_rows: usize,
}

/// API request to import selected CSV rows.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ImportSelectedUsersRequest {
    /// The bid year.
    pub bid_year: u16,
    /// The row indices to import (0-based).
    pub selected_rows: Vec<usize>,
    /// The CSV content (same as in preview).
    pub csv_content: String,
}

/// Result of importing a single user.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserImportResult {
    /// The row index (0-based).
    pub row_index: usize,
    /// The user's initials.
    pub initials: String,
    /// Whether the import succeeded.
    pub success: bool,
    /// Error message if the import failed.
    pub error: Option<String>,
}

/// API response for selective CSV import.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ImportSelectedUsersResponse {
    /// The bid year.
    pub bid_year: u16,
    /// Import results for each selected row.
    pub results: Vec<UserImportResult>,
    /// Total number of rows attempted.
    pub total_attempted: usize,
    /// Number of successful imports.
    pub successful: usize,
    /// Number of failed imports.
    pub failed: usize,
}

// ========================================================================
// CSV Preview Types
// ========================================================================

/// API request to preview CSV user data for the active bid year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewCsvUsersRequest {
    /// The raw CSV content.
    pub csv_content: String,
}

/// Status of a single CSV row validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CsvRowStatus {
    /// Row is valid and can be imported.
    Valid,
    /// Row has validation errors and cannot be imported.
    Invalid,
}

/// Result for a single CSV row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CsvRowPreview {
    /// The row number (1-based, excluding header).
    pub row_number: usize,
    /// The parsed initials (if valid).
    pub initials: Option<String>,
    /// The parsed name (if valid).
    pub name: Option<String>,
    /// The parsed area ID (if valid).
    pub area_id: Option<String>,
    /// The parsed user type (if valid).
    pub user_type: Option<String>,
    /// The parsed crew (if valid).
    pub crew: Option<u8>,
    /// The row validation status.
    pub status: CsvRowStatus,
    /// Zero or more validation error messages.
    pub errors: Vec<String>,
}

/// API response for CSV preview.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PreviewCsvUsersResponse {
    /// The bid year being validated against.
    pub bid_year: u16,
    /// Per-row validation results.
    pub rows: Vec<CsvRowPreview>,
    /// Total number of rows.
    pub total_rows: usize,
    /// Number of valid rows.
    pub valid_count: usize,
    /// Number of invalid rows.
    pub invalid_count: usize,
}

/// API request to import selected CSV rows into the active bid year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportCsvUsersRequest {
    /// The raw CSV content (same as preview).
    pub csv_content: String,
    /// The row indices (0-based, excluding header) to import.
    pub selected_row_indices: Vec<usize>,
}

/// Result of a single row import attempt.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CsvImportRowResult {
    /// The row index (0-based, excluding header).
    pub row_index: usize,
    /// The row number (1-based, for human display).
    pub row_number: usize,
    /// The initials from this row (if parsed).
    pub initials: Option<String>,
    /// The status of this import attempt.
    pub status: CsvImportRowStatus,
    /// Error message if the import failed.
    pub error: Option<String>,
}

/// Status of a single CSV row import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CsvImportRowStatus {
    /// Row was successfully imported.
    Success,
    /// Row import failed.
    Failed,
}

/// API response for CSV import.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ImportCsvUsersResponse {
    /// The bid year imported into.
    pub bid_year: u16,
    /// Total number of rows selected for import.
    pub total_selected: usize,
    /// Number of rows successfully imported.
    pub successful_count: usize,
    /// Number of rows that failed to import.
    pub failed_count: usize,
    /// Per-row import results.
    pub results: Vec<CsvImportRowResult>,
}
