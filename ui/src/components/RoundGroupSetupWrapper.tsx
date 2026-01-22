// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Round Group Setup Wrapper component.
 *
 * Fifth step in the bootstrap workflow.
 * Wraps the existing RoundGroupManagement and RoundManagement components
 * with bootstrap navigation.
 *
 * Purpose: Configure round groups and rounds.
 *
 * Functionality:
 * - Delegates to components implemented in Phase 30B
 * - List round groups
 * - Create/edit/delete round groups
 * - Navigate to rounds management per group
 * - Show round count per group
 *
 * This component acts as a wrapper page around the Phase 30B round group
 * management components, not a duplicate implementation.
 *
 * Completion criteria:
 * - At least one round group exists
 * - Each round group has at least one round defined
 */

import { useCallback, useEffect, useState } from "react";
import { getBootstrapCompleteness, NetworkError } from "../api";
import type {
  ConnectionState,
  GetBootstrapCompletenessResponse,
  LiveEvent,
} from "../types";
import { BootstrapNavigation } from "./BootstrapNavigation";
import { ReadinessWidget } from "./ReadinessWidget";
import { RoundGroupManagement } from "./RoundGroupManagement";

interface RoundGroupSetupWrapperProps {
  sessionToken: string | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function RoundGroupSetupWrapper({
  sessionToken,
  connectionState,
  lastEvent,
}: RoundGroupSetupWrapperProps) {
  const [completeness, setCompleteness] =
    useState<GetBootstrapCompletenessResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadCompleteness = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await getBootstrapCompleteness();
      setCompleteness(response);
    } catch (err) {
      if (err instanceof NetworkError) {
        setError(
          "Backend is unavailable. Please ensure the server is running.",
        );
      } else {
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load round group setup data",
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
      lastEvent.type === "round_group_created" ||
      lastEvent.type === "round_group_updated" ||
      lastEvent.type === "round_group_deleted" ||
      lastEvent.type === "round_created" ||
      lastEvent.type === "round_updated" ||
      lastEvent.type === "round_deleted"
    ) {
      void loadCompleteness();
    }
  }, [lastEvent, loadCompleteness]);

  if (loading) {
    return <div className="loading">Loading round group setup...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load Round Group Setup</h2>
        <p>{error}</p>
      </div>
    );
  }

  if (!completeness) {
    return <div className="error">No completeness data available</div>;
  }

  return (
    <div className="bootstrap-completeness">
      <BootstrapNavigation currentStep="round-groups" />
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
          <h2 className="section-title">Round Group Setup</h2>
          <p className="section-description">
            Configure round groups and rounds for the bidding process. Each
            round group represents a distinct bidding pool, and each round
            within a group represents a separate bidding opportunity.
          </p>

          {/* Round group blockers would be rendered here if defined in BlockingReason type */}

          <div className="bootstrap-section-content">
            <RoundGroupManagement
              sessionToken={sessionToken ?? ""}
              connectionState={connectionState}
              lastEvent={lastEvent}
            />
          </div>
        </section>
      </div>
    </div>
  );
}
