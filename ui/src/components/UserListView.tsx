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
  sessionToken: string | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function UserListView({
  sessionToken,
  connectionState,
  lastEvent,
}: UserListViewProps) {
  const { bidYearId, areaId } = useParams<{
    bidYearId: string;
    areaId: string;
  }>();
  const navigate = useNavigate();
  const [bidYearIdNum, setBidYearIdNum] = useState<number | null>(null);
  const [areaIdNum, setAreaIdNum] = useState<number | null>(null);
  const [bidYear, setBidYear] = useState<number | null>(null);
  const [areaCode, setAreaCode] = useState<string | null>(null);
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const previousConnectionState = useRef<ConnectionState | null>(null);

  // Parse and validate IDs on mount
  useEffect(() => {
    if (!bidYearId || !areaId) {
      setError("Invalid bid year ID or area ID");
      setLoading(false);
      return;
    }

    const parsedBidYearId = parseInt(bidYearId, 10);
    const parsedAreaId = parseInt(areaId, 10);

    if (Number.isNaN(parsedBidYearId) || Number.isNaN(parsedAreaId)) {
      setError("Invalid bid year ID or area ID");
      setLoading(false);
      return;
    }

    setBidYearIdNum(parsedBidYearId);
    setAreaIdNum(parsedAreaId);
  }, [bidYearId, areaId]);

  useEffect(() => {
    if (areaIdNum === null || !sessionToken) {
      if (!sessionToken) {
        setError("Not authenticated");
        setLoading(false);
      }
      return;
    }

    const loadUsers = async () => {
      try {
        setLoading(true);
        setError(null);
        const response = await listUsers(sessionToken, areaIdNum);
        setUsers(response.users);
        setBidYear(response.bid_year);
        setAreaCode(response.area_code);
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
  }, [areaIdNum, sessionToken]);

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

    if (wasNotConnected && nowConnected && areaIdNum !== null && sessionToken) {
      console.log("[UserListView] Connection established, refreshing data");
      const loadUsers = async () => {
        try {
          setLoading(true);
          setError(null);
          const response = await listUsers(sessionToken, areaIdNum);
          setUsers(response.users);
          setBidYear(response.bid_year);
          setAreaCode(response.area_code);
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
  }, [connectionState, areaIdNum, sessionToken]);

  // Refresh when relevant live events occur
  useEffect(() => {
    if (
      !lastEvent ||
      areaIdNum === null ||
      bidYear === null ||
      areaCode === null ||
      !sessionToken
    )
      return;

    // Events contain display values (bid_year as number, area as string code)
    if (
      (lastEvent.type === "user_registered" &&
        lastEvent.bid_year === bidYear &&
        lastEvent.area === areaCode) ||
      (lastEvent.type === "user_updated" &&
        lastEvent.bid_year === bidYear &&
        lastEvent.area === areaCode)
    ) {
      console.log("[UserListView] Relevant event received, refreshing data");
      const loadUsers = async () => {
        try {
          const response = await listUsers(sessionToken, areaIdNum);
          setUsers(response.users);
          setBidYear(response.bid_year);
          setAreaCode(response.area_code);
        } catch (err) {
          // Silently fail on live event refresh - connection state will show the issue
          console.error("Failed to refresh after live event:", err);
        }
      };
      void loadUsers();
    }
  }, [lastEvent, areaIdNum, bidYear, areaCode, sessionToken]);

  if (bidYearIdNum === null || areaIdNum === null) {
    return (
      <div className="error">
        <h2>Invalid Parameters</h2>
        <p>The bid year ID or area ID parameter is missing or invalid.</p>
        <button type="button" onClick={() => navigate("/admin")}>
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
          onClick={() => navigate(`/admin/bid-year/${bidYearIdNum}/areas`)}
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
          Users in Area {areaCode ?? areaIdNum} - Bid Year{" "}
          {bidYear ?? bidYearIdNum}
        </h2>
        <button
          type="button"
          onClick={() => navigate(`/admin/bid-year/${bidYearIdNum}/areas`)}
        >
          Back to Areas
        </button>
      </div>

      {users.length === 0 && (
        <div className="info-message">
          <p>
            No users registered for area {areaCode ?? areaIdNum} in bid year{" "}
            {bidYear ?? bidYearIdNum}. Use the API or CLI to register users.
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
              <div key={user.user_id} className={cardClassName}>
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
                    to={`/admin/bid-year/${bidYearIdNum}/areas/${areaIdNum}/users/${user.user_id}`}
                  >
                    View Details
                  </Link>
                  <Link
                    to={`/admin/bid-year/${bidYearIdNum}/areas/${areaIdNum}/users/${user.user_id}/edit`}
                    className="link-edit"
                  >
                    Edit
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
