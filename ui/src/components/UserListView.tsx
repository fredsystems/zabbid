// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * User List View component.
 *
 * Displays all users for a selected area in a bid year.
 * Shows user details including leave availability from a single API call.
 * Displays:
 * - initials, name, user_type
 * - earned leave (days + hours)
 * - remaining leave (days + hours)
 * - exhaustion and overdraw indicators
 *
 * This view aggregates all necessary data in one API call to avoid N+1 queries.
 */

import { useEffect, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { listUsers, NetworkError } from "../api";
import type { ConnectionState, LiveEvent, UserInfo } from "../types";

interface UserListViewProps {
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function UserListView({
  connectionState,
  lastEvent,
}: UserListViewProps) {
  const { year, areaId } = useParams<{ year: string; areaId: string }>();
  const navigate = useNavigate();
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const previousConnectionState = useRef<ConnectionState | null>(null);

  const bidYear = year ? parseInt(year, 10) : null;

  useEffect(() => {
    if (!bidYear || !areaId) {
      setError("Invalid bid year or area");
      setLoading(false);
      return;
    }

    const loadUsers = async () => {
      try {
        setLoading(true);
        setError(null);
        const response = await listUsers(bidYear, areaId);
        setUsers(response.users);
      } catch (err) {
        if (err instanceof NetworkError) {
          setError(
            "Backend is unavailable. Please ensure the server is running.",
          );
        } else {
          setError(err instanceof Error ? err.message : "Failed to load users");
        }
      } finally {
        setLoading(false);
      }
    };

    void loadUsers();
  }, [bidYear, areaId]);

  // Auto-refresh when connection is restored
  useEffect(() => {
    console.log(
      "[UserListView] Connection state changed:",
      previousConnectionState.current,
      "->",
      connectionState,
    );

    const wasNotConnected = previousConnectionState.current !== "connected";
    const nowConnected = connectionState === "connected";

    if (wasNotConnected && nowConnected && bidYear && areaId) {
      console.log("[UserListView] Connection established, refreshing data");
      const loadUsers = async () => {
        try {
          setLoading(true);
          setError(null);
          const response = await listUsers(bidYear, areaId);
          setUsers(response.users);
        } catch (err) {
          if (err instanceof NetworkError) {
            setError(
              "Backend is unavailable. Please ensure the server is running.",
            );
          } else {
            setError(
              err instanceof Error ? err.message : "Failed to load users",
            );
          }
        } finally {
          setLoading(false);
        }
      };
      void loadUsers();
    }

    previousConnectionState.current = connectionState;
  }, [connectionState, bidYear, areaId]);

  // Refresh when relevant live events occur
  useEffect(() => {
    if (!lastEvent || !bidYear || !areaId) return;

    if (
      (lastEvent.type === "user_registered" &&
        lastEvent.bid_year === bidYear &&
        lastEvent.area === areaId) ||
      (lastEvent.type === "user_updated" &&
        lastEvent.bid_year === bidYear &&
        lastEvent.area === areaId)
    ) {
      console.log("[UserListView] Relevant event received, refreshing data");
      const loadUsers = async () => {
        try {
          const response = await listUsers(bidYear, areaId);
          setUsers(response.users);
        } catch (err) {
          // Silently fail on live event refresh - connection state will show the issue
          console.error("Failed to refresh after live event:", err);
        }
      };
      void loadUsers();
    }
  }, [lastEvent, bidYear, areaId]);

  if (!bidYear || !areaId) {
    return (
      <div className="error">
        <h2>Invalid Parameters</h2>
        <p>The bid year or area parameter is missing or invalid.</p>
        <button type="button" onClick={() => navigate("/")}>
          Back to Overview
        </button>
      </div>
    );
  }

  if (loading) {
    return <div className="loading">Loading users...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load Users</h2>
        <p>{error}</p>
        {error.includes("unavailable") && (
          <p style={{ marginTop: "1rem", fontSize: "0.9rem", color: "#666" }}>
            Check the connection status indicator in the header. The UI will
            automatically refresh when the backend becomes available.
          </p>
        )}
        <button
          type="button"
          onClick={() => navigate(`/bid-year/${bidYear}/areas`)}
        >
          Back to Areas
        </button>
      </div>
    );
  }

  const formatLeave = (days: number, hours: number): string => {
    return `${days}d ${hours}h`;
  };

  return (
    <div className="user-list-view">
      <div className="view-header">
        <h2>
          Users in Area {areaId} - Bid Year {bidYear}
        </h2>
        <button
          type="button"
          onClick={() => navigate(`/bid-year/${bidYear}/areas`)}
        >
          Back to Areas
        </button>
      </div>

      {users.length === 0 && (
        <div className="info-message">
          <p>
            No users registered for area {areaId} in bid year {bidYear}. Use the
            API or CLI to register users.
          </p>
        </div>
      )}

      {users.length > 0 && (
        <div className="card-list">
          {users.map((user) => {
            const cardClassName = user.is_overdrawn
              ? "data-card card-overdrawn"
              : user.is_exhausted
                ? "data-card card-exhausted"
                : "data-card";

            return (
              <div key={user.initials} className={cardClassName}>
                <div className="card-header">
                  <div>
                    <h3 className="card-title">
                      {user.initials} - {user.name}
                    </h3>
                    <p className="card-subtitle">{user.user_type}</p>
                  </div>
                  <div className="card-badges">
                    {user.is_overdrawn && (
                      <span className="badge error">Overdrawn</span>
                    )}
                    {!user.is_overdrawn && user.is_exhausted && (
                      <span className="badge warning">Exhausted</span>
                    )}
                    {!user.is_overdrawn && !user.is_exhausted && (
                      <span className="badge success">Available</span>
                    )}
                  </div>
                </div>

                <div className="card-body">
                  <div className="card-field">
                    <span className="card-field-label">Crew</span>
                    <span className="card-field-value">
                      {user.crew ?? "N/A"}
                    </span>
                  </div>
                  <div className="card-field">
                    <span className="card-field-label">Earned Leave</span>
                    <span className="card-field-value">
                      {formatLeave(user.earned_days, user.earned_hours)}
                    </span>
                  </div>
                  <div className="card-field">
                    <span className="card-field-label">Remaining Leave</span>
                    <span
                      className={
                        user.remaining_days < 0 || user.remaining_hours < 0
                          ? "card-field-value negative"
                          : "card-field-value"
                      }
                    >
                      {formatLeave(user.remaining_days, user.remaining_hours)}
                    </span>
                  </div>
                </div>

                <div className="card-footer">
                  <Link
                    to={`/bid-year/${bidYear}/area/${encodeURIComponent(
                      areaId,
                    )}/user/${encodeURIComponent(user.initials)}`}
                  >
                    View Details
                  </Link>
                </div>
              </div>
            );
          })}
        </div>
      )}

      <div className="user-summary">
        <h3>User Summary</h3>
        <ul>
          <li>Total Users: {users.length}</li>
          <li>
            Users with Available Leave:{" "}
            {users.filter((u) => !u.is_exhausted && !u.is_overdrawn).length}
          </li>
          <li>
            Users with Exhausted Leave:{" "}
            {users.filter((u) => u.is_exhausted && !u.is_overdrawn).length}
          </li>
          <li>
            Users with Overdrawn Leave:{" "}
            {users.filter((u) => u.is_overdrawn).length}
          </li>
        </ul>
      </div>
    </div>
  );
}
