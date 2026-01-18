// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * API client for communicating with the zabbid backend.
 * All API calls go through the /api proxy configured in Vite.
 */

import type {
  BidYearInfo,
  BootstrapStatusResponse,
  CreateAreaResponse,
  CreateBidYearResponse,
  ErrorResponse,
  GetActiveBidYearResponse,
  GetBootstrapCompletenessResponse,
  ImportCsvUsersResponse,
  LeaveAvailabilityResponse,
  ListAreasResponse,
  ListBidYearsResponse,
  ListUsersResponse,
  OperatorInfo,
  OverrideAreaAssignmentResponse,
  PreviewCsvUsersResponse,
  RegisterUserResponse,
  SetActiveBidYearResponse,
  SetExpectedAreaCountResponse,
  SetExpectedUserCountResponse,
  UpdateUserResponse,
  WhoAmIResponse,
} from "./types";

const API_BASE = "/api";

/**
 * API error class for structured error handling.
 */
export class ApiError extends Error {
  constructor(
    message: string,
    public readonly status: number,
    public readonly error?: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

/**
 * Network error class for connection failures.
 * Indicates the backend is unreachable, not that it returned an error.
 */
export class NetworkError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "NetworkError";
  }
}

/**
 * Wrapper around fetch that handles JSON parsing and error responses.
 * Distinguishes between network failures (backend unreachable) and HTTP errors (backend responded with an error).
 */
async function fetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  let response: Response;

  try {
    response = await fetch(url, init);
  } catch (_error) {
    // Network error: backend is unreachable
    // This includes DNS failures, connection refused, timeouts, etc.
    throw new NetworkError(
      "Unable to connect to backend. Please ensure the server is running.",
    );
  }

  if (!response.ok) {
    // Try to parse the response body to distinguish between proxy errors and backend errors
    let errorMessage = `HTTP ${response.status}: ${response.statusText}`;
    let errorType: string | undefined;
    let isProxyError = false;

    try {
      const errorData = (await response.json()) as ErrorResponse;
      errorMessage = errorData.message || errorMessage;
      errorType = errorData.error;
    } catch {
      // If we can't parse JSON, it's likely a proxy error (backend unreachable)
      // Vite proxy returns plain text or HTML when it can't reach the backend
      isProxyError = true;
    }

    // Treat proxy errors as network errors (backend unreachable)
    // 502 Bad Gateway, 503 Service Unavailable, 504 Gateway Timeout
    // 500 that couldn't be parsed as JSON (from proxy when backend is down)
    if (
      response.status === 502 ||
      response.status === 503 ||
      response.status === 504 ||
      (response.status === 500 && isProxyError)
    ) {
      throw new NetworkError(
        "Unable to connect to backend. Please ensure the server is running.",
      );
    }

    throw new ApiError(errorMessage, response.status, errorType);
  }

  return response.json() as Promise<T>;
}

/**
 * List all bid years with canonical metadata and aggregate counts.
 */
export async function listBidYears(): Promise<BidYearInfo[]> {
  const response = await fetchJson<ListBidYearsResponse>(
    `${API_BASE}/bid_years`,
  );
  return response.bid_years;
}

/**
 * List all areas for a specific bid year.
 */
export async function listAreas(bidYearId: number): Promise<ListAreasResponse> {
  const url = `${API_BASE}/areas?bid_year_id=${encodeURIComponent(bidYearId)}`;
  return fetchJson<ListAreasResponse>(url);
}

/**
 * List all users in a specific area for a bid year.
 */
export async function listUsers(
  sessionToken: string,
  areaId: number,
): Promise<ListUsersResponse> {
  const url = `${API_BASE}/users?area_id=${encodeURIComponent(areaId)}`;
  return fetchJson<ListUsersResponse>(url, {
    headers: {
      Authorization: `Bearer ${sessionToken}`,
    },
  });
}

/**
 * Get detailed leave availability for a specific user.
 */
