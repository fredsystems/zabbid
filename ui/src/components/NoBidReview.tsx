// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * No Bid Review component.
 *
 * Phase 26D: Operational workflow for reviewing and resolving users
 * assigned to the "No Bid" system area during bootstrap.
 *
 * Features:
 * - Lists all users currently in No Bid area
 * - Per-user area assignment action
 * - Excludes No Bid from target area selection
 * - Mobile-first responsive design
 * - Empty state when zero users (success message)
 * - Lifecycle state context badge
 *
 * Constraints:
 * - Uses existing updateUser API only
 * - No backend changes
 * - No bulk operations
 * - All styling via SCSS (no inline styles)
 */

import { useCallback, useEffect, useState } from "react";

import {
  ApiError,
  listAreas,
  listBidYears,
  listUsers,
  NetworkError,
  updateUser,
} from "../api";
import type {
  AreaInfo,
  BidYearInfo,
  ConnectionState,
  LiveEvent,
  UserInfo,
} from "../types";

interface NoBidReviewProps {
  bidYearId: number;
  sessionToken: string | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function NoBidReview({
  bidYearId,
  sessionToken,
  connectionState,
  lastEvent,
}: NoBidReviewProps) {
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [areas, setAreas] = useState<AreaInfo[]>([]);
  const [bidYear, setBidYear] = useState<BidYearInfo | null>(null);
  const [noBidArea, setNoBidArea] = useState<AreaInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    if (!sessionToken) {
      setLoading(false);
      return;
    }

    try {
      setLoading(true);
      setError(null);

      // Load bid year info
      const bidYearsResponse = await listBidYears();
      const foundBidYear = bidYearsResponse.find(
        (by) => by.bid_year_id === bidYearId,
      );
      if (!foundBidYear) {
        setError("Bid year not found");
        return;
      }
      setBidYear(foundBidYear);

      // Load areas for this bid year
      const areasResponse = await listAreas(bidYearId);
      setAreas(areasResponse.areas);

      // Find the No Bid area (is_system_area === true && area_code === "NO BID")
      const noBidAreaFound = areasResponse.areas.find(
        (a) => a.is_system_area && a.area_code === "NO BID",
      );

      if (!noBidAreaFound) {
        setError("No Bid area not found for this bid year");
        return;
      }

      setNoBidArea(noBidAreaFound);

      // Load users in No Bid area
      const usersResponse = await listUsers(
        sessionToken,
        noBidAreaFound.area_id,
      );
      setUsers(usersResponse.users);
    } catch (err) {
      if (err instanceof NetworkError) {
        setError(
          "Backend is unavailable. Please ensure the server is running.",
        );
      } else if (err instanceof ApiError) {
        setError(err.message);
      } else {
        setError("Failed to load No Bid review data");
      }
    } finally {
      setLoading(false);
    }
  }, [sessionToken, bidYearId]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  // Auto-refresh when connection is restored
  useEffect(() => {
    if (connectionState === "connected") {
      void loadData();
    }
  }, [connectionState, loadData]);

  // Refresh when relevant live events occur
  useEffect(() => {
    if (!lastEvent) return;

    if (lastEvent.type === "user_updated") {
      void loadData();
    }
  }, [lastEvent, loadData]);

  const handleError = (errorMessage: string) => {
    setError(errorMessage);
    setTimeout(() => setError(null), 5000);
  };

  if (loading) {
    return (
      <div className="no-bid-review">
        <div className="loading">Loading No Bid review...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="no-bid-review">
        <div className="error-panel">
          <h2>Unable to Load No Bid Review</h2>
          <p>{error}</p>
        </div>
      </div>
    );
  }

  if (!bidYear || !noBidArea) {
    return (
      <div className="no-bid-review">
        <div className="error-panel">
          <h2>No Bid Review Unavailable</h2>
          <p>Could not locate the required bid year or No Bid area.</p>
        </div>
      </div>
    );
  }

