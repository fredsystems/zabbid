// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * No Bid Review Wrapper component.
 *
 * Fourth step in the bootstrap workflow.
 * Wraps the existing NoBidReview component with bootstrap navigation.
 *
 * Purpose: Resolve users in the No Bid system area.
 *
 * Functionality:
 * - Integrates existing No Bid review UI from Phase 26D
 * - List users in No Bid
 * - Reassign to competitive areas
 * - Explicitly confirm user remains in No Bid
 *
 * Completion criteria:
 * - Zero users in No Bid, OR
 * - All No Bid users explicitly reviewed
 *
 * If there are zero users in No Bid, this step is automatically complete.
 */

import { useCallback, useEffect, useState } from "react";
import { getBootstrapCompleteness, listAreas, NetworkError } from "../api";
import type {
  AreaInfo,
  ConnectionState,
  GetBootstrapCompletenessResponse,
  LiveEvent,
} from "../types";
import { BootstrapNavigation } from "./BootstrapNavigation";
import { NoBidReview } from "./NoBidReview";
import { ReadinessWidget } from "./ReadinessWidget";

interface NoBidReviewWrapperProps {
  sessionToken: string | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function NoBidReviewWrapper({
  sessionToken,
  connectionState,
  lastEvent,
}: NoBidReviewWrapperProps) {
  const [completeness, setCompleteness] =
    useState<GetBootstrapCompletenessResponse | null>(null);
  const [noBidArea, setNoBidArea] = useState<AreaInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadCompleteness = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await getBootstrapCompleteness();
      setCompleteness(response);

      // Find the No Bid area for the active bid year
      if (response.active_bid_year_id !== null) {
        const areasResponse = await listAreas(response.active_bid_year_id);
        const noBid = areasResponse.areas.find((a) => a.is_system_area);
        setNoBidArea(noBid ?? null);
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
            : "Failed to load No Bid review data",
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

    if (
      lastEvent.type === "user_updated" ||
      lastEvent.type === "area_created" ||
      lastEvent.type === "area_updated"
    ) {
      void loadCompleteness();
    }
  }, [lastEvent, loadCompleteness]);

  if (loading) {
    return <div className="loading">Loading No Bid review...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load No Bid Review</h2>
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
        <BootstrapNavigation currentStep="no-bid-review" />
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
            <h2 className="section-title">No Bid Review</h2>
            <p className="section-description">
              No active bid year. Please configure a bid year first.
            </p>
          </section>
        </div>
      </div>
    );
  }

  if (!noBidArea) {
    return (
      <div className="bootstrap-completeness">
        <BootstrapNavigation currentStep="no-bid-review" />
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
            <h2 className="section-title">No Bid Review</h2>
            <p className="section-description">
              No system area found. This step will be skipped.
            </p>
          </section>
        </div>
      </div>
    );
  }

  return (
    <div className="bootstrap-completeness">
      <BootstrapNavigation currentStep="no-bid-review" />
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
          <h2 className="section-title">No Bid Review</h2>
          <p className="section-description">
            Review and resolve users in the "No Bid" system area. Users can be
            reassigned to operational areas or explicitly confirmed to remain in
            No Bid.
          </p>
          <p className="section-description">
            If there are zero users in No Bid, this step is automatically
            complete.
          </p>

          {completeness.active_bid_year_id !== null && (
            <div className="bootstrap-section-content">
              <NoBidReview
                bidYearId={completeness.active_bid_year_id}
                sessionToken={sessionToken}
                connectionState={connectionState}
                lastEvent={lastEvent}
              />
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
