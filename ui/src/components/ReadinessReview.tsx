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

  const activeBidYearInfo = completeness.bid_years.find((by) => by.is_active);
  const lifecycleState = activeBidYearInfo?.lifecycle_state ?? "Draft";

  const isCanonicalizedOrLater =
    lifecycleState === "Canonicalized" ||
    lifecycleState === "Active" ||
    lifecycleState === "Closed";

  const isReady = completeness.is_ready_for_bidding;

  // Collect all blockers from all levels
  const allBlockers: BlockingReason[] = [
    ...completeness.blocking_reasons,
    ...completeness.bid_years.flatMap((by) => by.blocking_reasons),
    ...completeness.areas.flatMap((area) => area.blocking_reasons),
  ];

  const requiredConfirmationText = "I understand this action is irreversible";
  const confirmationMatches =
    confirmationText.trim() === requiredConfirmationText;

  return (
    <div className="bootstrap-completeness">
      <BootstrapNavigation currentStep="readiness" />

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
              <h3 className="section-title">Status Review</h3>
              <div className="status-summary">
                <div className="readiness-status-item">
                  <span className="status-label">Lifecycle State:</span>
                  <span
                    className={`lifecycle-badge ${lifecycleState.toLowerCase()}`}
                  >
                    {lifecycleState}
                  </span>
                </div>
                <div className="readiness-status-item">
                  <span className="status-label">Readiness:</span>
                  {isReady ? (
                    <span className="readiness-badge ready">✓ Ready</span>
                  ) : (
                    <span className="readiness-badge not-ready">
                      ✗ Not Ready ({allBlockers.length} blocker
                      {allBlockers.length !== 1 ? "s" : ""})
                    </span>
                  )}
                </div>
              </div>
            </section>

            {allBlockers.length > 0 && (
              <section className="bootstrap-section">
                <h3 className="section-title">Blocking Issues</h3>
                <p className="section-description">
                  The following issues must be resolved before confirming ready
                  to bid:
                </p>
                <div className="blockers-list-detail">
                  {allBlockers.map((blocker, index) => (
                    <BlockerItem
                      key={`${JSON.stringify(blocker)}-${index}`}
                      blocker={blocker}
                    />
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
        <button
          type="button"
          className="modal-overlay"
          onClick={() => setShowConfirmModal(false)}
          onKeyDown={(e) => {
            if (e.key === "Escape") setShowConfirmModal(false);
          }}
        >
          <div
            className="modal-content"
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
          >
            <h3>Confirm Ready to Bid</h3>
            <div className="modal-body">
              <p>
                <strong>This action is irreversible.</strong>
              </p>
              <p>
                Confirming will transition the system to Canonicalized state and
                freeze all baseline configuration.
              </p>
              <p>Type the following text to confirm:</p>
              <p className="confirmation-required-text">
                {requiredConfirmationText}
              </p>
              <input
                type="text"
                value={confirmationText}
                onChange={(e) => setConfirmationText(e.target.value)}
                placeholder="Type confirmation text here"
                className="confirmation-input"
              />
            </div>
            <div className="modal-actions">
              <button
                type="button"
                onClick={handleConfirmSubmit}
                disabled={!confirmationMatches || confirming}
                className="btn-primary"
              >
                {confirming ? "Confirming..." : "Confirm Ready to Bid"}
              </button>
              <button
                type="button"
                onClick={() => setShowConfirmModal(false)}
                disabled={confirming}
                className="btn-secondary"
              >
                Cancel
              </button>
            </div>
          </div>
        </button>
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
  if (br === "NoActiveBidYear") {
    return "No active bid year configured";
  }

  if (typeof br === "object") {
    if ("ExpectedAreaCountNotSet" in br) {
      const { bid_year } = br.ExpectedAreaCountNotSet;
      return `Bid Year ${bid_year}: Expected area count not set`;
    }
    if ("AreaCountMismatch" in br) {
      const { bid_year, expected, actual } = br.AreaCountMismatch;
      return `Bid Year ${bid_year}: Expected ${expected} areas, found ${actual}`;
    }
    if ("ExpectedUserCountNotSet" in br) {
      const { bid_year, area_code } = br.ExpectedUserCountNotSet;
      return `Area ${area_code} (Bid Year ${bid_year}): Expected user count not set`;
    }
    if ("UserCountMismatch" in br) {
      const { bid_year, area_code, expected, actual } = br.UserCountMismatch;
      return `Area ${area_code} (Bid Year ${bid_year}): Expected ${expected} users, found ${actual}`;
    }
    if ("UsersInNoBidArea" in br) {
      const { bid_year, user_count, sample_initials } = br.UsersInNoBidArea;
      const userList = sample_initials
        .slice(0, 5)
        .map((i: string) => `"${i}"`)
        .join(", ");
      return `Bid Year ${bid_year}: ${user_count} users in No Bid area (e.g. ${userList})`;
    }
    if ("AreaMissingRoundGroup" in br) {
      const { bid_year, area_code } = br.AreaMissingRoundGroup;
      return `Area ${area_code} (Bid Year ${bid_year}): No round group assigned`;
    }
    if ("RoundGroupHasNoRounds" in br) {
      const { bid_year, round_group_name } = br.RoundGroupHasNoRounds;
      return `Round Group "${round_group_name}" (Bid Year ${bid_year}): No rounds defined`;
    }
  }

  return "Unknown blocking reason";
}

// ============================================================================
// Blocker Link Resolver
// ============================================================================

function getBlockerLink(br: BlockingReason): string | null {
  if (br === "NoActiveBidYear") {
    return "/admin/bootstrap/bid-years";
  }

  if (typeof br === "object") {
    if ("ExpectedAreaCountNotSet" in br) {
      return "/admin/bootstrap/bid-years";
    }
    if ("AreaCountMismatch" in br) {
      return "/admin/bootstrap/areas";
    }
    if ("ExpectedUserCountNotSet" in br) {
      return "/admin/bootstrap/areas";
    }
    if ("UserCountMismatch" in br) {
      return "/admin/bootstrap/users";
    }
    if ("UsersInNoBidArea" in br) {
      return "/admin/bootstrap/no-bid-review";
    }
    if ("AreaMissingRoundGroup" in br) {
      return "/admin/bootstrap/area-round-groups";
    }
    if ("RoundGroupHasNoRounds" in br) {
      return "/admin/bootstrap/round-groups";
    }
  }

  return null;
}