export async function getLeaveAvailability(
  userId: number,
): Promise<LeaveAvailabilityResponse> {
  const url = `${API_BASE}/leave/availability?user_id=${encodeURIComponent(
    userId,
  )}`;
  return fetchJson<LeaveAvailabilityResponse>(url);
}

/**
 * Get bootstrap status summary for all bid years and areas.
 */
export async function getBootstrapStatus(): Promise<BootstrapStatusResponse> {
  return fetchJson<BootstrapStatusResponse>(`${API_BASE}/bootstrap/status`);
}

/**
 * Check if the system is in bootstrap mode (no operators exist).
 */
export async function checkBootstrapAuthStatus(): Promise<{
  is_bootstrap_mode: boolean;
}> {
  return fetchJson(`${API_BASE}/auth/bootstrap/status`);
}

/**
 * Perform bootstrap login with admin/admin credentials.
 */
export async function bootstrapLogin(
  username: string,
  password: string,
): Promise<{ bootstrap_token: string; is_bootstrap: boolean }> {
  return fetchJson(`${API_BASE}/auth/bootstrap/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ username, password }),
  });
}

/**
 * Create the first admin operator during bootstrap.
 */
export async function createFirstAdmin(
  loginName: string,
  displayName: string,
  password: string,
  passwordConfirmation: string,
): Promise<{
  operator_id: number;
  login_name: string;
  display_name: string;
  message: string;
}> {
  return fetchJson(`${API_BASE}/auth/bootstrap/create-first-admin`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      login_name: loginName,
      display_name: displayName,
      password,
      password_confirmation: passwordConfirmation,
    }),
  });
}

/**
 * Login with operator credentials.
 */
export async function login(
  loginName: string,
  password: string,
): Promise<{
  session_token: string;
  login_name: string;
  display_name: string;
  role: string;
  expires_at: string;
}> {
  return fetchJson(`${API_BASE}/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      login_name: loginName,
      password,
    }),
  });
}

/**
 * Logout and delete the current session.
 */
export async function logout(sessionToken: string): Promise<void> {
  await fetchJson(`${API_BASE}/auth/logout`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${sessionToken}`,
    },
    body: JSON.stringify({ session_token: sessionToken }),
  });
}

/**
 * Get current operator information.
 */
export async function whoami(sessionToken: string): Promise<WhoAmIResponse> {
  return fetchJson(`${API_BASE}/auth/me`, {
    headers: {
      Authorization: `Bearer ${sessionToken}`,
    },
  });
}

/**
 * List all operators (admin only).
 */
export async function listOperators(sessionToken: string): Promise<{
  operators: OperatorInfo[];
}> {
  return fetchJson(`${API_BASE}/operators`, {
    headers: {
      Authorization: `Bearer ${sessionToken}`,
    },
  });
}

/**
 * Create a new operator (admin only).
 */
export async function createOperator(
  sessionToken: string,
  loginName: string,
  displayName: string,
  role: string,
  password: string,
  passwordConfirmation: string,
): Promise<{
  success: boolean;
  message: string | null;
  event_id: number | null;
}> {
  return fetchJson(`${API_BASE}/operators`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${sessionToken}`,
    },
    body: JSON.stringify({
      cause_id: `create-operator-${Date.now()}`,
      cause_description: `Create operator ${loginName}`,
      login_name: loginName,
      display_name: displayName,
      role,
      password,
      password_confirmation: passwordConfirmation,
    }),
  });
}

/**
 * Disable an operator (admin only).
 */
export async function disableOperator(
  sessionToken: string,
  operatorId: number,
): Promise<{
  success: boolean;
  message: string | null;
  event_id: number | null;
}> {
  return fetchJson(`${API_BASE}/operators/disable`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${sessionToken}`,
    },
    body: JSON.stringify({
      cause_id: `disable-operator-${Date.now()}`,
      cause_description: `Disable operator ${operatorId}`,
      operator_id: operatorId,
    }),
  });
}

/**
 * Re-enable an operator (admin only).
 */
export async function enableOperator(
  sessionToken: string,
  operatorId: number,
): Promise<{
  success: boolean;
  message: string | null;
  event_id: number | null;
}> {
  return fetchJson(`${API_BASE}/operators/enable`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${sessionToken}`,
    },
    body: JSON.stringify({
      cause_id: `enable-operator-${Date.now()}`,
      cause_description: `Enable operator ${operatorId}`,
      operator_id: operatorId,
    }),
  });
}

