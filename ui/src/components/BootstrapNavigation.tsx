// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Bootstrap Navigation component.
 *
 * Displays a compact dropdown-style navigation for the bootstrap workflow.
 * Shows current step and allows navigation to any step.
 *
 * Navigation is never hard-blocked; operators can move between steps freely.
 * Only the final confirmation action (Ready to Bid) is gated.
 */

import { useNavigate } from "react-router-dom";

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
  const navigate = useNavigate();
  const currentStepIndex =
    WORKFLOW_STEPS.findIndex((step) => step.id === currentStep) + 1 || 1;

  const handleStepChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    navigate(event.target.value);
  };

  return (
    <nav className="bootstrap-navigation">
      <label htmlFor="bootstrap-step-select" className="nav-label">
        Bootstrap Workflow Step:
      </label>
      <div className="nav-dropdown-container">
        <span className="nav-step-indicator">
          Step {currentStepIndex} of {WORKFLOW_STEPS.length}
        </span>
        <select
          id="bootstrap-step-select"
          className="nav-dropdown"
          value={WORKFLOW_STEPS.find((step) => step.id === currentStep)?.path}
          onChange={handleStepChange}
        >
          {WORKFLOW_STEPS.map((step, index) => (
            <option key={step.id} value={step.path}>
              {index + 1}. {step.label}
            </option>
          ))}
        </select>
      </div>
    </nav>
  );
}
