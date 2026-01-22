// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Readiness Widget component.
 *
 * Displays lifecycle state and readiness status for the active bid year.
 * Shows blocker count and provides link to Readiness Review.
 *
 * This widget is visible on all bootstrap routes and relies exclusively
 * on backend readiness evaluation as the single source of truth.
 */

import { Link } from "react-router-dom";

interface ReadinessWidgetProps {
  lifecycleState: string;
  isReadyForBidding: boolean;
  blockerCount: number;
  hideReviewLink?: boolean;
}

export function ReadinessWidget({
  lifecycleState,
  isReadyForBidding,
  blockerCount,
  hideReviewLink = false,
}: ReadinessWidgetProps) {
  const getLifecycleColor = (state: string): string => {
    switch (state) {
      case "Draft":
        return "lifecycle-draft";
      case "Canonicalized":
        return "lifecycle-canonicalized";
      case "Active":
        return "lifecycle-active";
      case "Closed":
        return "lifecycle-closed";
      default:
        return "lifecycle-draft";
    }
  };

  return (
    <div className="readiness-widget">
      <div className="widget-header">
        <h4 className="widget-title">System Status</h4>
        <span
          className={`lifecycle-badge ${getLifecycleColor(lifecycleState)}`}
        >
          {lifecycleState}
        </span>
      </div>

      <div className="widget-body">
        {isReadyForBidding ? (
          <div className="status-ready">
            <span className="status-icon">✓</span>
            <span className="status-text">Ready for Bidding</span>
          </div>
        ) : (
          <div className="status-incomplete">
            <span className="status-icon">⚠</span>
            <div className="status-content">
              <span className="status-text">Bootstrap Incomplete</span>
              <span className="blocker-count">
                {blockerCount} blocker{blockerCount !== 1 ? "s" : ""}
              </span>
            </div>
          </div>
        )}

        {!isReadyForBidding && !hideReviewLink && (
          <Link to="/admin/bootstrap/readiness" className="review-link">
            Review Blockers →
          </Link>
        )}
      </div>
    </div>
  );
}
