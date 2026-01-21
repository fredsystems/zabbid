// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * TypeScript types matching the backend API contracts.
 * These types are derived from the Rust API DTOs and must remain in sync.
 */

/**
 * Global capabilities for an authenticated operator.
 */
export interface GlobalCapabilities {
  can_create_operator: boolean;
  can_create_bid_year: boolean;
  can_create_area: boolean;
  can_create_user: boolean;
  can_modify_users: boolean;
  can_bootstrap: boolean;
}

/**
 * Target-specific capabilities for an operator instance.
 */
export interface OperatorCapabilities {
  can_disable: boolean;
  can_delete: boolean;
}

/**
 * Target-specific capabilities for a user instance.
 */
export interface UserCapabilities {
  can_delete: boolean;
  can_move_area: boolean;
  can_edit_seniority: boolean;
}

/**
 * Response for registering a new user.
 * Note: user_id and bid_year_id are populated by the server after persistence.
 */
export interface RegisterUserResponse {
  /** The canonical bid year identifier (populated after persistence) */
  bid_year_id: number | null;
  /** The bid year the user was registered for (display value) */
  bid_year: number;
  /** The user's canonical identifier (populated after persistence) */
  user_id: number | null;
  /** The user's initials */
  initials: string;
  /** The user's name */
  name: string;
  /** Success message */
  message: string;
}

/**
 * Bid year information with canonical metadata and aggregate counts.
 */
export interface BidYearInfo {
  /** The canonical numeric identifier */
  bid_year_id: number;
  /** The year value (e.g., 2026) */
  year: number;
  /** The start date of the bid year (ISO 8601) */
  start_date: string;
  /** The number of pay periods (26 or 27) */
  num_pay_periods: number;
  /** The derived end date of the bid year (ISO 8601, inclusive) */
  end_date: string;
  /** The number of areas in this bid year */
  area_count: number;
  /** The total number of users across all areas in this bid year */
  total_user_count: number;
  /** The lifecycle state of the bid year */
  lifecycle_state: string;
}

/**
 * Response for listing all bid years.
 */
export interface ListBidYearsResponse {
  bid_years: BidYearInfo[];
}

/**
 * Response for creating a new bid year.
 */
export interface CreateBidYearResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The created bid year (display value) */
  year: number;
  /** The start date of the bid year */
  start_date: string;
  /** The number of pay periods */
  num_pay_periods: number;
  /** The derived end date of the bid year (inclusive) */
  end_date: string;
  /** Success message */
  message: string;
}
export interface CreateAreaResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year (display value) */
  bid_year: number;
  /** The canonical area identifier */
  area_id: number;
  /** The area code (display value) */
  area_code: string;
  /** A success message */
  message: string;
}

/**
 * Response for updating area metadata.
 */
export interface UpdateAreaResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year (display value) */
  bid_year: number;
  /** The canonical area identifier */
  area_id: number;
  /** The area code (immutable) */
  area_code: string;
  /** The updated display name */
  area_name: string | null;
  /** Success message */
  message: string;
}

export interface AssignAreaRoundGroupResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year (display value) */
  bid_year: number;
  /** The canonical area identifier */
  area_id: number;
  /** The area code (immutable) */
  area_code: string;
  /** The assigned round group ID (or null if cleared) */
  round_group_id: number | null;
  /** Success message */
  message: string;
}
/**
 * Information about a single area.
 */
export interface AreaInfo {
  /** The canonical area identifier */
  area_id: number;
  /** The area code (display value) */
  area_code: string;
  /** The area name (optional) */
  area_name: string | null;
  /** The number of users in this area */
  user_count: number;
  /** Whether this is a system-managed area (e.g., "No Bid") */
  is_system_area: boolean;
}

/**
 * Response for listing areas in a bid year.
 */
export interface ListAreasResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year (display value) */
  bid_year: number;
  /** The list of areas with metadata */
  areas: AreaInfo[];
}

/**
 * User information for listing with leave availability.
 */
export interface UserInfo {
  /** The user's canonical internal identifier */
  user_id: number;
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The canonical area identifier */
  area_id: number;
  /** The user's initials */
  initials: string;
  /** The user's name */
  name: string;
  /** The user's crew (1-7, optional) */
  crew: number | null;
  /** The user's type classification (CPC, CPC-IT, Dev-R, Dev-D) */
  user_type: string;
  /** Cumulative NATCA bargaining unit date (ISO 8601 date string) */
  cumulative_natca_bu_date: string;
  /** NATCA bargaining unit date (ISO 8601 date string) */
  natca_bu_date: string;
  /** Entry on Duty / FAA date (ISO 8601 date string) */
  eod_faa_date: string;
  /** Service Computation Date (ISO 8601 date string) */
  service_computation_date: string;
  /** Optional lottery value for tie-breaking */
  lottery_value: number | null;
  /** Total hours earned (from Phase 9, post-rounding) */
  earned_hours: number;
  /** Total days earned */
  earned_days: number;
  /** Remaining hours available (may be negative if overdrawn) */
  remaining_hours: number;
  /** Remaining days available (may be negative if overdrawn) */
  remaining_days: number;
  /** Whether all leave has been exhausted */
  is_exhausted: boolean;
  /** Whether leave balance is overdrawn */
  is_overdrawn: boolean;
  /** Target-specific capabilities for this user instance */
  capabilities: UserCapabilities;
}

