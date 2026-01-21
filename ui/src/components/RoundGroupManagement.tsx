// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Round Group Management component.
 *
 * Displays and manages round groups for the active bid year.
 * Supports create, update, and delete operations with lifecycle awareness.
 */

import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  ApiError,
  NetworkError,
  createRoundGroup,
  deleteRoundGroup,
  getActiveBidYear,
  listBidYears,
  listRoundGroups,
  updateRoundGroup,
} from "../api";
import styles from "../styles/round-groups.module.scss";
import type {
  BidYearInfo,
  ConnectionState,
  LiveEvent,
  RoundGroupInfo,
} from "../types";

interface RoundGroupManagementProps {
  sessionToken: string;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function RoundGroupManagement({
  sessionToken,
  connectionState: _connectionState,
  lastEvent,
}: RoundGroupManagementProps) {
  const navigate = useNavigate();
  const [bidYearId, setBidYearId] = useState<number | null>(null);
  const [bidYear, setBidYear] = useState<number | null>(null);
  const [lifecycleState, setLifecycleState] = useState<string | null>(null);
  const [roundGroups, setRoundGroups] = useState<RoundGroupInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);

  // Form state for creating
  const [createName, setCreateName] = useState("");
  const [createEditingEnabled, setCreateEditingEnabled] = useState(true);
  const [creating, setCreating] = useState(false);

  // Form state for editing
  const [editName, setEditName] = useState("");
  const [editEditingEnabled, setEditEditingEnabled] = useState(true);
  const [updating, setUpdating] = useState(false);

  // Delete state
  const [deletingId, setDeletingId] = useState<number | null>(null);

  const isCanonicalizedOrLater =
    lifecycleState === "Canonicalized" ||
    lifecycleState === "BiddingActive" ||
    lifecycleState === "BiddingClosed";

  // Load active bid year and round groups
  useEffect(() => {
    const loadData = async () => {
      try {
        setLoading(true);
        setError(null);

        const activeBidYearResponse = await getActiveBidYear();

        if (!activeBidYearResponse.bid_year_id || !activeBidYearResponse.year) {
          setError("No active bid year. Please set one first.");
          setLoading(false);
          return;
        }

        const activeBidYearId = activeBidYearResponse.bid_year_id;
        const activeYear = activeBidYearResponse.year;

        // Get full bid year info including lifecycle state
        const bidYearsResponse = await listBidYears();
        const activeBidYearInfo = bidYearsResponse.find(
          (by: BidYearInfo) => by.bid_year_id === activeBidYearId,
        );

        if (!activeBidYearInfo) {
          setError("Active bid year not found in bid years list.");
          setLoading(false);
          return;
        }

        setBidYearId(activeBidYearId);
        setBidYear(activeYear);
        setLifecycleState(activeBidYearInfo.lifecycle_state);

        const roundGroupsResponse = await listRoundGroups(
          sessionToken,
          activeBidYearId,
        );
        setRoundGroups(roundGroupsResponse.round_groups);
      } catch (err) {
        if (err instanceof NetworkError) {
          setError(
            "Backend is unavailable. Please ensure the server is running.",
          );
        } else {
          setError(
            err instanceof Error ? err.message : "Failed to load round groups",
          );
        }
      } finally {
        setLoading(false);
      }
    };

    void loadData();
  }, [sessionToken]);

  // Reload on live events
  useEffect(() => {
    if (!lastEvent || !bidYearId) {
      return;
    }

    const reloadData = async () => {
      try {
        const roundGroupsResponse = await listRoundGroups(
          sessionToken,
          bidYearId,
        );
        setRoundGroups(roundGroupsResponse.round_groups);

        const bidYearsResponse = await listBidYears();
        const activeBidYearInfo = bidYearsResponse.find(
          (by: BidYearInfo) => by.bid_year_id === bidYearId,
        );
        if (activeBidYearInfo) {
          setLifecycleState(activeBidYearInfo.lifecycle_state);
        }
      } catch (err) {
        console.error("Failed to reload after live event:", err);
      }
    };

    void reloadData();
  }, [lastEvent, sessionToken, bidYearId]);

