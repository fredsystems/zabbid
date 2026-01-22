// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Readiness Review component.
 *
 * Eighth and final step in the bootstrap workflow.
 * Allows admin to review all blockers and confirm readiness.
 *
 * Purpose: Review all blockers and confirm Ready to Bid.
 *
 * Functionality:
 * - Display computed readiness state (backend-derived only)
 * - List all blocking reasons
 * - Link blockers to relevant workflow sections
 * - Display lifecycle state badge
 * - Confirm Ready to Bid button (irreversible)
 * - Confirmation modal summarizing frozen inputs
 *
 * Completion criteria:
 * - No blockers remain
 * - Operator confirms Ready to Bid
 * - System transitions to Canonicalized
 */

import { useCallback, useEffect, useState } from "react";
import {
  confirmReadyToBid,
  getBootstrapCompleteness,
  NetworkError,
} from "../api";
import type {
  BlockingReason,
  ConnectionState,
  GetBootstrapCompletenessResponse,
  GlobalCapabilities,
  LiveEvent,
} from "../types";
import { BootstrapNavigation } from "./BootstrapNavigation";
import { ReadinessWidget } from "./ReadinessWidget";

interface ReadinessReviewProps {
  sessionToken: string | null;
  capabilities: GlobalCapabilities | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function ReadinessReview({
  sessionToken,
  capabilities,
  connectionState,
  lastEvent,
}: ReadinessReviewProps) {
  const [completeness, setCompleteness] =
    useState<GetBootstrapCompletenessResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showConfirmModal, setShowConfirmModal] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [confirmationText, setConfirmationText] = useState("");

  const isAdmin = capabilities?.can_create_bid_year ?? false;

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
            : "Failed to load readiness review data",
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

