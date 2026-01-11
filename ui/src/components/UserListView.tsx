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

import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { listUsers } from "../api";
import type { UserInfo } from "../types";

export function UserListView() {
  const { year, areaId } = useParams<{ year: string; areaId: string }>();
  const navigate = useNavigate();
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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
        setError(err instanceof Error ? err.message : "Failed to load users");
      } finally {
        setLoading(false);
      }
    };

    void loadUsers();
  }, [bidYear, areaId]);

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
        <h2>Error Loading Users</h2>
        <p>{error}</p>
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
        <table className="users-table">
          <thead>
            <tr>
              <th>Initials</th>
              <th>Name</th>
              <th>Type</th>
              <th>Crew</th>
              <th>Earned Leave</th>
              <th>Remaining Leave</th>
              <th>Status</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {users.map((user) => (
              <tr
                key={user.initials}
                className={
                  user.is_overdrawn
                    ? "user-overdrawn"
                    : user.is_exhausted
                      ? "user-exhausted"
                      : ""
                }
              >
                <td>{user.initials}</td>
                <td>{user.name}</td>
                <td>{user.user_type}</td>
                <td>{user.crew ?? "N/A"}</td>
                <td>{formatLeave(user.earned_days, user.earned_hours)}</td>
                <td
                  className={
                    user.remaining_days < 0 || user.remaining_hours < 0
                      ? "negative-balance"
                      : ""
                  }
                >
                  {formatLeave(user.remaining_days, user.remaining_hours)}
                </td>
                <td>
                  {user.is_overdrawn && (
                    <span className="badge error">Overdrawn</span>
                  )}
                  {!user.is_overdrawn && user.is_exhausted && (
                    <span className="badge warning">Exhausted</span>
                  )}
                  {!user.is_overdrawn && !user.is_exhausted && (
                    <span className="badge success">Available</span>
                  )}
                </td>
                <td>
                  <Link
                    to={`/bid-year/${bidYear}/area/${encodeURIComponent(
                      areaId,
                    )}/user/${encodeURIComponent(user.initials)}`}
                  >
                    View Details
                  </Link>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
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