/**
 * Response for listing users in an area.
 */
export interface ListUsersResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year (display value) */
  bid_year: number;
  /** The canonical area identifier */
  area_id: number;
  /** The area code (display value) */
  area_code: string;
  /** The list of users with leave information */
  users: UserInfo[];
}

/**
 * Detailed leave availability information for a specific user.
 */
export interface LeaveAvailabilityResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year (display value) */
  bid_year: number;
  /** The user's canonical internal identifier */
  user_id: number;
  /** The user's initials */
  initials: string;
  /** Total hours earned (from Phase 9, post-rounding) */
  earned_hours: number;
  /** Total days earned */
  earned_days: number;
  /** Total hours used */
  used_hours: number;
  /** Remaining hours available (may be negative if overdrawn) */
  remaining_hours: number;
  /** Remaining days available (may be negative if overdrawn) */
  remaining_days: number;
  /** Whether all leave has been exhausted */
  is_exhausted: boolean;
  /** Whether leave balance is overdrawn */
  is_overdrawn: boolean;
  /** Human-readable explanation of the calculation */
  explanation: string;
}

/**
 * Bootstrap status for a single bid year.
 */
export interface BidYearStatusInfo {
  /** The canonical numeric identifier */
  bid_year_id: number;
  /** The year value */
  year: number;
  /** The number of areas in this bid year */
  area_count: number;
  /** The total number of users across all areas */
  total_user_count: number;
}

/**
 * Area summary for bootstrap status.
 */
export interface AreaStatusInfo {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year this area belongs to (display value) */
  bid_year: number;
  /** The canonical area identifier */
  area_id: number;
  /** The area code (display value) */
  area_code: string;
  /** The number of users in this area */
  user_count: number;
}

/**
 * Bootstrap status response providing system-wide summary.
 */
export interface BootstrapStatusResponse {
  /** Summary of all bid years with counts */
  bid_years: BidYearStatusInfo[];
  /** Summary of all areas with counts */
  areas: AreaStatusInfo[];
}

/**
 * Error response from the API.
 */
export interface ErrorResponse {
  error: string;
  message: string;
}

/**
 * Live event types for WebSocket streaming.
 * These represent read-only state change notifications from the backend.
 */
export type LiveEvent =
  | { type: "bid_year_created"; year: number }
  | { type: "area_created"; bid_year: number; area: string }
  | {
      type: "user_registered";
      bid_year: number;
      area: string;
      initials: string;
    }
  | {
      type: "user_updated";
      bid_year: number;
      area: string;
      initials: string;
    }
  | { type: "checkpoint_created"; bid_year: number; area: string }
  | { type: "rolled_back"; bid_year: number; area: string }
  | { type: "round_finalized"; bid_year: number; area: string }
  | { type: "connected"; timestamp: string };

/**
 * Connection state for backend connectivity.
 */
export type ConnectionState =
  | "connecting"
  | "connected"
  | "disconnected"
  | "error";

/**
 * Blocking reason for bootstrap incompleteness.
 * Matches Rust serde's default externally-tagged enum serialization.
 */
export type BlockingReason =
  | "NoActiveBidYear"
  | {
      ExpectedAreaCountNotSet: {
        bid_year_id: number;
        bid_year: number;
      };
    }
  | {
      AreaCountMismatch: {
        bid_year_id: number;
        bid_year: number;
        expected: number;
        actual: number;
      };
    }
  | {
      ExpectedUserCountNotSet: {
        bid_year_id: number;
        bid_year: number;
        area_id: number;
        area_code: string;
      };
    }
  | {
      UserCountMismatch: {
        bid_year_id: number;
        bid_year: number;
        area_id: number;
        area_code: string;
        expected: number;
        actual: number;
      };
    }
  | {
      UsersInNoBidArea: {
        bid_year_id: number;
        bid_year: number;
        user_count: number;
        sample_initials: string[];
      };
    };

/**
 * Completeness status for a bid year.
 */
export interface BidYearCompletenessInfo {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year (display value) */
  year: number;
  /** Whether this bid year is active */
  is_active: boolean;
  /** Expected area count, if set */
  expected_area_count: number | null;
  /** Actual area count */
  actual_area_count: number;
  /** Whether the bid year is complete */
  is_complete: boolean;
  /** Blocking reasons preventing completeness */
  blocking_reasons: BlockingReason[];
  /** The lifecycle state of the bid year */
  lifecycle_state: string;
}

/**
 * Completeness status for an area.
 */
export interface AreaCompletenessInfo {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year (display value) */
  bid_year: number;
  /** The canonical area identifier */
  area_id: number;
  /** The area code (display value) */
  area_code: string;
  /** Expected user count, if set */
  expected_user_count: number | null;
  /** Actual user count */
  actual_user_count: number;
  /** Whether the area is complete */
  is_complete: boolean;
  /** Blocking reasons preventing completeness */
  blocking_reasons: BlockingReason[];
}

