// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Override Area Modal component.
 *
 * Modal for post-canonicalization area assignment override.
 * Requires:
 * - Area selection (non-system areas only)
 * - Override reason (min 10 characters)
 * - Explicit confirmation
 *
 * This modal is only shown when:
 * - Lifecycle >= Canonicalized
 * - User clicks "Change Area (Override Required)"
 */

import { useEffect, useState } from "react";
import { overrideAreaAssignment } from "../api";
import type { AreaInfo } from "../types";

interface OverrideAreaModalProps {
  /** The session token for authentication */
  sessionToken: string;
  /** The user's canonical ID */
  userId: number;
  /** The user's initials (for display) */
  userInitials: string;
  /** The current area ID */
  currentAreaId: number;
  /** List of available areas (non-system only) */
  availableAreas: AreaInfo[];
  /** Callback when override succeeds */
  onSuccess: () => void;
  /** Callback to cancel/close the modal */
  onCancel: () => void;
}

export function OverrideAreaModal({
  sessionToken,
  userId,
  userInitials,
  currentAreaId,
  availableAreas,
  onSuccess,
  onCancel,
}: OverrideAreaModalProps) {
  const [selectedAreaId, setSelectedAreaId] = useState<number | null>(null);
  const [reason, setReason] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Filter out system areas and current area
  const selectableAreas = availableAreas.filter(
    (area) => !area.is_system_area && area.area_id !== currentAreaId,
  );

  useEffect(() => {
    // Set first selectable area as default
    if (selectableAreas.length > 0 && selectedAreaId === null) {
      setSelectedAreaId(selectableAreas[0]?.area_id ?? null);
    }
  }, [selectableAreas, selectedAreaId]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!selectedAreaId) {
      setError("Please select an area");
      return;
    }

    if (reason.trim().length < 10) {
      setError("Override reason must be at least 10 characters");
      return;
    }

    try {
      setSaving(true);
      setError(null);

      await overrideAreaAssignment(
        sessionToken,
        userId,
        selectedAreaId,
        reason.trim(),
      );

      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to override area");
    } finally {
      setSaving(false);
    }
  };

  const handleCancel = () => {
    if (!saving) {
      onCancel();
    }
  };

  const reasonValid = reason.trim().length >= 10;

  return (
    <button
      type="button"
      className="modal-overlay"
      onClick={handleCancel}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          handleCancel();
        }
      }}
    >
      <div
        className="modal-content"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <div className="modal-header">
          <h3>Override Area Assignment</h3>
          <button
            type="button"
            className="modal-close"
            onClick={handleCancel}
            disabled={saving}
            aria-label="Close"
          >
            ×
          </button>
        </div>

        <div className="modal-body">
          <p className="modal-warning">
            You are about to override the canonical area assignment for user{" "}
            <strong>{userInitials}</strong>. This action creates an audit trail
            and requires justification.
          </p>

          {selectableAreas.length === 0 && (
            <div className="error">
              <p>No operational areas available for assignment.</p>
            </div>
          )}

          {selectableAreas.length > 0 && (
            <form onSubmit={handleSubmit}>
              <div className="form-row">
                <label htmlFor="override-area-select">New Area:</label>
                <select
                  id="override-area-select"
                  value={selectedAreaId ?? ""}
                  onChange={(e) => setSelectedAreaId(Number(e.target.value))}
                  disabled={saving}
                  required
                >
                  {selectableAreas.map((area) => (
                    <option key={area.area_id} value={area.area_id}>
                      {area.area_code}
                      {area.area_name ? ` - ${area.area_name}` : ""}
                    </option>
                  ))}
                </select>
              </div>

              <div className="form-row">
                <label htmlFor="override-reason">
                  Reason (min 10 characters):
                </label>
                <textarea
                  id="override-reason"
                  value={reason}
                  onChange={(e) => setReason(e.target.value)}
                  disabled={saving}
                  required
                  minLength={10}
                  rows={4}
                  placeholder="Explain why this override is necessary..."
                />
                <div className="character-count">
                  {reason.trim().length} / 10 characters
                  {reasonValid && " ✓"}
                </div>
              </div>

              {error && (
                <div className="error-message">
                  <p>{error}</p>
                </div>
              )}

              <div className="modal-actions">
                <button
                  type="button"
                  onClick={handleCancel}
                  disabled={saving}
                  className="btn-cancel"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={saving || !reasonValid || selectedAreaId === null}
                  className="btn-save"
                >
                  {saving ? "Saving..." : "Confirm Override"}
                </button>
              </div>
            </form>
          )}
        </div>
      </div>
    </button>
  );
}
