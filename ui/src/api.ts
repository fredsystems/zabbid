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
  ErrorResponse,
  LeaveAvailabilityResponse,
  ListAreasResponse,
  ListBidYearsResponse,
  ListUsersResponse,
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
 * Wrapper around fetch that handles JSON parsing and error responses.
 */
async function fetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, init);

  if (!response.ok) {
    let errorMessage = `HTTP ${response.status}: ${response.statusText}`;
    let errorType: string | undefined;

    try {
      const errorData = (await response.json()) as ErrorResponse;
      errorMessage = errorData.message || errorMessage;
      errorType = errorData.error;
    } catch {
      // If we can't parse error JSON, use the default message
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
export async function listAreas(bidYear: number): Promise<ListAreasResponse> {
  const url = `${API_BASE}/areas?bid_year=${encodeURIComponent(bidYear)}`;
  return fetchJson<ListAreasResponse>(url);
}

/**
 * List all users in a specific area for a bid year.
 */
export async function listUsers(
  bidYear: number,
  area: string,
): Promise<ListUsersResponse> {
  const url = `${API_BASE}/users?bid_year=${encodeURIComponent(bidYear)}&area=${encodeURIComponent(area)}`;
  return fetchJson<ListUsersResponse>(url);
}

/**
 * Get detailed leave availability for a specific user.
 */
export async function getLeaveAvailability(
  bidYear: number,
  area: string,
  initials: string,
): Promise<LeaveAvailabilityResponse> {
  const url = `${API_BASE}/leave/availability?bid_year=${encodeURIComponent(bidYear)}&area=${encodeURIComponent(area)}&initials=${encodeURIComponent(initials)}`;
  return fetchJson<LeaveAvailabilityResponse>(url);
}

/**
 * Get bootstrap status summary for all bid years and areas.
 */
export async function getBootstrapStatus(): Promise<BootstrapStatusResponse> {
  return fetchJson<BootstrapStatusResponse>(`${API_BASE}/bootstrap/status`);
}