/**
 * Delete an operator (admin only).
 * Only succeeds if the operator is not referenced by any audit events.
 */
export async function deleteOperator(
  sessionToken: string,
  operatorId: number,
): Promise<{
  success: boolean;
  message: string | null;
  event_id: number | null;
}> {
  return fetchJson(`${API_BASE}/operators/delete`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${sessionToken}`,
    },
    body: JSON.stringify({
      cause_id: `delete-operator-${Date.now()}`,
      cause_description: `Delete operator ${operatorId}`,
      operator_id: operatorId,
    }),
  });
}

/**
 * Get bootstrap completeness status for all bid years and areas.
 */
export async function getBootstrapCompleteness(): Promise<GetBootstrapCompletenessResponse> {
  return fetchJson<GetBootstrapCompletenessResponse>(
    `${API_BASE}/bootstrap/completeness`,
  );
}

/**
 * Get the currently active bid year.
 */
export async function getActiveBidYear(): Promise<GetActiveBidYearResponse> {
  return fetchJson<GetActiveBidYearResponse>(
    `${API_BASE}/bootstrap/bid-years/active`,
  );
}

/**
 * Set the active bid year (admin only).
 */
export async function setActiveBidYear(
  sessionToken: string,
  bidYearId: number,
): Promise<SetActiveBidYearResponse> {
  return fetchJson<SetActiveBidYearResponse>(
    `${API_BASE}/bootstrap/bid-years/active`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${sessionToken}`,
      },
      body: JSON.stringify({
        cause_id: `set-active-bid-year-${Date.now()}`,
        cause_description: `Set active bid year`,
        bid_year_id: bidYearId,
      }),
    },
  );
}

/**
 * Set expected area count for the active bid year (admin only).
 */
export async function setExpectedAreaCount(
  sessionToken: string,
  expectedCount: number,
): Promise<SetExpectedAreaCountResponse> {
  return fetchJson<SetExpectedAreaCountResponse>(
    `${API_BASE}/bootstrap/bid-years/expected-areas`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${sessionToken}`,
      },
      body: JSON.stringify({
        cause_id: `set-expected-area-count-${Date.now()}`,
        cause_description: `Set expected area count to ${expectedCount}`,
        expected_count: expectedCount,
      }),
    },
  );
}

/**
 * Set expected user count for an area in the active bid year (admin only).
 */
export async function setExpectedUserCount(
  sessionToken: string,
  areaId: number,
  expectedCount: number,
): Promise<SetExpectedUserCountResponse> {
  return fetchJson<SetExpectedUserCountResponse>(
    `${API_BASE}/bootstrap/areas/expected-users`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${sessionToken}`,
      },
      body: JSON.stringify({
        cause_id: `set-expected-user-count-${Date.now()}`,
        cause_description: `Set expected user count for area ${areaId} to ${expectedCount}`,
        area_id: areaId,
        expected_count: expectedCount,
      }),
    },
  );
}

/**
 * Update an existing user in the active bid year (admin only).
 */
