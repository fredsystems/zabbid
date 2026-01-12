// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * TypeScript types matching the backend API contracts.
 * These types are derived from the Rust API DTOs and must remain in sync.
 */

/**
 * Bid year information with canonical metadata and aggregate counts.
 */
export interface BidYearInfo {
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
}

/**
 * Response for listing all bid years.
 */
export interface ListBidYearsResponse {
  bid_years: BidYearInfo[];
}

/**
 * Information about a single area.
 */
export interface AreaInfo {
  /** The area identifier */
  area_id: string;
  /** The number of users in this area */
  user_count: number;
}

/**
 * Response for listing areas in a bid year.
 */
export interface ListAreasResponse {
  /** The bid year */
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
  /** The user's initials */
  initials: string;
  /** The user's name */
  name: string;
  /** The user's crew (1-7, optional) */
  crew: number | null;
  /** The user's type classification (CPC, CPC-IT, Dev-R, Dev-D) */
  user_type: string;
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
}

/**
 * Response for listing users in an area.
 */
export interface ListUsersResponse {
  /** The bid year */
  bid_year: number;
  /** The area identifier */
  area: string;
  /** The list of users with leave information */
  users: UserInfo[];
}

/**
 * Detailed leave availability information for a specific user.
 */
export interface LeaveAvailabilityResponse {
  /** The bid year */
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
  /** The bid year this area belongs to */
  bid_year: number;
  /** The area identifier */
  area_id: string;
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
  | { ExpectedAreaCountNotSet: { bid_year: number } }
  | {
      AreaCountMismatch: {
        bid_year: number;
        expected: number;
        actual: number;
      };
    }
  | { ExpectedUserCountNotSet: { bid_year: number; area: string } }
  | {
      UserCountMismatch: {
        bid_year: number;
        area: string;
        expected: number;
        actual: number;
      };
    };

/**
 * Completeness status for a bid year.
 */
export interface BidYearCompletenessInfo {
  /** The bid year */
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
}

/**
 * Completeness status for an area.
 */
export interface AreaCompletenessInfo {
  /** The bid year */
  bid_year: number;
  /** The area identifier */
  area: string;
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
  /** The currently active bid year, if any */
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
  /** The currently active bid year, if any */
  active_bid_year: number | null;
}

/**
 * Response for setting the active bid year.
 */
export interface SetActiveBidYearResponse {
  /** The year that was set as active */
  year: number;
  /** Success message */
  message: string;
}

/**
 * Response for setting expected area count.
 */
export interface SetExpectedAreaCountResponse {
  /** The bid year */
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
  /** The bid year */
  bid_year: number;
  /** The area identifier */
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
  /** The bid year */
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
  /** The bid year being validated against */
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
  /** The bid year imported into */
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
