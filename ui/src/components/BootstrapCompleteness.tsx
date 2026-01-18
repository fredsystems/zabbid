// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Bootstrap Completeness component.
 *
 * Displays bootstrap completeness status and provides admin controls
 * to complete the bootstrap workflow.
 *
 * This component is strictly non-authoritative:
 * - All completeness checks come from the backend
 * - All blocking reasons are rendered exactly as returned
 * - No domain logic exists in the UI
 *
 * Organized into clear sections:
 * 1. Overall system status
 * 2. Bid year management (list, create, set active)
 * 3. Area management (list, create, set expected counts)
 */

import { useCallback, useEffect, useState } from "react";
import {
  ApiError,
  createArea,
  createBidYear,
  getBootstrapCompleteness,
  listUsers,
  NetworkError,
  registerUser,
  setActiveBidYear,
  setExpectedAreaCount as setExpectedAreaCountApi,
  setExpectedUserCount as setExpectedUserCountApi,
  updateUser,
} from "../api";
import type {
  AreaCompletenessInfo,
  BidYearCompletenessInfo,
  BlockingReason,
  ConnectionState,
  GetBootstrapCompletenessResponse,
  GlobalCapabilities,
  ListUsersResponse,
  LiveEvent,
  UserInfo,
} from "../types";
import { CsvUserImport } from "./CsvUserImport";

