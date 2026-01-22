// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Bootstrap Navigation component.
 *
 * Displays a step-by-step navigation for the bootstrap workflow.
 * Shows completion status and allows navigation to any step.
 *
 * Navigation is never hard-blocked; operators can move between steps freely.
 * Only the final confirmation action (Ready to Bid) is gated.
 */

import { NavLink } from "react-router-dom";
import styles from "../styles/bootstrap-navigation.module.scss";

interface BootstrapNavigationProps {
  currentStep?: string;
}

interface NavStep {
  id: string;
  label: string;
  path: string;
}

const WORKFLOW_STEPS: NavStep[] = [
  { id: "bid-years", label: "Bid Years", path: "/admin/bootstrap/bid-years" },
  { id: "areas", label: "Areas", path: "/admin/bootstrap/areas" },
  { id: "users", label: "Users", path: "/admin/bootstrap/users" },
  {
    id: "no-bid-review",
    label: "No Bid Review",
    path: "/admin/bootstrap/no-bid-review",
  },
  {
    id: "round-groups",
    label: "Round Groups",
    path: "/admin/bootstrap/round-groups",
  },
  {
    id: "area-round-groups",
    label: "Area Assignments",
    path: "/admin/bootstrap/area-round-groups",
  },
  { id: "schedule", label: "Bid Schedule", path: "/admin/bootstrap/schedule" },
  {
    id: "readiness",
    label: "Readiness Review",
    path: "/admin/bootstrap/readiness",
  },
];

export function BootstrapNavigation({ currentStep }: BootstrapNavigationProps) {
  return (
    <nav className={styles.bootstrapNavigation}>
      <h3 className={styles.navTitle}>Bootstrap Workflow</h3>
      <ol className={styles.navSteps}>
        {WORKFLOW_STEPS.map((step, index) => (
          <li
            key={step.id}
            className={`${styles.navStep} ${currentStep === step.id ? styles.active : ""}`}
          >
            <NavLink
              to={step.path}
              className={({ isActive }) =>
                isActive ? styles.stepLinkActive : styles.stepLink
              }
            >
              <span className={styles.stepNumber}>{index + 1}</span>
              <span className={styles.stepLabel}>{step.label}</span>
            </NavLink>
          </li>
        ))}
      </ol>
    </nav>
  );
}
