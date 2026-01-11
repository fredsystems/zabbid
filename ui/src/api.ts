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