interface BootstrapCompletenessProps {
  sessionToken: string | null;
  capabilities: GlobalCapabilities | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function BootstrapCompleteness({
  sessionToken,
  capabilities,
  connectionState,
  lastEvent,
}: BootstrapCompletenessProps) {
  const [completeness, setCompleteness] =
    useState<GetBootstrapCompletenessResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Derive isAdmin from capabilities for backward compatibility
  // TODO: Replace all isAdmin checks with specific capability checks
  const isAdmin = capabilities?.can_create_bid_year ?? false;

  const loadCompleteness = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await getBootstrapCompleteness();
      setCompleteness(response);
    } catch (err) {
      if (err instanceof NetworkError) {
        setError(
          "Backend is unavailable. Please ensure the server is running.",
        );
      } else {
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load bootstrap completeness",
        );
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadCompleteness();
  }, [loadCompleteness]);

  // Auto-refresh when connection is restored
  useEffect(() => {
    if (connectionState === "connected") {
      void loadCompleteness();
    }
  }, [connectionState, loadCompleteness]);

  // Refresh when relevant live events occur
  useEffect(() => {
    if (!lastEvent) return;

    if (
      lastEvent.type === "bid_year_created" ||
      lastEvent.type === "area_created" ||
      lastEvent.type === "user_registered" ||
      lastEvent.type === "user_updated"
    ) {
      void loadCompleteness();
    }
  }, [lastEvent, loadCompleteness]);

  if (loading) {
    return <div className="loading">Loading bootstrap completeness...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load Bootstrap Completeness</h2>
        <p>{error}</p>
      </div>
    );
  }

  if (!completeness) {
    return <div className="error">No completeness data available</div>;
  }

  return (
    <div className="bootstrap-completeness">
      <h2>Bootstrap Completeness</h2>

      {/* Overall System Status */}
      <div
        className={`status-overview ${completeness.is_ready_for_bidding ? "complete" : "incomplete"}`}
      >
        <div className="status-header">
          <h3>System Status</h3>
          {completeness.is_ready_for_bidding ? (
            <span className="status-badge complete">✓ Ready for Bidding</span>
          ) : (
            <span className="status-badge incomplete">
              ⚠ Bootstrap Incomplete
            </span>
          )}
        </div>
        {completeness.active_bid_year !== null && (
          <p className="active-year-notice">
            <strong>Active Bid Year:</strong> {completeness.active_bid_year}
          </p>
        )}
        {completeness.active_bid_year === null && (
          <div className="error-banner" style={{ marginTop: "1rem" }}>
            <strong>No Active Bid Year</strong>
            <p>
              All mutations require an active bid year. Create a bid year below
              and set it as active before creating areas or users.
            </p>
          </div>
        )}
        {completeness.blocking_reasons.length > 0 && (
          <div className="blocking-issues">
            <h4>Issues Preventing Readiness:</h4>
            <ul>
              {completeness.blocking_reasons.map((reason) => (
                <li key={JSON.stringify(reason)}>
                  {renderBlockingReason(reason)}
                </li>
              ))}
            </ul>
          </div>
        )}
        {/* Phase 25E: Prominent display for users in No Bid */}
        {completeness.blocking_reasons.some(
          (r) => typeof r === "object" && "UsersInNoBidArea" in r,
        ) && (
          <div className="bootstrap-blocker-panel">
            <div className="blocker-title">Bootstrap Blocked</div>
            <div className="blocker-message">
              Users remain in the "No Bid" area. These users must be reviewed
              and assigned to an operational area before bootstrap can be
              completed.
            </div>
            {completeness.blocking_reasons
              .filter((r) => typeof r === "object" && "UsersInNoBidArea" in r)
              .map((reason) => {
                if (
                  typeof reason === "object" &&
                  "UsersInNoBidArea" in reason
                ) {
                  const { bid_year, user_count, sample_initials } =
                    reason.UsersInNoBidArea;
                  return (
                    <div key={bid_year} className="no-bid-users-list">
                      <strong>
                        Bid Year {bid_year}: {user_count} user
                        {user_count !== 1 ? "s" : ""}
                      </strong>
                      {sample_initials.length > 0 && (
                        <div>
                          {sample_initials.map((initials) => (
                            <span key={initials} className="user-initials">
                              {initials}
                            </span>
                          ))}
                          {user_count > sample_initials.length && (
                            <span className="user-initials">
                              +{user_count - sample_initials.length} more
                            </span>
                          )}
                        </div>
                      )}
                    </div>
                  );
                }
                return null;
              })}
          </div>
        )}
      </div>

      {/* Bid Years Section */}
      <section className="bootstrap-section">
        <h3 className="section-title">Bid Years</h3>
        {completeness.bid_years.length === 0 && (
          <p className="empty-state">
            No bid years configured. Create one below to get started.
          </p>
        )}
        <div className="bid-years-list">
          {completeness.bid_years.map((bidYear) => (
            <BidYearItem
              key={bidYear.year}
              bidYear={bidYear}
              isAdmin={isAdmin}
              sessionToken={sessionToken}
              activeBidYear={completeness.active_bid_year}
              onRefresh={loadCompleteness}
              onError={setError}
            />
          ))}
        </div>
        {isAdmin && (
          <CreateBidYearForm
            sessionToken={sessionToken}
            hasExistingBidYears={completeness.bid_years.length > 0}
            hasActiveBidYear={completeness.active_bid_year !== null}
            onRefresh={loadCompleteness}
            onError={setError}
          />
        )}
      </section>

      {/* Areas Section */}
      <section className="bootstrap-section">
        <h3 className="section-title">Areas</h3>
        {completeness.active_bid_year === null && (
          <p className="empty-state">
            No active bid year set. Set an active bid year above before creating
            areas.
          </p>
        )}
        {completeness.active_bid_year !== null &&
          completeness.areas.filter(
            (a) => a.bid_year === completeness.active_bid_year,
          ).length === 0 && (
            <p className="empty-state">
              No areas configured for the active bid year. Create one below.
            </p>
          )}
        <div className="areas-list">
          {completeness.areas
            .filter((area) => area.bid_year === completeness.active_bid_year)
            .map((area) => (
              <AreaItem
                key={`${area.bid_year}-${area.area_code}`}
                area={area}
                isAdmin={isAdmin}
                sessionToken={sessionToken}
                onRefresh={loadCompleteness}
                onError={setError}
              />
            ))}
        </div>
        {isAdmin && completeness.active_bid_year !== null && (
          <CreateAreaForm
            sessionToken={sessionToken}
            activeBidYear={completeness.active_bid_year}
            lifecycleState={
              completeness.bid_years.find(
                (by) => by.year === completeness.active_bid_year,
              )?.lifecycle_state ?? "Draft"
            }
            onRefresh={loadCompleteness}
            onError={setError}
          />
        )}
      </section>

      {/* Users Section */}
      {completeness.active_bid_year !== null &&
        completeness.areas.filter(
          (a) => a.bid_year === completeness.active_bid_year,
        ).length > 0 && (
          <section className="bootstrap-section">
            <h3 className="section-title">Users</h3>
            {completeness.areas
              .filter((area) => area.bid_year === completeness.active_bid_year)
              .map((area) => (
                <UserManagementForArea
                  key={`users-${area.bid_year}-${area.area_id}`}
                  areaId={area.area_id}
                  areaCode={area.area_code}
                  isAdmin={isAdmin}
                  sessionToken={sessionToken}
                  onError={setError}
                />
              ))}
          </section>
        )}

      {/* CSV User Import Section */}
      {isAdmin &&
        sessionToken !== null &&
        completeness.active_bid_year !== null &&
        completeness.areas.filter(
          (a) => a.bid_year === completeness.active_bid_year,
        ).length > 0 && (
          <section className="bootstrap-section">
            <h3 className="section-title">CSV User Import</h3>
            <p className="section-description">
              Import multiple users at once from CSV data. Select which rows to
              import after validation.
            </p>
            <CsvUserImport
              sessionToken={sessionToken}
              onImportComplete={() => void loadCompleteness()}
            />
          </section>
        )}

      {error && (
        <div className="error-banner">
          <strong>Error:</strong> {error}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Bid Year Item Component
// ============================================================================

interface BidYearItemProps {
  bidYear: BidYearCompletenessInfo;
  isAdmin: boolean;
  sessionToken: string | null;
  activeBidYear: number | null;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function BidYearItem({
  bidYear,
  isAdmin,
  sessionToken,
  activeBidYear,
  onRefresh,
  onError,
}: BidYearItemProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [expectedAreaCount, setExpectedAreaCount] = useState(
    bidYear.expected_area_count?.toString() ?? "",
  );
  const [saving, setSaving] = useState(false);
  const [settingActive, setSettingActive] = useState(false);

  const handleSetExpectedAreaCount = async () => {
    if (!sessionToken || !expectedAreaCount) return;

    const count = Number.parseInt(expectedAreaCount, 10);
    if (Number.isNaN(count) || count < 0) {
      onError("Expected area count must be a non-negative number");
      return;
    }

    try {
      setSaving(true);
      onError("");
      await setExpectedAreaCountApi(sessionToken, count);
      await onRefresh();
      setIsEditing(false);
    } catch (err) {
      if (err instanceof ApiError) {
        onError(`Failed to set expected area count: ${err.message}`);
      } else {
        onError(
          err instanceof Error
            ? err.message
            : "Failed to set expected area count",
        );
      }
    } finally {
      setSaving(false);
    }
  };

  const handleSetActive = async () => {
    if (!sessionToken) return;

    try {
      setSettingActive(true);
      onError("");
      await setActiveBidYear(sessionToken, bidYear.bid_year_id);
      await onRefresh();
    } catch (err) {
      if (err instanceof ApiError) {
        onError(`Failed to set active bid year: ${err.message}`);
      } else {
        onError(
          err instanceof Error ? err.message : "Failed to set active bid year",
        );
      }
    } finally {
      setSettingActive(false);
    }
  };

  const isActive = activeBidYear === bidYear.year;

  return (
    <div
      className={`bid-year-item ${bidYear.is_complete ? "complete" : "incomplete"}`}
    >
      <div className="item-header">
        <div className="item-title-group">
          <h4>Bid Year {bidYear.year}</h4>
          <div className="badges">
            {isActive && <span className="badge active">Active</span>}
            {bidYear.is_complete ? (
              <span className="badge complete">✓ Complete</span>
            ) : (
              <span className="badge incomplete">⚠ Incomplete</span>
            )}
            <span
              className={`badge lifecycle-${bidYear.lifecycle_state.toLowerCase()}`}
              title={`Lifecycle: ${bidYear.lifecycle_state}`}
            >
              {bidYear.lifecycle_state}
              {(bidYear.lifecycle_state === "Canonicalized" ||
                bidYear.lifecycle_state === "BiddingActive" ||
                bidYear.lifecycle_state === "BiddingClosed") &&
                " 🔒"}
            </span>
          </div>
        </div>
        {isAdmin && !isActive && (
          <button
            type="button"
            onClick={handleSetActive}
            disabled={settingActive}
            className="btn-toggle-active"
          >
            {settingActive ? "Setting..." : "Set Active"}
          </button>
        )}
      </div>

      <div className="item-body">
        {!isEditing ? (
          <div className="item-details">
            <dl>
              <dt>Lifecycle:</dt>
              <dd>
                {bidYear.lifecycle_state}
                {bidYear.lifecycle_state === "Draft" && " — Setup in progress"}
                {bidYear.lifecycle_state === "BootstrapComplete" &&
                  " — Ready for canonicalization"}
                {bidYear.lifecycle_state === "Canonicalized" &&
                  " — Structure locked"}
                {bidYear.lifecycle_state === "BiddingActive" &&
                  " — Bidding in progress"}
                {bidYear.lifecycle_state === "BiddingClosed" &&
                  " — Bidding complete"}
              </dd>
              <dt>Expected Areas:</dt>
              <dd>{bidYear.expected_area_count ?? "Not Set"}</dd>
              <dt>Actual Areas:</dt>
              <dd>{bidYear.actual_area_count}</dd>
            </dl>
            {isAdmin && (
              <button
                type="button"
                onClick={() => setIsEditing(true)}
                className="btn-edit"
              >
                Edit
              </button>
            )}
          </div>
        ) : (
          <div className="item-edit-form">
            <div className="form-row">
              <label htmlFor={`expected-area-${bidYear.year}`}>
                Expected Areas:
              </label>
              <input
                id={`expected-area-${bidYear.year}`}
                type="number"
                min="0"
                value={expectedAreaCount}
                onChange={(e) => setExpectedAreaCount(e.target.value)}
                disabled={saving}
              />
            </div>
            <div className="form-actions">
              <button
                type="button"
                onClick={handleSetExpectedAreaCount}
                disabled={!expectedAreaCount || saving}
                className="btn-save"
              >
                {saving ? "Saving..." : "Save"}
              </button>
              <button
                type="button"
                onClick={() => {
                  setIsEditing(false);
                  setExpectedAreaCount(
                    bidYear.expected_area_count?.toString() ?? "",
                  );
                }}
                disabled={saving}
                className="btn-cancel"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {bidYear.blocking_reasons.length > 0 && (
          <div className="item-issues">
            <strong>Issues:</strong>
            <ul>
              {bidYear.blocking_reasons.map((reason) => (
                <li key={JSON.stringify(reason)}>
                  {renderBlockingReason(reason)}
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Create Bid Year Form Component
// ============================================================================

interface CreateBidYearFormProps {
  sessionToken: string | null;
  hasExistingBidYears: boolean;
  hasActiveBidYear: boolean;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function CreateBidYearForm({
  sessionToken,
  hasExistingBidYears,
  hasActiveBidYear,
  onRefresh,
  onError,
}: CreateBidYearFormProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [year, setYear] = useState("");
  const [startDate, setStartDate] = useState("");
  const [payPeriods, setPayPeriods] = useState("26");
  const [expectedAreaCount, setExpectedAreaCount] = useState("");
  const [creating, setCreating] = useState(false);

  const handleCreate = async () => {
    if (!sessionToken || !year || !startDate) return;

    const yearNum = Number.parseInt(year, 10);
    const payPeriodsNum = Number.parseInt(payPeriods, 10);

    if (Number.isNaN(yearNum) || yearNum < 2000 || yearNum > 2100) {
      onError("Year must be between 2000 and 2100");
      return;
    }

    if (payPeriodsNum !== 26 && payPeriodsNum !== 27) {
      onError("Pay periods must be 26 or 27");
      return;
    }

    try {
      setCreating(true);
      onError("");
      const createdBidYear = await createBidYear(
        sessionToken,
        yearNum,
        startDate,
        payPeriodsNum,
      );

      // If this is the first bid year, automatically set it as active
      if (!hasExistingBidYears || !hasActiveBidYear) {
        try {
          await setActiveBidYear(sessionToken, createdBidYear.bid_year_id);
        } catch (err) {
          console.warn("Failed to set newly created bid year as active:", err);
        }
      }

      // If expected area count is provided, set it
      if (expectedAreaCount) {
        const expectedCount = Number.parseInt(expectedAreaCount, 10);
        if (!Number.isNaN(expectedCount) && expectedCount >= 0) {
          try {
            await setExpectedAreaCountApi(sessionToken, expectedCount);
          } catch (err) {
            // Don't fail the whole operation if setting expected count fails
            console.warn("Failed to set expected area count:", err);
          }
        }
      }

      await onRefresh();
      setIsOpen(false);
      setYear("");
      setStartDate("");
      setPayPeriods("26");
      setExpectedAreaCount("");
    } catch (err) {
      if (err instanceof ApiError) {
        onError(`Failed to create bid year: ${err.message}`);
      } else {
        onError(
          err instanceof Error ? err.message : "Failed to create bid year",
        );
      }
    } finally {
      setCreating(false);
    }
  };

  if (!isOpen) {
    return (
      <button
        type="button"
        onClick={() => setIsOpen(true)}
        className="btn-create"
      >
        + Create New Bid Year
      </button>
    );
  }

  return (
    <div className="create-form">
      <h4>Create New Bid Year</h4>
      <div className="form-row">
        <label htmlFor="new-bid-year">Year:</label>
        <input
          id="new-bid-year"
          type="number"
          min="2000"
          max="2100"
          value={year}
          onChange={(e) => setYear(e.target.value)}
          disabled={creating}
          placeholder="e.g., 2026"
          autoFocus
        />
      </div>
      <div className="form-row">
        <label htmlFor="new-bid-year-start">Start Date:</label>
        <input
          id="new-bid-year-start"
          type="date"
          min="2000-01-01"
          max="2100-12-31"
          value={startDate}
          onChange={(e) => setStartDate(e.target.value)}
          disabled={creating}
        />
      </div>
      <div className="form-row">
        <label htmlFor="new-bid-year-pay-periods">Pay Periods:</label>
        <select
          id="new-bid-year-pay-periods"
          value={payPeriods}
          onChange={(e) => setPayPeriods(e.target.value)}
          disabled={creating}
        >
          <option value="26">26</option>
          <option value="27">27</option>
        </select>
      </div>
      <div className="form-row">
        <label htmlFor="new-bid-year-expected-areas">
          Expected Area Count (optional):
        </label>
        <input
          id="new-bid-year-expected-areas"
          type="number"
          min="0"
          max="100"
          value={expectedAreaCount}
          onChange={(e) => setExpectedAreaCount(e.target.value)}
          disabled={creating}
          placeholder="e.g., 5"
        />
      </div>
      <div className="form-actions">
        <button
          type="button"
          onClick={handleCreate}
          disabled={!year || !startDate || creating}
          className="btn-save"
        >
          {creating ? "Creating..." : "Create"}
        </button>
        <button
          type="button"
          onClick={() => {
            setIsOpen(false);
            setYear("");
            setStartDate("");
            setPayPeriods("26");
            setExpectedAreaCount("");
          }}
          disabled={creating}
          className="btn-cancel"
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

// ============================================================================
// Area Item Component
// ============================================================================

interface AreaItemProps {
  area: AreaCompletenessInfo;
  isAdmin: boolean;
  sessionToken: string | null;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function AreaItem({
  area,
  isAdmin,
  sessionToken,
  onRefresh,
  onError,
}: AreaItemProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [expectedUserCount, setExpectedUserCount] = useState(
    area.expected_user_count?.toString() ?? "",
  );
  const [saving, setSaving] = useState(false);

  const handleSetExpectedUserCount = async () => {
    if (!sessionToken || !expectedUserCount) return;

    const count = Number.parseInt(expectedUserCount, 10);
    if (Number.isNaN(count) || count < 0) {
      onError("Expected user count must be a non-negative number");
      return;
    }

    try {
      setSaving(true);
      onError("");
      await setExpectedUserCountApi(sessionToken, area.area_id, count);
      await onRefresh();
      setIsEditing(false);
    } catch (err) {
      if (err instanceof ApiError) {
        onError(`Failed to set expected user count: ${err.message}`);
      } else {
        onError(
          err instanceof Error
            ? err.message
            : "Failed to set expected user count",
        );
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <div
      className={`area-item ${area.is_complete ? "complete" : "incomplete"}`}
    >
      <div className="item-header">
        <div className="item-title-group">
          <h4>
            {area.area_code}{" "}
            <span className="area-year">(Year {area.bid_year})</span>
          </h4>
          {area.is_complete ? (
            <span className="badge complete">✓ Complete</span>
          ) : (
            <span className="badge incomplete">⚠ Incomplete</span>
          )}
        </div>
      </div>

      <div className="item-body">
        {!isEditing ? (
          <div className="item-details">
            <dl>
              <dt>Expected Users:</dt>
              <dd>{area.expected_user_count ?? "Not Set"}</dd>
              <dt>Actual Users:</dt>
              <dd>{area.actual_user_count}</dd>
            </dl>
            {isAdmin && (
              <button
                type="button"
                onClick={() => setIsEditing(true)}
                className="btn-edit"
              >
                Edit
              </button>
            )}
          </div>
        ) : (
          <div className="item-edit-form">
            <div className="form-row">
              <label
                htmlFor={`expected-user-${area.bid_year}-${area.area_code}`}
              >
                Expected Users:
              </label>
              <input
                id={`expected-user-${area.bid_year}-${area.area_code}`}
                type="number"
                min="0"
                value={expectedUserCount}
                onChange={(e) => setExpectedUserCount(e.target.value)}
                disabled={saving}
              />
            </div>
            <div className="form-actions">
              <button
                type="button"
                onClick={handleSetExpectedUserCount}
                disabled={!expectedUserCount || saving}
                className="btn-save"
              >
                {saving ? "Saving..." : "Save"}
              </button>
              <button
                type="button"
                onClick={() => {
                  setIsEditing(false);
                  setExpectedUserCount(
                    area.expected_user_count?.toString() ?? "",
                  );
                }}
                disabled={saving}
                className="btn-cancel"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {area.blocking_reasons.length > 0 && (
          <div className="item-issues">
            <strong>Issues:</strong>
            <ul>
              {area.blocking_reasons.map((reason) => (
                <li key={JSON.stringify(reason)}>
                  {renderBlockingReason(reason)}
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Create Area Form Component
// ============================================================================

interface CreateAreaFormProps {
  sessionToken: string | null;
  activeBidYear: number;
  lifecycleState: string;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function CreateAreaForm({
  sessionToken,
  activeBidYear,
  lifecycleState,
  onRefresh,
  onError,
}: CreateAreaFormProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [areaId, setAreaId] = useState("");
  const [expectedUserCount, setExpectedUserCount] = useState("");
  const [creating, setCreating] = useState(false);

  // Phase 25E: Disable area creation after canonicalization
  const isCanonicalizedOrLater =
    lifecycleState === "Canonicalized" ||
    lifecycleState === "BiddingActive" ||
    lifecycleState === "BiddingClosed";

  const handleCreate = async () => {
    if (!sessionToken || !areaId) return;

    try {
      setCreating(true);
      onError("");
      const createdArea = await createArea(sessionToken, areaId);

      // Refresh to ensure the backend has the latest metadata
      await onRefresh();

      // If expected user count is provided, set it immediately
      if (expectedUserCount) {
        const count = Number.parseInt(expectedUserCount, 10);
        if (!Number.isNaN(count) && count >= 0) {
          await setExpectedUserCountApi(
            sessionToken,
            createdArea.area_id,
            count,
          );
        }
      }

      await onRefresh();
      setIsOpen(false);
      setAreaId("");
      setExpectedUserCount("");
    } catch (err) {
      if (err instanceof ApiError) {
        onError(`Failed to create area: ${err.message}`);
      } else {
        onError(err instanceof Error ? err.message : "Failed to create area");
      }
    } finally {
      setCreating(false);
    }
  };

  if (!isOpen) {
    return (
      <button
        type="button"
        onClick={() => setIsOpen(true)}
        className="btn-create"
        disabled={isCanonicalizedOrLater}
        title={
          isCanonicalizedOrLater
            ? `Cannot create areas after canonicalization (current state: ${lifecycleState})`
            : "Create a new area for this bid year"
        }
      >
        + Create New Area
      </button>
    );
  }

  return (
    <div className="create-form">
      <h4>Create New Area (Year {activeBidYear})</h4>
      {isCanonicalizedOrLater && (
        <div className="warning-message" style={{ marginBottom: "1rem" }}>
          <strong>Area creation is disabled.</strong>
          <p>
            The bid year is in {lifecycleState} state. Areas cannot be created
            after canonicalization. Use an override if structural changes are
            required.
          </p>
        </div>
      )}
      <div className="form-row">
        <label htmlFor="new-area-id">Area ID:</label>
        <input
          id="new-area-id"
          type="text"
          value={areaId}
          onChange={(e) => setAreaId(e.target.value)}
          disabled={creating || isCanonicalizedOrLater}
          placeholder="e.g., ZAB"
          autoFocus
        />
      </div>
      <div className="form-row">
        <label htmlFor="new-area-expected-users">
          Expected Users (optional):
        </label>
        <input
          id="new-area-expected-users"
          type="number"
          min="0"
          value={expectedUserCount}
          onChange={(e) => setExpectedUserCount(e.target.value)}
          disabled={creating || isCanonicalizedOrLater}
          placeholder="e.g., 50"
        />
      </div>
      <div className="form-actions">
        <button
          type="button"
          onClick={handleCreate}
          disabled={!areaId || creating || isCanonicalizedOrLater}
          className="btn-save"
        >
          {creating ? "Creating..." : "Create"}
        </button>
        <button
          type="button"
          onClick={() => {
            setIsOpen(false);
            setAreaId("");
            setExpectedUserCount("");
          }}
          disabled={creating}
          className="btn-cancel"
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

// ============================================================================
// User Management for Area Component
// ============================================================================

interface UserManagementForAreaProps {
  areaId: number;
  areaCode: string;
  isAdmin: boolean;
  sessionToken: string | null;
  onError: (error: string) => void;
}

function UserManagementForArea({
  areaId,
  areaCode,
  isAdmin,
  sessionToken,
  onError,
}: UserManagementForAreaProps) {
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [showCreateForm, setShowCreateForm] = useState(false);

  useEffect(() => {
    const loadUsers = async () => {
      if (!sessionToken) {
        setUsers([]);
        setLoading(false);
        return;
      }

      try {
        setLoading(true);
        const response: ListUsersResponse = await listUsers(
          sessionToken,
          areaId,
        );
        setUsers(response.users);
      } catch (err) {
        console.error("Failed to load users:", err);
        setUsers([]);
      } finally {
        setLoading(false);
      }
    };

    void loadUsers();
  }, [areaId, sessionToken]);

  const refreshUsers = async () => {
    if (!sessionToken) {
      setUsers([]);
      return;
    }

    try {
      setLoading(true);
      const response: ListUsersResponse = await listUsers(sessionToken, areaId);
      setUsers(response.users);
    } catch (err) {
      console.error("Failed to load users:", err);
      setUsers([]);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="area-user-management">
      <div className="area-user-header">
        <h4>
          {areaCode} - {users.length} user{users.length !== 1 ? "s" : ""}
        </h4>
      </div>

      {loading && <p className="loading-text">Loading users...</p>}

      {!loading && users.length === 0 && (
        <p className="empty-state">No users in this area yet.</p>
      )}

      {!loading && users.length > 0 && (
        <div className="users-list">
          {users.map((user) => (
            <UserItem
              key={user.user_id}
              user={user}
              areaId={areaCode}
              isAdmin={isAdmin}
              sessionToken={sessionToken}
              onRefresh={refreshUsers}
              onError={onError}
            />
          ))}
        </div>
      )}

      {isAdmin && !showCreateForm && (
        <button
          type="button"
          onClick={() => setShowCreateForm(true)}
          className="btn-create"
        >
          + Add User to {areaCode}
        </button>
      )}

      {isAdmin && showCreateForm && (
        <CreateUserForm
          areaId={areaId}
          areaCode={areaCode}
          sessionToken={sessionToken}
          onSuccess={() => {
            setShowCreateForm(false);
            void refreshUsers();
          }}
          onCancel={() => setShowCreateForm(false)}
          onError={onError}
        />
      )}
    </div>
  );
}

// ============================================================================
// User Item Component
// ============================================================================

interface UserItemProps {
  user: UserInfo;
  areaId: string;
  isAdmin: boolean;
  sessionToken: string | null;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function UserItem({
  user,
  areaId,
  isAdmin,
  sessionToken,
  onRefresh,
  onError,
}: UserItemProps) {
  const [isEditing, setIsEditing] = useState(false);

  if (isEditing) {
    return (
      <EditUserForm
        user={user}
        areaId={areaId}
        sessionToken={sessionToken}
        onSuccess={() => {
          setIsEditing(false);
          void onRefresh();
        }}
        onCancel={() => setIsEditing(false)}
        onError={onError}
      />
    );
  }

  return (
    <div className="user-item">
      <div className="user-item-header">
        <div className="user-title-group">
          <h5>
            {user.initials} - {user.name}
          </h5>
          <div className="user-meta">
            <span className="user-type">{user.user_type}</span>
            {user.crew !== null && (
              <span className="user-crew">Crew {user.crew}</span>
            )}
          </div>
        </div>
        {isAdmin && (
          <button
            type="button"
            onClick={() => setIsEditing(true)}
            className="btn-edit"
          >
            Edit
          </button>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Create User Form Component
// ============================================================================

interface CreateUserFormProps {
  areaId: number;
  areaCode: string;
  sessionToken: string | null;
  onSuccess: () => void;
  onCancel: () => void;
  onError: (error: string) => void;
}

function CreateUserForm({
  areaId,
  areaCode,
  sessionToken,
  onSuccess,
  onCancel,
  onError,
}: CreateUserFormProps) {
  const [initials, setInitials] = useState("");
  const [name, setName] = useState("");
  const [userType, setUserType] = useState("CPC");
  const [crew, setCrew] = useState("");
  const [cumulativeNatcaBuDate, setCumulativeNatcaBuDate] = useState("");
  const [natcaBuDate, setNatcaBuDate] = useState("");
  const [eodFaaDate, setEodFaaDate] = useState("");
  const [serviceComputationDate, setServiceComputationDate] = useState("");
  const [lotteryValue, setLotteryValue] = useState("");
  const [creating, setCreating] = useState(false);

  const handleCreate = async () => {
    if (!sessionToken || !initials || !name) return;

    const crewNum = crew ? Number.parseInt(crew, 10) : null;
    const lotteryNum = lotteryValue ? Number.parseInt(lotteryValue, 10) : null;

    if (
      crewNum !== null &&
      (Number.isNaN(crewNum) || crewNum < 1 || crewNum > 7)
    ) {
      onError("Crew must be a number between 1 and 7");
      return;
    }

    try {
      setCreating(true);
      onError("");
      await registerUser(
        sessionToken,
        initials,
        name,
        areaId,
        areaCode,
        userType,
        crewNum,
        cumulativeNatcaBuDate,
        natcaBuDate,
        eodFaaDate,
        serviceComputationDate,
        lotteryNum,
      );
      onSuccess();
    } catch (err) {
      if (err instanceof ApiError) {
        onError(`Failed to create user: ${err.message}`);
      } else {
        onError(err instanceof Error ? err.message : "Failed to create user");
      }
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="create-form user-form">
      <h4>Add User to {areaId}</h4>

      <div className="form-row">
        <label htmlFor="new-user-initials">Initials:</label>
        <input
          id="new-user-initials"
          type="text"
          value={initials}
          onChange={(e) => setInitials(e.target.value.toUpperCase())}
          disabled={creating}
          placeholder="e.g., ABC"
          autoFocus
        />
      </div>

      <div className="form-row">
        <label htmlFor="new-user-name">Name:</label>
        <input
          id="new-user-name"
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          disabled={creating}
          placeholder="e.g., John Doe"
        />
      </div>

      <div className="form-row">
        <label htmlFor="new-user-type">User Type:</label>
        <select
          id="new-user-type"
          value={userType}
          onChange={(e) => setUserType(e.target.value)}
          disabled={creating}
        >
          <option value="CPC">CPC</option>
          <option value="CPC-IT">CPC-IT</option>
          <option value="Dev-R">Dev-R</option>
          <option value="Dev-D">Dev-D</option>
        </select>
      </div>

      <div className="form-row">
        <label htmlFor="new-user-crew">Crew (optional):</label>
        <input
          id="new-user-crew"
          type="number"
          min="1"
          max="7"
          value={crew}
          onChange={(e) => setCrew(e.target.value)}
          disabled={creating}
          placeholder="1-7"
        />
      </div>

      <div className="form-row">
        <label htmlFor="new-user-cum-natca">Cumulative NATCA BU Date:</label>
        <input
          id="new-user-cum-natca"
          type="date"
          min="1960-01-01"
          max="2100-12-31"
          value={cumulativeNatcaBuDate}
          onChange={(e) => setCumulativeNatcaBuDate(e.target.value)}
          disabled={creating}
        />
      </div>

      <div className="form-row">
        <label htmlFor="new-user-natca">NATCA BU Date:</label>
        <input
          id="new-user-natca"
          type="date"
          min="1960-01-01"
          max="2100-12-31"
          value={natcaBuDate}
          onChange={(e) => setNatcaBuDate(e.target.value)}
          disabled={creating}
        />
      </div>

      <div className="form-row">
        <label htmlFor="new-user-eod">EOD/FAA Date:</label>
        <input
          id="new-user-eod"
          type="date"
          min="1960-01-01"
          max="2100-12-31"
          value={eodFaaDate}
          onChange={(e) => setEodFaaDate(e.target.value)}
          disabled={creating}
        />
      </div>

      <div className="form-row">
        <label htmlFor="new-user-scd">Service Computation Date:</label>
        <input
          id="new-user-scd"
          type="date"
          min="1960-01-01"
          max="2100-12-31"
          value={serviceComputationDate}
          onChange={(e) => setServiceComputationDate(e.target.value)}
          disabled={creating}
        />
      </div>

      <div className="form-row">
        <label htmlFor="new-user-lottery">Lottery Value (optional):</label>
        <input
          id="new-user-lottery"
          type="number"
          min="0"
          value={lotteryValue}
          onChange={(e) => setLotteryValue(e.target.value)}
          disabled={creating}
        />
      </div>

      <div className="form-actions">
        <button
          type="button"
          onClick={handleCreate}
          disabled={!initials || !name || creating}
          className="btn-save"
        >
          {creating ? "Creating..." : "Create User"}
        </button>
        <button
          type="button"
          onClick={onCancel}
          disabled={creating}
          className="btn-cancel"
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

// ============================================================================
// Edit User Form Component
// ============================================================================

interface EditUserFormProps {
  user: UserInfo;
  areaId: string;
  sessionToken: string | null;
  onSuccess: () => void;
  onCancel: () => void;
  onError: (error: string) => void;
}

function EditUserForm({
  user,
  areaId,
  sessionToken,
  onSuccess,
  onCancel,
  onError,
}: EditUserFormProps) {
  const [name, setName] = useState(user.name);
  const [userType, setUserType] = useState(user.user_type);
  const [crew, setCrew] = useState(user.crew?.toString() ?? "");
  // Note: We don't have the seniority dates in UserInfo, so we can't edit them here
  // This would need to be fetched from a separate endpoint or included in UserInfo
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    if (!sessionToken || !name) return;

    const crewNum = crew ? Number.parseInt(crew, 10) : null;

    if (
      crewNum !== null &&
      (Number.isNaN(crewNum) || crewNum < 1 || crewNum > 7)
    ) {
      onError("Crew must be a number between 1 and 7");
      return;
    }

    try {
      setSaving(true);
      onError("");
      // Note: This is a simplified update - we're using placeholder dates
      // In a full implementation, we'd need to fetch the full user data first
      await updateUser(
        sessionToken,
        user.user_id,
        user.initials,
        name,
        areaId,
        userType,
        crewNum,
        "2020-01-01", // Placeholder - should fetch actual values
        "2020-01-01",
        "2020-01-01",
        "2020-01-01",
        null,
      );
      onSuccess();
    } catch (err) {
      if (err instanceof ApiError) {
        onError(`Failed to update user: ${err.message}`);
      } else {
        onError(err instanceof Error ? err.message : "Failed to update user");
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="user-item editing">
      <div className="item-edit-form">
        <h5>Edit {user.initials}</h5>

        <div className="form-row">
          <label htmlFor={`edit-user-name-${user.initials}`}>Name:</label>
          <input
            id={`edit-user-name-${user.initials}`}
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={saving}
          />
        </div>

        <div className="form-row">
          <label htmlFor={`edit-user-type-${user.initials}`}>User Type:</label>
          <select
            id={`edit-user-type-${user.initials}`}
            value={userType}
            onChange={(e) => setUserType(e.target.value)}
            disabled={saving}
          >
            <option value="CPC">CPC</option>
            <option value="CPC-IT">CPC-IT</option>
            <option value="Dev-R">Dev-R</option>
            <option value="Dev-D">Dev-D</option>
          </select>
        </div>

        <div className="form-row">
          <label htmlFor={`edit-user-crew-${user.initials}`}>
            Crew (optional):
          </label>
          <input
            id={`edit-user-crew-${user.initials}`}
            type="number"
            min="1"
            max="7"
            value={crew}
            onChange={(e) => setCrew(e.target.value)}
            disabled={saving}
          />
        </div>

        <div className="form-actions">
          <button
            type="button"
            onClick={handleSave}
            disabled={!name || saving}
            className="btn-save"
          >
            {saving ? "Saving..." : "Save"}
          </button>
          <button
            type="button"
            onClick={onCancel}
            disabled={saving}
            className="btn-cancel"
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// Helper Functions
// ============================================================================

function renderBlockingReason(reason: BlockingReason): string {
  if (reason === "NoActiveBidYear") {
    return "No active bid year is set";
  }
  if (typeof reason === "object") {
    if ("ExpectedAreaCountNotSet" in reason) {
      return `Expected area count not set for bid year ${reason.ExpectedAreaCountNotSet.bid_year}`;
    }
    if ("AreaCountMismatch" in reason) {
      const { bid_year, expected, actual } = reason.AreaCountMismatch;
      return `Area count mismatch for bid year ${bid_year}: expected ${expected}, got ${actual}`;
    }
    if ("ExpectedUserCountNotSet" in reason) {
      const { bid_year, area_code } = reason.ExpectedUserCountNotSet;
      return `Expected user count not set for area ${area_code} in bid year ${bid_year}`;
    }
    if ("UserCountMismatch" in reason) {
      const { bid_year, area_code, expected, actual } =
        reason.UserCountMismatch;
      return `User count mismatch for area ${area_code} in bid year ${bid_year}: expected ${expected}, got ${actual}`;
    }
    if ("UsersInNoBidArea" in reason) {
      const { bid_year, user_count, sample_initials } = reason.UsersInNoBidArea;
      const userList =
        sample_initials.length > 0
          ? ` (${sample_initials.join(", ")}${user_count > sample_initials.length ? ", ..." : ""})`
          : "";
      return `${user_count} user${user_count !== 1 ? "s" : ""} remain in No Bid area for bid year ${bid_year}${userList}`;
    }
  }
  return "Unknown blocking reason";
}
