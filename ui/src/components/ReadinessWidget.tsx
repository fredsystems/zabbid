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
import styles from "../styles/readiness-widget.module.scss";

interface ReadinessWidgetProps {
  lifecycleState: string;
  isReadyForBidding: boolean;
  blockerCount: number;
}

export function ReadinessWidget({
  lifecycleState,
  isReadyForBidding,
  blockerCount,
}: ReadinessWidgetProps) {
  const getLifecycleColor = (state: string): string => {
    switch (state) {
      case "Draft":
        return styles.lifecycleDraft;
      case "Canonicalized":
        return styles.lifecycleCanonicalized;
      case "Active":
        return styles.lifecycleActive;
      case "Closed":
        return styles.lifecycleClosed;
      default:
        return styles.lifecycleDraft;
    }
  };

  return (
    <div className={styles.readinessWidget}>
      <div className={styles.widgetHeader}>
        <h4 className={styles.widgetTitle}>System Status</h4>
        <span
          className={`${styles.lifecycleBadge} ${getLifecycleColor(lifecycleState)}`}
        >
          {lifecycleState}
        </span>
      </div>

      <div className={styles.widgetBody}>
        {isReadyForBidding ? (
          <div className={styles.statusReady}>
            <span className={styles.statusIcon}>✓</span>
            <span className={styles.statusText}>Ready for Bidding</span>
          </div>
        ) : (
          <div className={styles.statusIncomplete}>
            <span className={styles.statusIcon}>⚠</span>
            <div className={styles.statusContent}>
              <span className={styles.statusText}>Bootstrap Incomplete</span>
              <span className={styles.blockerCount}>
                {blockerCount} blocker{blockerCount !== 1 ? "s" : ""}
              </span>
            </div>
          </div>
        )}

        {!isReadyForBidding && (
          <Link to="/admin/bootstrap/readiness" className={styles.reviewLink}>
            Review Blockers →
          </Link>
        )}
      </div>
    </div>
  );
}
