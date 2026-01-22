// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Area Setup component.
 *
 * Second step in the bootstrap workflow.
 * Allows admin to configure all operational areas.
 *
 * Completion criteria:
 * - Actual non-system area count matches expected
 * - All non-system areas have expected user counts set
 */

import { useCallback, useEffect, useState } from "react";
import {
  ApiError,
  createArea,
  getBootstrapCompleteness,
  NetworkError,
  setExpectedUserCount as setExpectedUserCountApi,
} from "../api";
import type {
  AreaCompletenessInfo,
  BlockingReason,
  ConnectionState,
  GetBootstrapCompletenessResponse,
  GlobalCapabilities,
  LiveEvent,
} from "../types";
import { BootstrapNavigation } from "./BootstrapNavigation";
import { ReadinessWidget } from "./ReadinessWidget";

interface AreaSetupProps {
  sessionToken: string | null;
  capabilities: GlobalCapabilities | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function AreaSetup({
  sessionToken,
  capabilities,
  connectionState,
  lastEvent,
}: AreaSetupProps) {
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

    if (lastEvent.type === "area_created") {
      void loadCompleteness();
    }
  }, [lastEvent, loadCompleteness]);

  if (loading) {
    return <div className="loading">Loading area configuration...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load Area Setup</h2>
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

  const getLifecycleStateForBidYear = (bidYearId: number): string => {
    const bidYear = completeness?.bid_years.find(
      (by) => by.bid_year_id === bidYearId,
    );
    return bidYear?.lifecycle_state ?? "Draft";
  };

  return (
    <div className="bootstrap-completeness">
      <h2>Bootstrap Workflow: Area Setup</h2>

      <BootstrapNavigation currentStep="areas" />

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
        <h3 className="section-title">Areas</h3>
        <p className="section-description">
          Configure all operational areas for the active bid year. Set expected
          user counts for each area to enable roster validation.
        </p>

        {completeness.active_bid_year === null && (
          <p className="empty-state">
            No active bid year set. Return to Bid Year Setup to set an active
            bid year.
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
                lifecycleState={getLifecycleStateForBidYear(area.bid_year_id)}
                onRefresh={loadCompleteness}
                onError={setError}
              />
            ))}
        </div>

        {isAdmin && completeness.active_bid_year !== null && (
          <CreateAreaForm
            sessionToken={sessionToken}
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

      {error && (
        <div className="error-banner">
          <strong>Error:</strong> {error}
        </div>
      )}
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
  lifecycleState: string;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function AreaItem({
  area,
  isAdmin,
  sessionToken,
  lifecycleState,
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

  const isSystemArea = area.is_system_area;
  const isCanonicalizedOrLater =
    lifecycleState === "Canonicalized" ||
    lifecycleState === "BiddingActive" ||
    lifecycleState === "BiddingClosed";

  return (
    <div
      className={`area-item ${area.is_complete ? "complete" : "incomplete"}`}
    >
      <div className="item-header">
        <div className="item-title-group">
          <h4>
            {area.area_code}
            {isSystemArea && " (System Area)"}
          </h4>
          <div className="badges">
            {area.is_complete ? (
              <span className="badge complete">✓ Complete</span>
            ) : (
              <span className="badge incomplete">⚠ Incomplete</span>
            )}
          </div>
        </div>
      </div>

      <div className="item-body">
        {!isEditing ? (
          <div className="item-details">
            <dl>
              {!isSystemArea && (
                <>
                  <dt>Expected Users:</dt>
                  <dd>{area.expected_user_count ?? "Not Set"}</dd>
                  <dt>Actual Users:</dt>
                  <dd>{area.actual_user_count}</dd>
                </>
              )}
              {isSystemArea && (
                <>
                  <dt>Type:</dt>
                  <dd>System Area</dd>
                  <dt>Current Users:</dt>
                  <dd>{area.actual_user_count}</dd>
                </>
              )}
            </dl>
            {isAdmin && !isSystemArea && !isCanonicalizedOrLater && (
              <button
                type="button"
                onClick={() => setIsEditing(true)}
                className="btn-edit"
              >
                Edit Expected Count
              </button>
            )}
          </div>
        ) : (
          <div className="item-edit-form">
            <div className="form-row">
              <label htmlFor={`expected-user-${area.area_id}`}>
                Expected Users:
              </label>
              <input
                id={`expected-user-${area.area_id}`}
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
  lifecycleState: string;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function CreateAreaForm({
  sessionToken,
  lifecycleState,
  onRefresh,
  onError,
}: CreateAreaFormProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [areaId, setAreaId] = useState("");
  const [expectedUserCount, setExpectedUserCount] = useState("");
  const [creating, setCreating] = useState(false);

  const isCanonicalizedOrLater =
    lifecycleState === "Canonicalized" ||
    lifecycleState === "BiddingActive" ||
    lifecycleState === "BiddingClosed";

  const handleCreate = async () => {
    if (!sessionToken || !areaId) return;

    try {
      setCreating(true);
      onError("");
      const createdArea = await createArea(sessionToken, areaId.toUpperCase());

      if (expectedUserCount) {
        const count = Number.parseInt(expectedUserCount, 10);
        if (!Number.isNaN(count) && count >= 0) {
          try {
            await setExpectedUserCountApi(
              sessionToken,
              createdArea.area_id,
              count,
            );
          } catch (err) {
            console.warn("Failed to set expected user count:", err);
          }
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

  if (isCanonicalizedOrLater) {
    return null;
  }

  if (!isOpen) {
    return (
      <button
        type="button"
        onClick={() => setIsOpen(true)}
        className="btn-create"
      >
        + Create New Area
      </button>
    );
  }

  return (
    <div className="create-form">
      <h4>Create New Area</h4>
      <div className="form-row">
        <label htmlFor="new-area-id">Area Code:</label>
        <input
          id="new-area-id"
          type="text"
          value={areaId}
          onChange={(e) => setAreaId(e.target.value)}
          disabled={creating}
          placeholder="e.g., A1"
          autoFocus
        />
      </div>
      <div className="form-row">
        <label htmlFor="new-area-expected-users">
          Expected User Count (optional):
        </label>
        <input
          id="new-area-expected-users"
          type="number"
          min="0"
          max="1000"
          value={expectedUserCount}
          onChange={(e) => setExpectedUserCount(e.target.value)}
          disabled={creating}
          placeholder="e.g., 50"
        />
      </div>
      <div className="form-actions">
        <button
          type="button"
          onClick={handleCreate}
          disabled={!areaId || creating}
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