  const handleCreateSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!bidYearId) {
      setError("No active bid year");
      return;
    }

    if (!createName.trim()) {
      setError("Round group name is required");
      return;
    }

    try {
      setCreating(true);
      setError(null);

      await createRoundGroup(
        sessionToken,
        bidYearId,
        createName.trim(),
        createEditingEnabled,
      );

      // Reload round groups
      const roundGroupsResponse = await listRoundGroups(
        sessionToken,
        bidYearId,
      );
      setRoundGroups(roundGroupsResponse.round_groups);

      // Reset form
      setCreateName("");
      setCreateEditingEnabled(true);
      setShowCreateForm(false);
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
      } else {
        setError(
          err instanceof Error ? err.message : "Failed to create round group",
        );
      }
    } finally {
      setCreating(false);
    }
  };

  const handleEditClick = (rg: RoundGroupInfo) => {
    setEditingId(rg.round_group_id);
    setEditName(rg.name);
    setEditEditingEnabled(rg.editing_enabled);
  };

  const handleEditCancel = () => {
    setEditingId(null);
    setEditName("");
    setEditEditingEnabled(true);
  };

  const handleEditSubmit = async (e: React.FormEvent, roundGroupId: number) => {
    e.preventDefault();

    if (!bidYearId) {
      setError("No active bid year");
      return;
    }

    if (!editName.trim()) {
      setError("Round group name is required");
      return;
    }

    try {
      setUpdating(true);
      setError(null);

      await updateRoundGroup(
        sessionToken,
        roundGroupId,
        editName.trim(),
        editEditingEnabled,
      );

      // Reload round groups
      const roundGroupsResponse = await listRoundGroups(
        sessionToken,
        bidYearId,
      );
      setRoundGroups(roundGroupsResponse.round_groups);

      // Reset edit state
      setEditingId(null);
      setEditName("");
      setEditEditingEnabled(true);
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
      } else {
        setError(
          err instanceof Error ? err.message : "Failed to update round group",
        );
      }
    } finally {
      setUpdating(false);
    }
  };

  const handleDeleteClick = async (roundGroupId: number) => {
    if (
      !window.confirm(
        "Are you sure you want to delete this round group? This action cannot be undone.",
      )
    ) {
      return;
    }

    if (!bidYearId) {
      setError("No active bid year");
      return;
    }

    try {
      setDeletingId(roundGroupId);
      setError(null);

      await deleteRoundGroup(sessionToken, roundGroupId);

      // Reload round groups
      const roundGroupsResponse = await listRoundGroups(
        sessionToken,
        bidYearId,
      );
      setRoundGroups(roundGroupsResponse.round_groups);
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
      } else {
        setError(
          err instanceof Error ? err.message : "Failed to delete round group",
        );
      }
    } finally {
      setDeletingId(null);
    }
  };

  const handleManageRoundsClick = (roundGroupId: number) => {
    navigate(`/admin/round-groups/${roundGroupId}/rounds`);
  };

  const getLifecycleBadgeClass = () => {
    switch (lifecycleState) {
      case "Active":
        return styles.active;
      case "Canonicalized":
        return styles.canonicalized;
      case "BiddingActive":
        return styles.biddingActive;
      case "BiddingClosed":
        return styles.biddingClosed;
      default:
        return styles.active;
    }
  };

  const formatLifecycleState = (state: string | null) => {
    if (!state) return "Unknown";
    if (state === "Canonicalized") return "Ready to Bid (Confirmed)";
    return state.replace(/([A-Z])/g, " $1").trim();
  };

  if (loading) {
    return <div className={styles.loadingMessage}>Loading round groups...</div>;
  }

  if (!bidYearId || !bidYear) {
    return (
      <div className={styles.roundGroupsContainer}>
        <div className={styles.errorMessage}>
          {error || "No active bid year. Please set one first."}
        </div>
      </div>
    );
  }

  return (
    <div className={styles.roundGroupsContainer}>
      <div className={styles.header}>
        <div>
          <h2>Round Groups for Bid Year {bidYear}</h2>
          <p className={styles.sectionDescription}>
            Configure bidding round groups for this bid year. Each round group
            defines a collection of bidding rounds.
            {isCanonicalizedOrLater && (
              <strong>
                {" "}
                Note: Modifications are blocked after Ready to Bid (Confirmed).
              </strong>
            )}
          </p>
          {lifecycleState && (
            <span
              className={`${styles.lifecycleBadge} ${getLifecycleBadgeClass()}`}
            >
              {formatLifecycleState(lifecycleState)}
            </span>
          )}
        </div>
      </div>

      {error && <div className={styles.errorMessage}>{error}</div>}

      {!showCreateForm && !isCanonicalizedOrLater && (
        <button
          type="button"
          className={styles.btnPrimary}
          onClick={() => setShowCreateForm(true)}
        >
          Create Round Group
        </button>
      )}

      {showCreateForm && (
        <form onSubmit={handleCreateSubmit} className={styles.createForm}>
          <h3>Create New Round Group</h3>
          <div className={styles.formGroup}>
            <label htmlFor="createName">Name</label>
            <input
              type="text"
              id="createName"
              value={createName}
              onChange={(e) => setCreateName(e.target.value)}
              required
              disabled={creating}
            />
          </div>
          <div className={styles.formGroup}>
            <div className={styles.checkboxWrapper}>
              <input
                type="checkbox"
                id="createEditingEnabled"
                checked={createEditingEnabled}
                onChange={(e) => setCreateEditingEnabled(e.target.checked)}
                disabled={creating}
              />
              <label htmlFor="createEditingEnabled">Editing Enabled</label>
            </div>
          </div>
          <div className={styles.formActions}>
            <button
              type="submit"
              className={styles.btnSave}
              disabled={creating}
            >
              {creating ? "Creating..." : "Create"}
            </button>
            <button
              type="button"
              className={styles.btnSecondary}
              onClick={() => {
                setShowCreateForm(false);
                setCreateName("");
                setCreateEditingEnabled(true);
              }}
              disabled={creating}
            >
              Cancel
            </button>
          </div>
        </form>
      )}

      {roundGroups.length === 0 ? (
        <div className={styles.emptyState}>
          No round groups configured. Create one above to get started.
        </div>
      ) : (
        <div className={styles.roundGroupsList}>
          {roundGroups.map((rg) => (
            <div key={rg.round_group_id} className={styles.roundGroupCard}>
              <div className={styles.cardHeader}>
                <h4>{rg.name}</h4>
                <div className={styles.headerActions}>
                  <button
                    type="button"
                    className={styles.btnPrimary}
                    onClick={() => handleManageRoundsClick(rg.round_group_id)}
                  >
                    Manage Rounds
                  </button>
                  {!isCanonicalizedOrLater &&
                    editingId !== rg.round_group_id && (
                      <>
                        <button
                          type="button"
                          className={styles.btnSecondary}
                          onClick={() => handleEditClick(rg)}
                        >
                          Edit
                        </button>
                        <button
                          type="button"
                          className={styles.btnDanger}
                          onClick={() => handleDeleteClick(rg.round_group_id)}
                          disabled={deletingId === rg.round_group_id}
                        >
                          {deletingId === rg.round_group_id
                            ? "Deleting..."
                            : "Delete"}
                        </button>
                      </>
                    )}
                </div>
              </div>

              {editingId === rg.round_group_id ? (
                <form
                  onSubmit={(e) => handleEditSubmit(e, rg.round_group_id)}
                  className={styles.editForm}
                >
                  <div className={styles.formGroup}>
                    <label htmlFor={`editName-${rg.round_group_id}`}>
                      Name
                    </label>
                    <input
                      type="text"
                      id={`editName-${rg.round_group_id}`}
                      value={editName}
                      onChange={(e) => setEditName(e.target.value)}
                      required
                      disabled={updating}
                    />
                  </div>
                  <div className={styles.formGroup}>
                    <div className={styles.checkboxWrapper}>
                      <input
                        type="checkbox"
                        id={`editEditingEnabled-${rg.round_group_id}`}
                        checked={editEditingEnabled}
                        onChange={(e) =>
                          setEditEditingEnabled(e.target.checked)
                        }
                        disabled={updating}
                      />
                      <label
                        htmlFor={`editEditingEnabled-${rg.round_group_id}`}
                      >
                        Editing Enabled
                      </label>
                    </div>
                  </div>
                  <div className={styles.formActions}>
                    <button
                      type="submit"
                      className={styles.btnSave}
                      disabled={updating}
                    >
                      {updating ? "Saving..." : "Save"}
                    </button>
                    <button
                      type="button"
                      className={styles.btnSecondary}
                      onClick={handleEditCancel}
                      disabled={updating}
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              ) : (
                <div className={styles.cardBody}>
                  <dl>
                    <dt>Round Group ID:</dt>
                    <dd>{rg.round_group_id}</dd>
                    <dt>Bid Year ID:</dt>
                    <dd>{rg.bid_year_id}</dd>
                    <dt>Editing Enabled:</dt>
                    <dd>{rg.editing_enabled ? "Yes" : "No"}</dd>
                  </dl>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
