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
  const { bidYearId } = useParams<{ bidYearId: string }>();
  const navigate = useNavigate();
  const [bidYearIdNum, setBidYearIdNum] = useState<number | null>(null);
  const [bidYear, setBidYear] = useState<number | null>(null);
  const [areas, setAreas] = useState<AreaInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const previousConnectionState = useRef<ConnectionState | null>(null);

  // Parse and validate bidYearId on mount
  useEffect(() => {
    if (!bidYearId) {
      setError("Invalid bid year ID");
      setLoading(false);
      return;
    }

    const parsed = parseInt(bidYearId, 10);
    if (Number.isNaN(parsed)) {
      setError("Invalid bid year ID");
      setLoading(false);
      return;
    }

    setBidYearIdNum(parsed);
  }, [bidYearId]);

  useEffect(() => {
    if (bidYearIdNum === null) {
      return;
    }

    const loadAreas = async () => {
      try {
        setLoading(true);
        setError(null);
        const response = await listAreas(bidYearIdNum);
        setAreas(response.areas);
        setBidYear(response.bid_year);
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
  }, [bidYearIdNum]);

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

    if (wasNotConnected && nowConnected && bidYearIdNum !== null) {
      console.log("[AreaView] Connection established, refreshing data");
      const loadAreas = async () => {
        try {
          setLoading(true);
          setError(null);
          const response = await listAreas(bidYearIdNum);
          setAreas(response.areas);
          setBidYear(response.bid_year);
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
  }, [connectionState, bidYearIdNum]);

  // Refresh when relevant live events occur
  useEffect(() => {
    if (!lastEvent || bidYearIdNum === null || bidYear === null) return;

    // Events contain display values (bid_year as number, area as string)
    // We compare against the fetched bidYear value
    if (
      (lastEvent.type === "area_created" && lastEvent.bid_year === bidYear) ||
      (lastEvent.type === "user_registered" && lastEvent.bid_year === bidYear)
    ) {
      console.log("[AreaView] Relevant event received, refreshing data");
      const loadAreas = async () => {
        try {
          const response = await listAreas(bidYearIdNum);
          setAreas(response.areas);
          setBidYear(response.bid_year);
        } catch (err) {
          // Silently fail on live event refresh - connection state will show the issue
          console.error("Failed to refresh after live event:", err);
        }
      };
      void loadAreas();
    }
  }, [lastEvent, bidYearIdNum, bidYear]);

  if (bidYearIdNum === null) {
    return (
      <div className="error">
        <h2>Invalid Bid Year ID</h2>
        <p>The bid year ID parameter is missing or invalid.</p>
        <button type="button" onClick={() => navigate("/admin")}>
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
        <button type="button" onClick={() => navigate("/admin")}>
          Back to Overview
        </button>
      </div>
    );
  }

  return (
    <div className="area-view">
      <div className="view-header">
        <h2>Areas for Bid Year {bidYear ?? bidYearIdNum}</h2>
        <button type="button" onClick={() => navigate("/admin")}>
          Back to Overview
        </button>
      </div>

      {areas.length === 0 && (
        <div className="info-message">
          <p>
            No areas configured for bid year {bidYear ?? bidYearIdNum}. Use the
            API or CLI to create areas.
          </p>
        </div>
      )}

      {areas.length > 0 && (
        <div className="card-list">
          {areas.map((area) => (
            <div
              key={area.area_id}
              className={`data-card ${area.is_system_area ? "system-area" : ""}`}
            >
              <div className="card-header">
                <div>
                  <h3 className="card-title">
                    Area {area.area_code}
                    {area.is_system_area && (
                      <span
                        className="badge system-area-badge"
                        title="System-managed area. Cannot be renamed or deleted."
                      >
                        System Area
                      </span>
                    )}
                  </h3>
                  {area.area_name && (
                    <p className="card-subtitle">{area.area_name}</p>
                  )}
                  <p className="card-subtitle">
                    {area.user_count} {area.user_count === 1 ? "user" : "users"}
                  </p>
                </div>
              </div>

              <div className="card-body">
                <div className="card-field">
                  <span className="card-field-label">User Count</span>
                  <span className="card-field-value">{area.user_count}</span>
                </div>
                {area.is_system_area && (
                  <div className="card-field">
                    <span className="card-field-label">Type</span>
                    <span className="card-field-value">System Managed</span>
                  </div>
                )}
              </div>

              <div className="card-footer">
                <Link
                  to={`/admin/bid-year/${bidYearIdNum}/areas/${area.area_id}/users`}
                >
                  View Users
                </Link>
              </div>
            </div>
          ))}
        </div>
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
