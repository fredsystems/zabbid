// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * User Detail View component.
 *
 * Displays full user metadata and detailed leave accrual breakdown.
 * Shows:
 * - All user metadata (initials, name, type, crew, seniority dates)
 * - Leave accrual breakdown (rich model from Phase 9)
 * - Derived totals and availability
 * - Human-readable explanation
 *
 * This is a read-only view in Phase 12.
 */

import { useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { getLeaveAvailability, NetworkError } from "../api";
import type {
  ConnectionState,
  LeaveAvailabilityResponse,
  LiveEvent,
} from "../types";

interface UserDetailViewProps {
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function UserDetailView({
  connectionState,
  lastEvent,
}: UserDetailViewProps) {
  const { year, areaId, initials } = useParams<{
    year: string;
    areaId: string;
    initials: string;
  }>();
  const navigate = useNavigate();
  const [leaveData, setLeaveData] = useState<LeaveAvailabilityResponse | null>(
    null,
  );
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const previousConnectionState = useRef<ConnectionState | null>(null);

  const bidYear = year ? parseInt(year, 10) : null;

  useEffect(() => {
    if (!bidYear || !areaId || !initials) {
      setError("Invalid parameters");
      setLoading(false);
      return;
    }

    const loadLeaveData = async () => {
      try {
        setLoading(true);
        setError(null);
        const response = await getLeaveAvailability(bidYear, areaId, initials);
        setLeaveData(response);
      } catch (err) {
        if (err instanceof NetworkError) {
          setError(
            "Backend is unavailable. Please ensure the server is running.",
          );
        } else {
          setError(
            err instanceof Error ? err.message : "Failed to load leave data",
          );
        }
      } finally {
        setLoading(false);
      }
    };

    void loadLeaveData();
  }, [bidYear, areaId, initials]);

  // Auto-refresh when connection is restored
  useEffect(() => {
    console.log(
      "[UserDetailView] Connection state changed:",
      previousConnectionState.current,
      "->",
      connectionState,
    );

    const wasNotConnected = previousConnectionState.current !== "connected";
    const nowConnected = connectionState === "connected";

    if (wasNotConnected && nowConnected && bidYear && areaId && initials) {
      console.log("[UserDetailView] Connection established, refreshing data");
      const loadLeaveData = async () => {
        try {
          setLoading(true);
          setError(null);
          const response = await getLeaveAvailability(
            bidYear,
            areaId,
            initials,
          );
          setLeaveData(response);
        } catch (err) {
          if (err instanceof NetworkError) {
            setError(
              "Backend is unavailable. Please ensure the server is running.",
            );
          } else {
            setError(
              err instanceof Error ? err.message : "Failed to load leave data",
            );
          }
        } finally {
          setLoading(false);
        }
      };
      void loadLeaveData();
    }

    previousConnectionState.current = connectionState;
  }, [connectionState, bidYear, areaId, initials]);

  // Refresh when relevant live events occur
  useEffect(() => {
    if (!lastEvent || !bidYear || !areaId || !initials) return;

    if (
      (lastEvent.type === "user_registered" &&
        lastEvent.bid_year === bidYear &&
        lastEvent.area === areaId &&
        lastEvent.initials === initials) ||
      (lastEvent.type === "user_updated" &&
        lastEvent.bid_year === bidYear &&
        lastEvent.area === areaId &&
        lastEvent.initials === initials)
    ) {
      console.log("[UserDetailView] Relevant event received, refreshing data");
      const loadLeaveData = async () => {
        try {
          const response = await getLeaveAvailability(
            bidYear,
            areaId,
            initials,
          );
          setLeaveData(response);
        } catch (err) {
          // Silently fail on live event refresh - connection state will show the issue
          console.error("Failed to refresh after live event:", err);
        }
      };
      void loadLeaveData();
    }
  }, [lastEvent, bidYear, areaId, initials]);

  if (!bidYear || !areaId || !initials) {
    return (
      <div className="error">
        <h2>Invalid Parameters</h2>
        <p>Required parameters are missing or invalid.</p>
        <button type="button" onClick={() => navigate("/")}>
          Back to Overview
        </button>
      </div>
    );
  }

  if (loading) {
    return <div className="loading">Loading user details...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load User Details</h2>
        <p>{error}</p>
        {error.includes("unavailable") && (
          <p style={{ marginTop: "1rem", fontSize: "0.9rem", color: "#666" }}>
            Check the connection status indicator in the header. The UI will
            automatically refresh when the backend becomes available.
          </p>
        )}
        <button
          type="button"
          onClick={() =>
            navigate(
              `/bid-year/${bidYear}/area/${encodeURIComponent(areaId)}/users`,
            )
          }
        >
          Back to User List
        </button>
      </div>
    );
  }

  if (!leaveData) {
    return (
      <div className="error">
        <h2>No Data Available</h2>
        <p>Leave data could not be loaded for user {initials}.</p>
        <button
          type="button"
          onClick={() =>
            navigate(
              `/bid-year/${bidYear}/area/${encodeURIComponent(areaId)}/users`,
            )
          }
        >
          Back to User List
        </button>
      </div>
    );
  }

  const formatLeave = (days: number, hours: number): string => {
    return `${days} days, ${hours} hours`;
  };

  return (
    <div className="user-detail-view">
      <div className="view-header">
        <h2>User Details: {leaveData.initials}</h2>
        <button
          type="button"
          onClick={() =>
            navigate(
              `/bid-year/${bidYear}/area/${encodeURIComponent(areaId)}/users`,
            )
          }
        >
          Back to User List
        </button>
      </div>

      <div className="user-metadata">
        <h3>User Information</h3>
        <div className="metadata-grid">
          <div className="metadata-item">
            <span className="label">Initials:</span>
            <span className="value">{leaveData.initials}</span>
          </div>
          <div className="metadata-item">
            <span className="label">Bid Year:</span>
            <span className="value">{leaveData.bid_year}</span>
          </div>
          <div className="metadata-item">
            <span className="label">Area:</span>
            <span className="value">{areaId}</span>
          </div>
        </div>
      </div>

      <div className="leave-summary">
        <h3>Leave Summary</h3>
        <div className="summary-cards">
          <div className="summary-card">
            <div className="card-title">Earned Leave</div>
            <div className="card-value">
              {formatLeave(leaveData.earned_days, leaveData.earned_hours)}
            </div>
            <div className="card-subtitle">
              Total accrued for bid year {leaveData.bid_year}
            </div>
          </div>

          <div className="summary-card">
            <div className="card-title">Used Leave</div>
            <div className="card-value">{leaveData.used_hours} hours</div>
            <div className="card-subtitle">Leave hours consumed</div>
          </div>

          <div
            className={`summary-card ${
              leaveData.is_overdrawn
                ? "card-error"
                : leaveData.is_exhausted
                  ? "card-warning"
                  : "card-success"
            }`}
          >
            <div className="card-title">Remaining Leave</div>
            <div className="card-value">
              {formatLeave(leaveData.remaining_days, leaveData.remaining_hours)}
            </div>
            <div className="card-subtitle">
              {leaveData.is_overdrawn && "Overdrawn - negative balance"}
              {!leaveData.is_overdrawn &&
                leaveData.is_exhausted &&
                "Exhausted - no leave available"}
              {!leaveData.is_overdrawn &&
                !leaveData.is_exhausted &&
                "Leave available"}
            </div>
          </div>
        </div>
      </div>

      <div className="leave-status">
        <h3>Status Indicators</h3>
        <div className="status-badges">
          {leaveData.is_overdrawn && (
            <div className="status-item error">
              <span className="badge error">Overdrawn</span>
              <p>
                This user has used more leave than earned. The remaining balance
                is negative.
              </p>
            </div>
          )}
          {!leaveData.is_overdrawn && leaveData.is_exhausted && (
            <div className="status-item warning">
              <span className="badge warning">Exhausted</span>
              <p>This user has used all available leave for this bid year.</p>
            </div>
          )}
          {!leaveData.is_overdrawn && !leaveData.is_exhausted && (
            <div className="status-item success">
              <span className="badge success">Available</span>
              <p>This user has leave remaining for this bid year.</p>
            </div>
          )}
        </div>
      </div>

      <div className="leave-explanation">
        <h3>Calculation Explanation</h3>
        <div className="explanation-box">
          <pre>{leaveData.explanation}</pre>
        </div>
      </div>

      <div className="leave-breakdown">
        <h3>Leave Breakdown</h3>
        <table className="breakdown-table">
          <thead>
            <tr>
              <th>Category</th>
              <th>Days</th>
              <th>Hours</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>Total Earned</td>
              <td>{leaveData.earned_days}</td>
              <td>{leaveData.earned_hours}</td>
            </tr>
            <tr>
              <td>Total Used</td>
              <td>—</td>
              <td>{leaveData.used_hours}</td>
            </tr>
            <tr className={leaveData.remaining_days < 0 ? "negative-row" : ""}>
              <td>
                <strong>Remaining</strong>
              </td>
              <td>
                <strong>{leaveData.remaining_days}</strong>
              </td>
              <td>
                <strong>{leaveData.remaining_hours}</strong>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}