    // Reload on any state change
    void loadCompleteness();
  }, [lastEvent, loadCompleteness]);

  const handleConfirmClick = () => {
    setShowConfirmModal(true);
    setConfirmationText("");
  };

  const handleConfirmSubmit = async () => {
    if (!sessionToken || !completeness) return;

    const activeBidYearInfo = completeness.bid_years.find(
      (by) => by.year === completeness.active_bid_year,
    );

    if (!activeBidYearInfo) {
      setError("Active bid year not found");
      return;
    }

    try {
      setConfirming(true);
      setError(null);
      await confirmReadyToBid(
        sessionToken,
        activeBidYearInfo.bid_year_id,
        confirmationText,
      );
      setShowConfirmModal(false);
      await loadCompleteness();
    } catch (err) {
      if (err instanceof Error) {
        setError(`Failed to confirm ready to bid: ${err.message}`);
      } else {
        setError("Failed to confirm ready to bid");
      }
    } finally {
      setConfirming(false);
    }
  };

  if (loading) {
    return <div className="loading">Loading readiness review...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load Readiness Review</h2>
        <p>{error}</p>
      </div>
    );
  }

  if (!completeness) {
    return <div className="error">No completeness data available</div>;
  }

  const isCanonicalizedOrLater =
    completeness.lifecycle_state === "Canonicalized" ||
    completeness.lifecycle_state === "Active" ||
    completeness.lifecycle_state === "Complete";

  const isReady = completeness.is_ready;
  const blockers = completeness.blocking_reasons;

  const requiredConfirmationText = "I understand this action is irreversible";
  const confirmationMatches =
    confirmationText.trim() === requiredConfirmationText;

  return (
    <div className="bootstrap-completeness">
      <BootstrapNavigation currentStep="readiness" />
      <ReadinessWidget
        lifecycleState={completeness.lifecycle_state}
        isReady={completeness.is_ready}
        blockingReasons={completeness.blocking_reasons}
      />

      <div className="bootstrap-content">
        <section className="bootstrap-section">
          <h2 className="section-title">Readiness Review</h2>
          <p className="section-description">
            Review the system state and resolve any blocking issues before
            confirming ready to bid.
          </p>

          {isCanonicalizedOrLater && (
            <div className="success-banner">
              <strong>✓ System Confirmed:</strong> This bid year has been
              canonicalized and is ready for bidding operations.
            </div>
          )}
        </section>

        {!isCanonicalizedOrLater && (
          <>
            <section className="bootstrap-section">
              <h3 className="section-title">System Status</h3>
              <div className="status-summary">
                <div className="status-item">
                  <span className="status-label">Lifecycle State:</span>
                  <span className={`lifecycle-badge ${completeness.lifecycle_state.toLowerCase()}`}>
                    {completeness.lifecycle_state}
                  </span>
                </div>
                <div className="status-item">
                  <span className="status-label">Readiness:</span>
                  {isReady ? (
                    <span className="readiness-badge ready">✓ Ready</span>
                  ) : (
                    <span className="readiness-badge not-ready">
                      ✗ Not Ready ({blockers.length} blocker
                      {blockers.length !== 1 ? "s" : ""})
                    </span>
                  )}
                </div>
              </div>
            </section>

            {blockers.length > 0 && (
              <section className="bootstrap-section">
                <h3 className="section-title">Blocking Issues</h3>
                <p className="section-description">
                  The following issues must be resolved before confirming ready
                  to bid:
                </p>
                <div className="blockers-list-detail">
                  {blockers.map((blocker, idx) => (
                    <BlockerItem key={idx} blocker={blocker} />
                  ))}
                </div>
              </section>
            )}

            {isReady && (
              <section className="bootstrap-section">
                <h3 className="section-title">Confirm Ready to Bid</h3>
                <div className="confirmation-info">
                  <p>
                    <strong>All blocking issues have been resolved.</strong>
                  </p>
                  <p>
                    Confirming ready to bid will transition the system to{" "}
                    <strong>Canonicalized</strong> state. This action is{" "}
                    <strong>irreversible</strong>.
                  </p>
                  <p>After confirmation, the following data will be frozen:</p>
                  <ul>
                    <li>Bid year configuration</li>
                    <li>Area definitions and round group assignments</li>
                    <li>User roster and area assignments</li>
                    <li>Round groups and rounds</li>
                    <li>Bid schedule</li>
                  </ul>
                  <p>
                    You will still be able to make corrections via override
                    mechanisms, but the baseline configuration will be locked.
                  </p>
                </div>
                {isAdmin && (
                  <button
                    type="button"
                    onClick={handleConfirmClick}
                    className="btn-confirm-ready"
                  >
                    Confirm Ready to Bid
                  </button>
                )}
              </section>
            )}
          </>
        )}

        {isCanonicalizedOrLater && (
          <section className="bootstrap-section">
            <h3 className="section-title">Bootstrap Complete</h3>
            <p className="section-description">
              This bid year has been confirmed and canonicalized. The bootstrap
              workflow is complete.
            </p>
            <p className="section-description">
              You can now proceed with bidding operations.
            </p>
          </section>
        )}

        {error && (
          <div className="error-banner">
            <strong>Error:</strong> {error}
          </div>
        )}
      </div>

      {showConfirmModal && (
        <div className="modal-overlay" onClick={() => setShowConfirmModal(false)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <h3>Confirm Ready to Bid</h3>
            <div className="modal-body">
              <p>
                <strong>This action is irreversible.</strong>
              </p>
              <p>
                Confirming will transition the system to Canonicalized state
                and freeze all baseline configuration.
              </p>
              <p>
                Type the following text to confirm:
              </p>
              <p className="confirmation-required-text">
                {requiredConfirmationText}
              </p>
              <input
                type="text"
                value={confirmationText}
                onChange={(e) => setConfirmationText(e.target.value)}
                placeholder="Type confirmation text here"
                disabled={confirming}
              />
            </div>
            <div className="modal-actions">
              <button
                type="button"
                onClick={handleConfirmSubmit}
                disabled={!confirmationMatches || confirming}
                className="btn-confirm"
              >
                {confirming ? "Confirming..." : "Confirm"}
              </button>
              <button
                type="button"
                onClick={() => setShowConfirmModal(false)}
                disabled={confirming}
                className="btn-cancel"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Blocker Item Component
// ============================================================================

interface BlockerItemProps {
  blocker: BlockingReason;
}

function BlockerItem({ blocker }: BlockerItemProps) {
  const message = renderBlockingReason(blocker);
  const link = getBlockerLink(blocker);

  return (
    <div className="blocker-item">
      <div className="blocker-message">{message}</div>
      {link && (
        <a href={link} className="blocker-link">
          Go to section →
        </a>
      )}
    </div>
  );
}

// ============================================================================
// Blocking Reason Renderer
// ============================================================================

function renderBlockingReason(br: BlockingReason): string {
  switch (br.reason_type) {
    case "NoActiveBidYear":
      return "No active bid year configured";
    case "ExpectedAreaCountNotSet": {
      const { bid_year } = br.details;
      return `Bid Year ${bid_year}: Expected area count not set`;
    }
    case "AreaCountMismatch": {
      const { bid_year, expected, actual } = br.details;
      return `Bid Year ${bid_year}: Expected ${expected} areas, found ${actual}`;
    }
    case "ExpectedUserCountNotSet": {
      const { bid_year, area_code } = br.details;
      return `Area ${area_code} (Bid Year ${bid_year}): Expected user count not set`;
    }
    case "UserCountMismatch": {
      const { bid_year, area_code, expected, actual } = br.details;
      return `Area ${area_code} (Bid Year ${bid_year}): Expected ${expected} users, found ${actual}`;
    }
    case "UnexpectedUsers": {
      const { bid_year, user_count, sample_initials } = br.details;
      const userList = sample_initials
        .slice(0, 5)
        .map((i: string) => `"${i}"`)
        .join(", ");
      return `Bid Year ${bid_year}: ${user_count} unexpected users (e.g. ${userList})`;
    }
    case "NoRoundGroups": {
      const { bid_year } = br.details;
      return `Bid Year ${bid_year}: No round groups defined`;
    }
    case "RoundGroupHasNoRounds": {
      const { bid_year, round_group_name } = br.details;
      return `Bid Year ${bid_year}: Round group "${round_group_name}" has no rounds`;
    }
    case "AreaMissingRoundGroup": {
      const { bid_year, area_code } = br.details;
      return `Area ${area_code} (Bid Year ${bid_year}): No round group assigned`;
    }
    case "BidScheduleNotSet": {
      const { bid_year } = br.details;
      return `Bid Year ${bid_year}: Bid schedule not configured`;
    }
    default:
      return `Unknown blocking reason: ${br.reason_type}`;
  }
}

// ============================================================================
// Blocker Link Resolver
// ============================================================================

function getBlockerLink(br: BlockingReason): string | null {
  switch (br.reason_type) {
    case "NoActiveBidYear":
    case "ExpectedAreaCountNotSet":
      return "/admin/bootstrap/bid-years";
    case "AreaCountMismatch":
    case "ExpectedUserCountNotSet":
      return "/admin/bootstrap/areas";
    case "UserCountMismatch":
    case "UnexpectedUsers":
      return "/admin/bootstrap/users";
    case "NoRoundGroups":
    case "RoundGroupHasNoRounds":
      return "/admin/bootstrap/round-groups";
    case "AreaMissingRoundGroup":
      return "/admin/bootstrap/area-round-groups";
    case "BidScheduleNotSet":
      return "/admin/bootstrap/schedule";
    default:
      return null;
  }
}
