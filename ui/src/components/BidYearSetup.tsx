// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Bid Year Setup component.
 *
 * First step in the bootstrap workflow.
 * Allows admin to configure bid year metadata and activate.
 *
 * Completion criteria:
 * - Exactly one bid year is active
 * - Expected non-system area count is set
 */

import { useCallback, useEffect, useState } from "react";
import {
  ApiError,
  createBidYear,
  getBootstrapCompleteness,
  NetworkError,
  setActiveBidYear,
  setExpectedAreaCount as setExpectedAreaCountApi,
} from "../api";
import type {
  BidYearCompletenessInfo,
  BlockingReason,
  ConnectionState,
  GetBootstrapCompletenessResponse,
  GlobalCapabilities,
  LiveEvent,
} from "../types";
import { BootstrapNavigation } from "./BootstrapNavigation";
import { ReadinessWidget } from "./ReadinessWidget";

interface BidYearSetupProps {
  sessionToken: string | null;
  capabilities: GlobalCapabilities | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function BidYearSetup({
  sessionToken,
  capabilities,
  connectionState,
  lastEvent,
}: BidYearSetupProps) {
  const [completeness, setCompleteness] =
    useState<GetBootstrapCompletenessResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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

  useEffect(() => {
    if (connectionState === "connected") {
      void loadCompleteness();
    }
  }, [connectionState, loadCompleteness]);

  useEffect(() => {
    if (!lastEvent) return;

    if (lastEvent.type === "bid_year_created") {
      void loadCompleteness();
    }
  }, [lastEvent, loadCompleteness]);

  if (loading) {
    return <div className="loading">Loading bid year configuration...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load Bid Year Setup</h2>
        <p>{error}</p>
      </div>
    );
  }

  if (!completeness) {
    return <div className="error">No completeness data available</div>;
  }

  const activeBidYear = completeness.bid_years.find(
    (by) => by.year === completeness.active_bid_year,
  );

  return (
    <div className="bootstrap-completeness">
      <h2>Bootstrap Workflow: Bid Year Setup</h2>

      <BootstrapNavigation currentStep="bid-years" />

      <ReadinessWidget
        lifecycleState={activeBidYear?.lifecycle_state ?? "Draft"}
        isReadyForBidding={completeness.is_ready_for_bidding}
        blockerCount={
          completeness.blocking_reasons.length +
          completeness.bid_years.reduce(
            (sum, by) => sum + by.blocking_reasons.length,
            0,
          ) +
          completeness.areas.reduce(
            (sum, area) => sum + area.blocking_reasons.length,
            0,
          )
        }
      />

      <section className="bootstrap-section">
        <h3 className="section-title">Bid Years</h3>
        <p className="section-description">
          Configure bid year metadata and set the active bid year. All
          subsequent configuration will apply to the active bid year.
        </p>

        {completeness.active_bid_year === null && (
          <div className="error-banner error-banner-spaced">
            <strong>No Active Bid Year</strong>
            <p>
              All mutations require an active bid year. Create a bid year below
              and set it as active before creating areas or users.
            </p>
          </div>
        )}

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
// Blocking Reason Renderer
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