/**
 * Bootstrap completeness response.
 */
export interface GetBootstrapCompletenessResponse {
  /** The canonical ID of the currently active bid year, if any */
  active_bid_year_id: number | null;
  /** The currently active bid year (display value), if any */
  active_bid_year: number | null;
  /** Completeness information for all bid years */
  bid_years: BidYearCompletenessInfo[];
  /** Completeness information for all areas */
  areas: AreaCompletenessInfo[];
  /** Whether the system is ready for bidding */
  is_ready_for_bidding: boolean;
  /** Top-level blocking reasons */
  blocking_reasons: BlockingReason[];
}

/**
 * Response for getting the active bid year.
 */
export interface GetActiveBidYearResponse {
  /** The canonical ID of the currently active bid year, if any */
  active_bid_year_id: number | null;
  /** The currently active bid year (display value), if any */
  active_bid_year: number | null;
}

/**
 * Response for setting the active bid year.
 */
export interface SetActiveBidYearResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The year that was set as active (display value) */
  year: number;
  /** Success message */
  message: string;
}

/**
 * Response for setting expected area count.
 */
export interface SetExpectedAreaCountResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year (display value) */
  bid_year: number;
  /** The expected area count that was set */
  expected_count: number;
  /** Success message */
  message: string;
}

/**
 * Response for setting expected user count.
 */
export interface SetExpectedUserCountResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year (display value) */
  bid_year: number;
  /** The canonical area identifier */
  area_id: number;
  /** The area code (display value) */
  area: string;
  /** The expected user count that was set */
  expected_count: number;
  /** Success message */
  message: string;
}

/**
 * Response for updating a user.
 */
export interface UpdateUserResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year (display value) */
  bid_year: number;
  /** The user's canonical internal identifier */
  user_id: number;
  /** The user's initials */
  initials: string;
  /** The user's name */
  name: string;
  /** Success message */
  message: string;
}

/**
 * Response for overriding a user's area assignment.
 */
export interface OverrideAreaAssignmentResponse {
  /** The audit event ID */
  audit_event_id: number;
  /** Success message */
  message: string;
}

/**
 * Status of a single CSV row validation.
 */
export type CsvRowStatus = "valid" | "invalid";

/**
 * Result for a single CSV row preview.
 */
export interface CsvRowPreview {
  /** The row number (1-based, excluding header) */
  row_number: number;
  /** The parsed initials (if valid) */
  initials: string | null;
  /** The parsed name (if valid) */
  name: string | null;
  /** The parsed area ID (if valid) */
  area_id: string | null;
  /** The parsed user type (if valid) */
  user_type: string | null;
  /** The parsed crew (if valid) */
  crew: number | null;
  /** The row validation status */
  status: CsvRowStatus;
  /** Zero or more validation error messages */
  errors: string[];
}

/**
 * Response for CSV preview.
 */
export interface PreviewCsvUsersResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year being validated against (display value) */
  bid_year: number;
  /** Per-row validation results */
  rows: CsvRowPreview[];
  /** Total number of rows */
  total_rows: number;
  /** Number of valid rows */
  valid_count: number;
  /** Number of invalid rows */
  invalid_count: number;
}

/**
 * Status of a single CSV row import.
 */
export type CsvImportRowStatus = "success" | "failed";

/**
 * Information about an operator.
 */
export interface OperatorInfo {
  /** The operator's internal identifier */
  operator_id: number;
  /** The operator's login name */
  login_name: string;
  /** The operator's display name */
  display_name: string;
  /** The operator's role (Admin or Bidder) */
  role: string;
  /** Whether the operator is disabled */
  is_disabled: boolean;
  /** When the operator was created */
  created_at: string;
  /** When the operator last logged in */
  last_login_at: string | null;
  /** Target-specific capabilities for this operator instance */
  capabilities: OperatorCapabilities;
}

/**
 * Response for the whoami endpoint.
 */
export interface WhoAmIResponse {
  /** The operator's login name */
  login_name: string;
  /** The operator's display name */
  display_name: string;
  /** The operator's role */
  role: string;
  /** Whether the operator is disabled */
  is_disabled: boolean;
  /** Global capabilities for this operator */
  capabilities: GlobalCapabilities;
}

/**
 * Result of a single row import attempt.
 */
export interface CsvImportRowResult {
  /** The row index (0-based, excluding header) */
  row_index: number;
  /** The row number (1-based, for human display) */
  row_number: number;
  /** The initials from this row (if parsed) */
  initials: string | null;
  /** The status of this import attempt */
  status: CsvImportRowStatus;
  /** Error message if the import failed */
  error: string | null;
}

/**
 * Response for CSV import.
 */
export interface ImportCsvUsersResponse {
  /** The canonical bid year identifier */
  bid_year_id: number;
  /** The bid year imported into (display value) */
  bid_year: number;
  /** Total number of rows selected for import */
  total_selected: number;
  /** Number of rows successfully imported */
  successful_count: number;
  /** Number of rows that failed to import */
  failed_count: number;
  /** Per-row import results */
  results: CsvImportRowResult[];
}
