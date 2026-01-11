// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Area View component.
 *
 * Displays all areas for a selected bid year.
 * Shows area_id and user count for each area.
 * Allows navigation into a specific area to view users.
 */

import { useEffect, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { listAreas, NetworkError } from "../api";
import type { AreaInfo, ConnectionState, LiveEvent } from "../types";

interface AreaViewProps {
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function AreaView({ connectionState, lastEvent }: AreaViewProps) {
  const { year } = useParams<{ year: string }>();
  const navigate = useNavigate();
  const [areas, setAreas] = useState<AreaInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const previousConnectionState = useRef<ConnectionState | null>(null);

  const bidYear = year ? parseInt(year, 10) : null;

  useEffect(() => {
    if (!bidYear) {
      setError("Invalid bid year");
      setLoading(false);
      return;
    }

    const loadAreas = async () => {
      try {
        setLoading(true);
        setError(null);
        const response = await listAreas(bidYear);
        setAreas(response.areas);
      } catch (err) {
        if (err instanceof NetworkError) {
          setError(
            "Backend is unavailable. Please ensure the server is running.",
          );
        } else {
          setError(err instanceof Error ? err.message : "Failed to load areas");
        }
      } finally {
        setLoading(false);
      }
    };

    void loadAreas();
  }, [bidYear]);

  // Auto-refresh when connection is restored
  useEffect(() => {
    console.log(
      "[AreaView] Connection state changed:",
      previousConnectionState.current,
      "->",
      connectionState,
    );

    const wasNotConnected = previousConnectionState.current !== "connected";
    const nowConnected = connectionState === "connected";

    if (wasNotConnected && nowConnected && bidYear) {
      console.log("[AreaView] Connection established, refreshing data");
      const loadAreas = async () => {
        try {
          setLoading(true);
          setError(null);
          const response = await listAreas(bidYear);
          setAreas(response.areas);
        } catch (err) {
          if (err instanceof NetworkError) {
            setError(
              "Backend is unavailable. Please ensure the server is running.",
            );
          } else {
            setError(
              err instanceof Error ? err.message : "Failed to load areas",
            );
          }
        } finally {
          setLoading(false);
        }
      };
      void loadAreas();
    }

    previousConnectionState.current = connectionState;
  }, [connectionState, bidYear]);

  // Refresh when relevant live events occur
  useEffect(() => {
    if (!lastEvent || !bidYear) return;

    if (
      (lastEvent.type === "area_created" && lastEvent.bid_year === bidYear) ||
      (lastEvent.type === "user_registered" && lastEvent.bid_year === bidYear)
    ) {
      console.log("[AreaView] Relevant event received, refreshing data");
      const loadAreas = async () => {
        try {
          const response = await listAreas(bidYear);
          setAreas(response.areas);
        } catch (err) {
          // Silently fail on live event refresh - connection state will show the issue
          console.error("Failed to refresh after live event:", err);
        }
      };
      void loadAreas();
    }
  }, [lastEvent, bidYear]);

  if (!bidYear) {
    return (
      <div className="error">
        <h2>Invalid Bid Year</h2>
        <p>The bid year parameter is missing or invalid.</p>
        <button type="button" onClick={() => navigate("/")}>
          Back to Overview
        </button>
      </div>
    );
  }

  if (loading) {
    return <div className="loading">Loading areas...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load Areas</h2>
        <p>{error}</p>
        {error.includes("unavailable") && (
          <p style={{ marginTop: "1rem", fontSize: "0.9rem", color: "#666" }}>
            Check the connection status indicator in the header. The UI will
            automatically refresh when the backend becomes available.
          </p>
        )}
        <button type="button" onClick={() => navigate("/")}>
          Back to Overview
        </button>
      </div>
    );
  }

  return (
    <div className="area-view">
      <div className="view-header">
        <h2>Areas for Bid Year {bidYear}</h2>
        <button type="button" onClick={() => navigate("/")}>
          Back to Overview
        </button>
      </div>

      {areas.length === 0 && (
        <div className="info-message">
          <p>
            No areas configured for bid year {bidYear}. Use the API or CLI to
            create areas.
          </p>
        </div>
      )}

      {areas.length > 0 && (
        <table className="areas-table">
          <thead>
            <tr>
              <th>Area ID</th>
              <th>User Count</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {areas.map((area) => (
              <tr key={area.area_id}>
                <td>{area.area_id}</td>
                <td>{area.user_count}</td>
                <td>
                  <Link
                    to={`/bid-year/${bidYear}/area/${encodeURIComponent(
                      area.area_id,
                    )}/users`}
                  >
                    View Users
                  </Link>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      <div className="area-summary">
        <h3>Area Summary</h3>
        <ul>
          <li>Total Areas: {areas.length}</li>
          <li>
            Total Users: {areas.reduce((sum, a) => sum + a.user_count, 0)}
          </li>
        </ul>
      </div>
    </div>
  );
}