  // Empty state: zero users in No Bid (success)
  if (users.length === 0) {
    return (
      <div className="no-bid-review">
        <h2>No Bid Review - Year {bidYear.year}</h2>

        <div className="no-bid-header">
          <div className="lifecycle-badge">
            <span
              className={`badge lifecycle-${bidYear.lifecycle_state.toLowerCase()}`}
            >
              {bidYear.lifecycle_state}
            </span>
          </div>
        </div>

        <div className="success-panel">
          <h3>All Clear</h3>
          <p>
            No users are currently assigned to the No Bid area. Bootstrap can
            proceed.
          </p>
        </div>
      </div>
    );
  }

  // Operational areas (exclude No Bid)
  const operationalAreas = areas.filter((a) => !a.is_system_area);

  return (
    <div className="no-bid-review">
      <h2>No Bid Review - Year {bidYear.year}</h2>

      <div className="no-bid-header">
        <div className="lifecycle-badge">
          <span
            className={`badge lifecycle-${bidYear.lifecycle_state.toLowerCase()}`}
          >
            {bidYear.lifecycle_state}
          </span>
        </div>
        <p className="review-description">
          {users.length} user{users.length !== 1 ? "s" : ""} remain in No Bid.
          Assign each user to an operational area to clear this blocker.
        </p>
      </div>

      <div className="no-bid-users-list">
        {users.map((user) => (
          <UserAssignmentCard
            key={user.user_id}
            user={user}
            areas={operationalAreas}
            sessionToken={sessionToken}
            onSuccess={loadData}
            onError={handleError}
          />
        ))}
      </div>

      {error && (
        <div className="error-banner">
          <strong>Error:</strong> {error}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// User Assignment Card Component
// ============================================================================

interface UserAssignmentCardProps {
  user: UserInfo;
  areas: AreaInfo[];
  sessionToken: string | null;
  onSuccess: () => Promise<void>;
  onError: (error: string) => void;
}

function UserAssignmentCard({
  user,
  areas,
  sessionToken,
  onSuccess,
  onError,
}: UserAssignmentCardProps) {
  const [selectedAreaId, setSelectedAreaId] = useState<string>("");
  const [assigning, setAssigning] = useState(false);

  const handleAssign = async () => {
    if (!sessionToken || !selectedAreaId) return;

    const areaIdNum = parseInt(selectedAreaId, 10);
    if (Number.isNaN(areaIdNum)) return;

    try {
      setAssigning(true);
      onError("");

      await updateUser(
        sessionToken,
        user.user_id,
        user.initials,
        user.name,
        areaIdNum,
        user.user_type,
        user.crew,
        user.cumulative_natca_bu_date,
        user.natca_bu_date,
        user.eod_faa_date,
        user.service_computation_date,
        user.lottery_value,
      );

      await onSuccess();
    } catch (err) {
      if (err instanceof ApiError) {
        onError(`Failed to assign ${user.initials}: ${err.message}`);
      } else {
        onError(
          `Failed to assign ${user.initials}: ${err instanceof Error ? err.message : "Unknown error"}`,
        );
      }
    } finally {
      setAssigning(false);
    }
  };

  return (
    <div className="user-assignment-card">
      <div className="user-info">
        <h4>
          {user.initials} - {user.name}
        </h4>
        <div className="user-meta">
          <span className="user-type-badge">{user.user_type}</span>
          {user.crew !== null && (
            <span className="user-crew-badge">Crew {user.crew}</span>
          )}
        </div>
      </div>

      <div className="assignment-controls">
        <label htmlFor={`area-select-${user.user_id}`}>Assign to Area:</label>
        <select
          id={`area-select-${user.user_id}`}
          value={selectedAreaId}
          onChange={(e) => setSelectedAreaId(e.target.value)}
          disabled={assigning}
        >
          <option value="">— Select Area —</option>
          {areas.map((area) => (
            <option key={area.area_id} value={area.area_id.toString()}>
              {area.area_code}
              {area.area_name ? ` - ${area.area_name}` : ""}
            </option>
          ))}
        </select>
        <button
          type="button"
          onClick={handleAssign}
          disabled={!selectedAreaId || assigning}
          className="btn-assign"
        >
          {assigning ? "Assigning..." : "Assign"}
        </button>
      </div>
    </div>
  );
}
