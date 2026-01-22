// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Area → Round Group Assignment Wrapper component.
 *
 * Sixth step in the bootstrap workflow.
 * Provides interface for assigning round groups to areas.
 *
 * Purpose: Assign exactly one round group to each non-system area.
 *
 * Functionality:
 * - List all non-system areas
 * - Assign round group to each area
 * - Show assignment status and readiness impact
 *
 * Completion criteria:
 * - Every non-system area has exactly one round group assigned
 */

import { useCallback, useEffect, useState } from "react";
import {
  assignAreaRoundGroup,
  getBootstrapCompleteness,
  listAreas,
  listRoundGroups,
  NetworkError,
} from "../api";
import type {
  AreaInfo,
  ConnectionState,
  GetBootstrapCompletenessResponse,
  GlobalCapabilities,
  LiveEvent,
  RoundGroupInfo,
} from "../types";
import { BootstrapNavigation } from "./BootstrapNavigation";
import { ReadinessWidget } from "./ReadinessWidget";

interface AreaRoundGroupAssignmentWrapperProps {
  sessionToken: string | null;
  capabilities: GlobalCapabilities | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function AreaRoundGroupAssignmentWrapper({
  sessionToken,
  capabilities,
  connectionState,
  lastEvent,
}: AreaRoundGroupAssignmentWrapperProps) {
  const [completeness, setCompleteness] =
    useState<GetBootstrapCompletenessResponse | null>(null);
  const [areas, setAreas] = useState<AreaInfo[]>([]);
  const [roundGroups, setRoundGroups] = useState<RoundGroupInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const isAdmin = capabilities?.can_create_bid_year ?? false;

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await getBootstrapCompleteness();
      setCompleteness(response);

      if (response.active_bid_year_id !== null && sessionToken) {
        const [areasResponse, roundGroupsResponse] = await Promise.all([
          listAreas(response.active_bid_year_id),
          listRoundGroups(sessionToken, response.active_bid_year_id),
        ]);
        setAreas(areasResponse.areas);
        setRoundGroups(roundGroupsResponse.round_groups);
      }
    } catch (err) {
      if (err instanceof NetworkError) {
        setError(
          "Backend is unavailable. Please ensure the server is running.",
        );
      } else {
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load area round group assignment data",
        );
      }
    } finally {
      setLoading(false);
    }
  }, [sessionToken]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  useEffect(() => {
    if (connectionState === "connected") {
      void loadData();
    }
  }, [connectionState, loadData]);

  useEffect(() => {
    if (!lastEvent) return;

    if (
      lastEvent.type === "area_round_group_assigned" ||
      lastEvent.type === "area_created" ||
      lastEvent.type === "area_updated" ||
      lastEvent.type === "round_group_created" ||
      lastEvent.type === "round_group_updated"
    ) {
      void loadData();
    }
  }, [lastEvent, loadData]);

  if (loading) {
    return (
      <div className="loading">Loading area round group assignments...</div>
    );
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load Area Round Group Assignment</h2>
        <p>{error}</p>
      </div>
    );
  }

  if (!completeness) {
    return <div className="error">No completeness data available</div>;
  }

  if (completeness.active_bid_year === null) {
    return (
      <div className="bootstrap-completeness">
        <BootstrapNavigation currentStep="area-round-groups" />
        <ReadinessWidget
          lifecycleState={completeness.bid_years[0]?.lifecycle_state ?? "Draft"}
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
        <div className="bootstrap-content">
          <section className="bootstrap-section">
            <h2 className="section-title">Area → Round Group Assignment</h2>
            <p className="section-description">Loading...</p>
          </section>
        </div>
      </div>
    );
  }

  const nonSystemAreas = areas.filter((a) => !a.is_system_area);

  const activeBidYearInfo = completeness.bid_years.find((by) => by.is_active);
  const lifecycleState = activeBidYearInfo?.lifecycle_state ?? "Draft";

  const isCanonicalizedOrLater =
    lifecycleState === "Canonicalized" ||
    lifecycleState === "Active" ||
    lifecycleState === "Closed";

  return (
    <div className="bootstrap-completeness">
      <BootstrapNavigation currentStep="area-round-groups" />
      <ReadinessWidget
        lifecycleState={completeness.bid_years[0]?.lifecycle_state ?? "Draft"}
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

      <div className="bootstrap-content">
        <section className="bootstrap-section">
          <h2 className="section-title">Area → Round Group Assignment</h2>
          <p className="section-description">
            Assign a round group to each operational area. Each area must have
            exactly one round group assignment before proceeding.
          </p>

          {isCanonicalizedOrLater && (
            <div className="info-banner">
              <strong>Note:</strong> Area round group assignments are locked
              after canonicalization.
            </div>
          )}

          {/* Assignment blockers would be rendered here if defined in BlockingReason type */}
        </section>

        {roundGroups.length === 0 && (
          <section className="bootstrap-section">
            <p className="empty-state">
              No round groups configured yet. Please configure round groups in
              the previous step.
            </p>
          </section>
        )}

        {roundGroups.length > 0 && nonSystemAreas.length === 0 && (
          <section className="bootstrap-section">
            <p className="empty-state">
              No operational areas configured yet. Please configure areas first.
            </p>
          </section>
        )}

        {roundGroups.length > 0 && nonSystemAreas.length > 0 && (
          <section className="bootstrap-section">
            <h3 className="section-title">Area Assignments</h3>
            <div className="area-assignments-list">
              {nonSystemAreas
                .sort((a, b) => a.area_code.localeCompare(b.area_code))
                .map((area) => (
                  <AreaRoundGroupAssignment
                    key={area.area_id}
                    area={area}
                    roundGroups={roundGroups}
                    isAdmin={isAdmin}
                    sessionToken={sessionToken}
                    isLocked={isCanonicalizedOrLater}
                    onRefresh={loadData}
                    onError={setError}
                  />
                ))}
            </div>
          </section>
        )}

        {error && (
          <div className="error-banner">
            <strong>Error:</strong> {error}
          </div>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Area Round Group Assignment Component
// ============================================================================

interface AreaRoundGroupAssignmentProps {
  area: AreaInfo;
  roundGroups: RoundGroupInfo[];
  isAdmin: boolean;
  sessionToken: string | null;
  isLocked: boolean;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function AreaRoundGroupAssignment({
  area,
  roundGroups,
  isAdmin,
  sessionToken,
  isLocked,
  onRefresh,
  onError,
}: AreaRoundGroupAssignmentProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [selectedRoundGroupId, setSelectedRoundGroupId] = useState<
    number | null
  >(area.round_group_id);
  const [saving, setSaving] = useState(false);

  const currentRoundGroup = roundGroups.find(
    (rg) => rg.round_group_id === area.round_group_id,
  );

  const handleSave = async () => {
    if (!sessionToken) {
      onError("Session token missing");
      return;
    }

    try {
      setSaving(true);
      onError("");
      await assignAreaRoundGroup(
        sessionToken,
        area.area_id,
        selectedRoundGroupId,
      );
      setIsEditing(false);
      await onRefresh();
    } catch (err) {
      if (err instanceof Error) {
        onError(`Failed to assign round group: ${err.message}`);
      } else {
        onError("Failed to assign round group");
      }
    } finally {
      setSaving(false);
    }
  };

  const handleCancel = () => {
    setIsEditing(false);
    setSelectedRoundGroupId(area.round_group_id);
  };

  if (isEditing) {
    return (
      <div className="area-assignment-item edit-mode">
        <div className="area-assignment-header">
          <h4>
            {area.area_code}
            {area.area_name && ` - ${area.area_name}`}
          </h4>
        </div>
        <div className="form-row">
          <label htmlFor={`round-group-${area.area_id}`}>
            Round Group:
            <select
              id={`round-group-${area.area_id}`}
              value={selectedRoundGroupId ?? ""}
              onChange={(e) => {
                const val = e.target.value;
                setSelectedRoundGroupId(
                  val === "" ? null : Number.parseInt(val, 10),
                );
              }}
              disabled={saving}
            >
              <option value="">Not Assigned</option>
              {roundGroups.map((rg) => (
                <option key={rg.round_group_id} value={rg.round_group_id}>
                  {rg.name}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="form-actions">
          <button
            type="button"
            onClick={handleSave}
            disabled={saving}
            className="btn-save"
          >
            {saving ? "Saving..." : "Save"}
          </button>
          <button
            type="button"
            onClick={handleCancel}
            disabled={saving}
            className="btn-cancel"
          >
            Cancel
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="area-assignment-item">
      <div className="area-assignment-header">
        <h4>
          {area.area_code}
          {area.area_name && ` - ${area.area_name}`}
        </h4>
        {isAdmin && !isLocked && (
          <button
            type="button"
            onClick={() => setIsEditing(true)}
            className="btn-edit"
          >
            Edit
          </button>
        )}
      </div>
      <div className="area-assignment-body">
        <div className="assignment-info">
          <span className="assignment-label">Round Group:</span>
          {currentRoundGroup ? (
            <span className="assignment-value">{currentRoundGroup.name}</span>
          ) : (
            <span className="assignment-value unassigned">Not Assigned</span>
          )}
          {!currentRoundGroup && (
            <span className="blocker-badge">Blocks Readiness</span>
          )}
        </div>
      </div>
    </div>
  );
}

// Note: Blocking reason rendering removed - area assignment blockers
// would need to be added to the BlockingReason discriminated union type
