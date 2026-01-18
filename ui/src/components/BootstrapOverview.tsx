// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Bootstrap Overview component.
 *
 * Displays all bid years in the system and identifies the active bid year.
 * Shows bootstrap completeness: area count and total user count per bid year.
 *
 * Active bid year logic:
 * - If exactly one bid year exists, it is considered active
 * - If zero or multiple bid years exist, the operator must select one
 * - This is a read-only view; no mutations are performed
 */

import { useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { listBidYears, NetworkError } from "../api";
import type { BidYearInfo, ConnectionState, LiveEvent } from "../types";

interface BootstrapOverviewProps {
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function BootstrapOverview({
  connectionState,
  lastEvent,
}: BootstrapOverviewProps) {
  const [bidYears, setBidYears] = useState<BidYearInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const previousConnectionState = useRef<ConnectionState | null>(null);

  useEffect(() => {
    const loadBidYears = async () => {
      try {
        setLoading(true);
        setError(null);
        const years = await listBidYears();
        setBidYears(years);
      } catch (err) {
        if (err instanceof NetworkError) {
          setError(
            "Backend is unavailable. Please ensure the server is running.",
          );
        } else {
          setError(
            err instanceof Error ? err.message : "Failed to load bid years",
          );
        }
      } finally {
        setLoading(false);
      }
    };

    void loadBidYears();
  }, []);

  // Auto-refresh when connection is restored
  useEffect(() => {
    console.log(
      "[BootstrapOverview] Connection state changed:",
      previousConnectionState.current,
      "->",
      connectionState,
    );

    const wasNotConnected = previousConnectionState.current !== "connected";
    const nowConnected = connectionState === "connected";

    if (wasNotConnected && nowConnected) {
      console.log(
        "[BootstrapOverview] Connection established, refreshing data",
      );
      const loadBidYears = async () => {
        try {
          setLoading(true);
          setError(null);
          const years = await listBidYears();
          setBidYears(years);
        } catch (err) {
          if (err instanceof NetworkError) {
            setError(
              "Backend is unavailable. Please ensure the server is running.",
            );
          } else {
            setError(
              err instanceof Error ? err.message : "Failed to load bid years",
            );
          }
        } finally {
          setLoading(false);
        }
      };
      void loadBidYears();
    }

    previousConnectionState.current = connectionState;
  }, [connectionState]);

  // Refresh when relevant live events occur
  useEffect(() => {
    if (!lastEvent) return;

    if (lastEvent.type === "bid_year_created") {
      console.log(
        "[BootstrapOverview] Bid year created event, refreshing data",
      );
      const loadBidYears = async () => {
        try {
          const years = await listBidYears();
          setBidYears(years);
        } catch (err) {
          // Silently fail on live event refresh - connection state will show the issue
          console.error("Failed to refresh after live event:", err);
        }
      };
      void loadBidYears();
    }
  }, [lastEvent]);

  if (loading) {
    return <div className="loading">Loading bid years...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load Bid Years</h2>
        <p>{error}</p>
        {error.includes("unavailable") && (
          <p style={{ marginTop: "1rem", fontSize: "0.9rem", color: "#666" }}>
            Check the connection status indicator in the header. The UI will
            automatically refresh when the backend becomes available.
          </p>
        )}
      </div>
    );
  }

  // Determine active bid year: if exactly one exists, it's active
  const activeBidYear = bidYears.length === 1 ? bidYears[0] : null;

  return (
    <div className="bootstrap-overview">
      <h2>Bootstrap Overview</h2>

      {bidYears.length === 0 && (
        <div className="info-message">
          <p>
            No bid years configured. Use the API or CLI to create a bid year.
          </p>
        </div>
      )}

      {bidYears.length > 1 && (
        <div className="warning-message">
          <p>
            <strong>Multiple bid years detected.</strong> The system is designed
            to operate on exactly one active bid year at a time. Select a bid
            year below to view its details.
          </p>
        </div>
      )}

      {activeBidYear && (
        <div className="active-bid-year-notice">
          <p>
            <strong>Active Bid Year:</strong> {activeBidYear.year}
          </p>
        </div>
      )}

      {bidYears.length > 0 && (
        <div className="bid-years-list">
          {bidYears.map((year) => {
            const isActive = activeBidYear?.year === year.year;
            return (
              <div
                key={year.year}
                className={`bid-year-card ${isActive ? "active" : ""}`}
              >
                <div className="card-header">
                  <h3>Bid Year {year.year}</h3>
                  <div className="badges">
                    {isActive && <span className="badge active">Active</span>}
                    {!isActive && (
                      <span className="badge inactive">Inactive</span>
                    )}
                    <span
                      className={`badge lifecycle-${year.lifecycle_state.toLowerCase()}`}
                      title={`Lifecycle: ${year.lifecycle_state}`}
                    >
                      {year.lifecycle_state}
                      {(year.lifecycle_state === "Canonicalized" ||
                        year.lifecycle_state === "BiddingActive" ||
                        year.lifecycle_state === "BiddingClosed") &&
                        " 🔒"}
                    </span>
                  </div>
                </div>
                <div className="card-body">
                  <dl>
                    <dt>Start Date:</dt>
                    <dd>{year.start_date}</dd>
                    <dt>End Date:</dt>
                    <dd>{year.end_date}</dd>
                    <dt>Pay Periods:</dt>
                    <dd>{year.num_pay_periods}</dd>
                    <dt>Lifecycle:</dt>
                    <dd>
                      {year.lifecycle_state}
                      {year.lifecycle_state === "Draft" &&
                        " — Setup in progress"}
                      {year.lifecycle_state === "BootstrapComplete" &&
                        " — Ready for canonicalization"}
                      {year.lifecycle_state === "Canonicalized" &&
                        " — Structure locked"}
                      {year.lifecycle_state === "BiddingActive" &&
                        " — Bidding in progress"}
                      {year.lifecycle_state === "BiddingClosed" &&
                        " — Bidding complete"}
                    </dd>
                    <dt>Areas:</dt>
                    <dd>{year.area_count}</dd>
                    <dt>Total Users:</dt>
                    <dd>{year.total_user_count}</dd>
                  </dl>
                </div>
                <div className="card-footer">
                  <Link
                    to={`/admin/bid-year/${year.bid_year_id}/areas`}
                    className="btn-view"
                  >
                    View Areas
                  </Link>
                </div>
              </div>
            );
          })}
        </div>
      )}

      <div className="bootstrap-summary">
        <h3>System Summary</h3>
        <ul>
          <li>Total Bid Years: {bidYears.length}</li>
          <li>
            Total Areas: {bidYears.reduce((sum, y) => sum + y.area_count, 0)}
          </li>
          <li>
            Total Users:{" "}
            {bidYears.reduce((sum, y) => sum + y.total_user_count, 0)}
          </li>
        </ul>
      </div>
    </div>
  );
}