export async function updateUser(
  sessionToken: string,
  userId: number,
  initials: string,
  name: string,
  area: string,
  userType: string,
  crew: number | null,
  cumulativeNatcaBuDate: string,
  natcaBuDate: string,
  eodFaaDate: string,
  serviceComputationDate: string,
  lotteryValue: number | null,
): Promise<UpdateUserResponse> {
  return fetchJson<UpdateUserResponse>(`${API_BASE}/users/update`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${sessionToken}`,
    },
    body: JSON.stringify({
      cause_id: `update-user-${Date.now()}`,
      cause_description: `Update user ${initials} in ${area}`,
      user_id: userId,
      initials,
      name,
      area,
      user_type: userType,
      crew,
      cumulative_natca_bu_date: cumulativeNatcaBuDate,
      natca_bu_date: natcaBuDate,
      eod_faa_date: eodFaaDate,
      service_computation_date: serviceComputationDate,
      lottery_value: lotteryValue,
    }),
  });
}

/**
 * Create a new bid year (admin only).
 */
export async function createBidYear(
  sessionToken: string,
  year: number,
  startDate: string,
  numPayPeriods: number,
): Promise<CreateBidYearResponse> {
  return fetchJson(`${API_BASE}/bid_years`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${sessionToken}`,
    },
    body: JSON.stringify({
      cause_id: `create-bid-year-${Date.now()}`,
      cause_description: `Create bid year ${year}`,
      year,
      start_date: startDate,
      num_pay_periods: numPayPeriods,
    }),
  });
}

/**
 * Create a new area in the active bid year (admin only).
 */
export async function createArea(
  sessionToken: string,
  areaCode: string,
): Promise<CreateAreaResponse> {
  return fetchJson<CreateAreaResponse>(`${API_BASE}/areas`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${sessionToken}`,
    },
    body: JSON.stringify({
      cause_id: `create-area-${Date.now()}`,
      cause_description: `Create area ${areaCode}`,
      area_id: areaCode,
    }),
  });
}

/**
 * Register a new user in the active bid year (admin only).
 */
export async function registerUser(
  sessionToken: string,
  initials: string,
  name: string,
  areaId: number,
  areaCode: string,
  userType: string,
  crew: number | null,
  cumulativeNatcaBuDate: string,
  natcaBuDate: string,
  eodFaaDate: string,
  serviceComputationDate: string,
  lotteryValue: number | null,
): Promise<RegisterUserResponse> {
  return fetchJson(`${API_BASE}/users`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${sessionToken}`,
    },
    body: JSON.stringify({
      cause_id: `register-user-${Date.now()}`,
      cause_description: `Register user ${initials} in ${areaCode}`,
      initials,
      name,
      area_id: areaId,
      user_type: userType,
      crew,
      cumulative_natca_bu_date: cumulativeNatcaBuDate,
      natca_bu_date: natcaBuDate,
      eod_faa_date: eodFaaDate,
      service_computation_date: serviceComputationDate,
      lottery_value: lotteryValue,
    }),
  });
}

/**
 * Preview CSV user data for import validation in the active bid year (admin only).
 */
export async function previewCsvUsers(
  sessionToken: string,
  csvContent: string,
): Promise<PreviewCsvUsersResponse> {
  return fetchJson<PreviewCsvUsersResponse>(
    `${API_BASE}/bootstrap/users/csv/preview`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${sessionToken}`,
      },
      body: JSON.stringify({
        csv_content: csvContent,
      }),
    },
  );
}

/**
 * Import selected CSV rows as users in the active bid year (admin only).
 */
export async function importCsvUsers(
  sessionToken: string,
  csvContent: string,
  selectedRowIndices: number[],
): Promise<ImportCsvUsersResponse> {
  return fetchJson<ImportCsvUsersResponse>(
    `${API_BASE}/bootstrap/users/csv/import`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${sessionToken}`,
      },
      body: JSON.stringify({
        csv_content: csvContent,
        selected_row_indices: selectedRowIndices,
      }),
    },
  );
}

/**
 * Override a user's area assignment after canonicalization.
 *
 * @param sessionToken - The session token for authentication
 * @param userId - The user's canonical identifier
 * @param newAreaId - The new area ID to assign
 * @param reason - The reason for the override (min 10 characters)
 * @returns Promise resolving to the override response
 */
export async function overrideAreaAssignment(
  sessionToken: string,
  userId: number,
  newAreaId: number,
  reason: string,
): Promise<OverrideAreaAssignmentResponse> {
  return fetchJson<OverrideAreaAssignmentResponse>(
    `${API_BASE}/users/override-area`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${sessionToken}`,
      },
      body: JSON.stringify({
        user_id: userId,
        new_area_id: newAreaId,
        reason,
      }),
    },
  );
}
