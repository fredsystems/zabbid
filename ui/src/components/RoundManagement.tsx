// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Round Management component.
 *
 * Displays and manages rounds for a specific round group.
 * Supports create, update, and delete operations with lifecycle awareness.
 */

import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  ApiError,
  createRound,
  deleteRound,
  getActiveBidYear,
  listBidYears,
  listRoundGroups,
  listRounds,
  NetworkError,
  updateRound,
} from "../api";

import type {
  BidYearInfo,
  ConnectionState,
  LiveEvent,
  RoundGroupInfo,
  RoundInfo,
} from "../types";

interface RoundManagementProps {
  sessionToken: string;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function RoundManagement({
  sessionToken,
  connectionState: _connectionState,
  lastEvent,
}: RoundManagementProps) {
  const navigate = useNavigate();
  const { roundGroupId } = useParams<{ roundGroupId: string }>();
  const [roundGroupIdNum, setRoundGroupIdNum] = useState<number | null>(null);
  const [roundGroupName, setRoundGroupName] = useState<string | null>(null);
  const [_bidYearId, setBidYearId] = useState<number | null>(null);
  const [bidYear, setBidYear] = useState<number | null>(null);
  const [lifecycleState, setLifecycleState] = useState<string | null>(null);
  const [rounds, setRounds] = useState<RoundInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);

  // Form state for creating
  const [createRoundNumber, setCreateRoundNumber] = useState<number>(1);
  const [createName, setCreateName] = useState("");
  const [createSlotsPerDay, setCreateSlotsPerDay] = useState<number>(1);
  const [createMaxGroups, setCreateMaxGroups] = useState<number>(1);
  const [createMaxTotalHours, setCreateMaxTotalHours] = useState<number>(0);
  const [createIncludeHolidays, setCreateIncludeHolidays] = useState(false);
  const [createAllowOverbid, setCreateAllowOverbid] = useState(false);
  const [creating, setCreating] = useState(false);

  // Form state for editing
  const [editRoundNumber, setEditRoundNumber] = useState<number>(1);
  const [editName, setEditName] = useState("");
  const [editSlotsPerDay, setEditSlotsPerDay] = useState<number>(1);
  const [editMaxGroups, setEditMaxGroups] = useState<number>(1);
  const [editMaxTotalHours, setEditMaxTotalHours] = useState<number>(0);
  const [editIncludeHolidays, setEditIncludeHolidays] = useState(false);
  const [editAllowOverbid, setEditAllowOverbid] = useState(false);
  const [updating, setUpdating] = useState(false);

  // Delete state
  const [deletingId, setDeletingId] = useState<number | null>(null);

  const isCanonicalizedOrLater =
    lifecycleState === "Canonicalized" ||
    lifecycleState === "BiddingActive" ||
    lifecycleState === "BiddingClosed";

  // Parse and validate roundGroupId on mount
  useEffect(() => {
    if (!roundGroupId) {
      setError("Invalid round group ID");
      setLoading(false);
      return;
    }

    const parsed = parseInt(roundGroupId, 10);
    if (Number.isNaN(parsed)) {
      setError("Invalid round group ID");
      setLoading(false);
      return;
    }

    setRoundGroupIdNum(parsed);
  }, [roundGroupId]);

  // Load round group info, bid year, and rounds
  useEffect(() => {
    if (roundGroupIdNum === null) {
      return;
    }

    const loadData = async () => {
      try {
        setLoading(true);
        setError(null);

        const activeBidYearResponse = await getActiveBidYear();

        if (
          activeBidYearResponse.bid_year_id === null ||
          activeBidYearResponse.year === null
        ) {
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

        // Load round groups to find the name
        const roundGroupsResponse = await listRoundGroups(
          sessionToken,
          activeBidYearId,
        );

        const roundGroup = roundGroupsResponse.round_groups.find(
          (rg: RoundGroupInfo) => rg.round_group_id === roundGroupIdNum,
        );

        if (!roundGroup) {
          setError("Round group not found");
          setLoading(false);
          return;
        }

        setRoundGroupName(roundGroup.name);

        // Load rounds for this round group
        const roundsResponse = await listRounds(sessionToken, roundGroupIdNum);
        setRounds(roundsResponse.rounds);
      } catch (err) {
        if (err instanceof NetworkError) {
          setError(
            "Backend is unavailable. Please ensure the server is running.",
          );
        } else {
          setError(
            err instanceof Error ? err.message : "Failed to load rounds",
          );
        }
      } finally {
        setLoading(false);
      }
    };

    void loadData();
  }, [sessionToken, roundGroupIdNum]);

  // Reload on live events
  useEffect(() => {
    if (!lastEvent || roundGroupIdNum === null) {
      return;
    }

    const reloadData = async () => {
      try {
        const roundsResponse = await listRounds(sessionToken, roundGroupIdNum);
        setRounds(roundsResponse.rounds);

        const bidYearsResponse = await listBidYears();
        const activeBidYearInfo = bidYearsResponse.find(
          (by: BidYearInfo) => by.bid_year_id === _bidYearId,
        );
        if (activeBidYearInfo) {
          setLifecycleState(activeBidYearInfo.lifecycle_state);
        }
      } catch (err) {
        console.error("Failed to reload after live event:", err);
      }
    };

    void reloadData();
  }, [lastEvent, sessionToken, roundGroupIdNum, _bidYearId]);

  const handleCreateSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (roundGroupIdNum === null) {
      setError("No round group selected");
      return;
    }

    if (!createName.trim()) {
      setError("Round name is required");
      return;
    }

    if (createRoundNumber < 1) {
      setError("Round number must be positive");
      return;
    }

    try {
      setCreating(true);
      setError(null);

      await createRound(
        sessionToken,
        roundGroupIdNum,
        createRoundNumber,
        createName.trim(),
        createSlotsPerDay,
        createMaxGroups,
        createMaxTotalHours,
        createIncludeHolidays,
        createAllowOverbid,
      );

      // Reload rounds
      const roundsResponse = await listRounds(sessionToken, roundGroupIdNum);
      setRounds(roundsResponse.rounds);

      // Reset form
      setCreateRoundNumber(1);
      setCreateName("");
      setCreateSlotsPerDay(1);
      setCreateMaxGroups(1);
      setCreateMaxTotalHours(0);
      setCreateIncludeHolidays(false);
      setCreateAllowOverbid(false);
      setShowCreateForm(false);
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
      } else {
        setError(err instanceof Error ? err.message : "Failed to create round");
      }
    } finally {
      setCreating(false);
    }
  };

  const handleEditClick = (round: RoundInfo) => {
    setEditingId(round.round_id);
    setEditRoundNumber(round.round_number);
    setEditName(round.name);
    setEditSlotsPerDay(round.slots_per_day);
    setEditMaxGroups(round.max_groups);
    setEditMaxTotalHours(round.max_total_hours);
    setEditIncludeHolidays(round.include_holidays);
    setEditAllowOverbid(round.allow_overbid);
  };

  const handleEditCancel = () => {
    setEditingId(null);
    setEditRoundNumber(1);
    setEditName("");
    setEditSlotsPerDay(1);
    setEditMaxGroups(1);
    setEditMaxTotalHours(0);
    setEditIncludeHolidays(false);
    setEditAllowOverbid(false);
  };

  const handleEditSubmit = async (e: React.FormEvent, roundId: number) => {
    e.preventDefault();

    if (roundGroupIdNum === null) {
      setError("No round group selected");
      return;
    }

    if (!editName.trim()) {
      setError("Round name is required");
      return;
    }

    if (editRoundNumber < 1) {
      setError("Round number must be positive");
      return;
    }

    try {
      setUpdating(true);
      setError(null);

      await updateRound(
        sessionToken,
        roundId,
        roundGroupIdNum,
        editRoundNumber,
        editName.trim(),
        editSlotsPerDay,
        editMaxGroups,
        editMaxTotalHours,
        editIncludeHolidays,
        editAllowOverbid,
      );

      // Reload rounds
      const roundsResponse = await listRounds(sessionToken, roundGroupIdNum);
      setRounds(roundsResponse.rounds);

      // Reset edit state
      handleEditCancel();
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
      } else {
        setError(err instanceof Error ? err.message : "Failed to update round");
      }
    } finally {
      setUpdating(false);
    }
  };

  const handleDeleteClick = async (roundId: number) => {
    if (
      !window.confirm(
        "Are you sure you want to delete this round? This action cannot be undone.",
      )
    ) {
      return;
    }

    if (roundGroupIdNum === null) {
      setError("No round group selected");
      return;
    }

    try {
      setDeletingId(roundId);
      setError(null);

      await deleteRound(sessionToken, roundId);

      // Reload rounds
      const roundsResponse = await listRounds(sessionToken, roundGroupIdNum);
      setRounds(roundsResponse.rounds);
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
      } else {
        setError(err instanceof Error ? err.message : "Failed to delete round");
      }
    } finally {
      setDeletingId(null);
    }
  };

  const handleBackClick = () => {
    navigate("/admin/round-groups");
  };

  const getLifecycleBadgeClass = () => {
    switch (lifecycleState) {
      case "Active":
        return "active";
      case "Canonicalized":
        return "canonicalized";
      case "BiddingActive":
        return "bidding-active";
      case "BiddingClosed":
        return "bidding-closed";
      default:
        return "active";
    }
  };

  const formatLifecycleState = (state: string | null) => {
    if (!state) return "Unknown";
    if (state === "Canonicalized") return "Ready to Bid (Confirmed)";
    return state.replace(/([A-Z])/g, " $1").trim();
  };

  if (loading) {
    return <div className="loading-message">Loading rounds...</div>;
  }

  if (!roundGroupIdNum || !roundGroupName) {
    return (
      <div className="rounds-container">
        <div className="error-message">{error || "Round group not found."}</div>
      </div>
    );
  }

  return (
    <div className="rounds-container">
      <div className="context-bar">
        <div className="context-info">
          <h3>{roundGroupName}</h3>
          <p>
            Round Group ID: {roundGroupIdNum} • Bid Year: {bidYear}
          </p>
        </div>
        <button type="button" className="btn-back" onClick={handleBackClick}>
          Back to Round Groups
        </button>
      </div>

      <div className="header">
        <div>
          <h2>Rounds</h2>
          <p className="section-description">
            Manage bidding rounds for this round group. Round numbers determine
            bidding sequence.
            {isCanonicalizedOrLater && (
              <strong>
                {" "}
                Note: Modifications are blocked after Ready to Bid (Confirmed).
              </strong>
            )}
          </p>
          {lifecycleState && (
            <span className={`lifecycle-badge ${getLifecycleBadgeClass()}`}>
              {formatLifecycleState(lifecycleState)}
            </span>
          )}
        </div>
      </div>

      {error && <div className="error-message">{error}</div>}

      {!showCreateForm && !isCanonicalizedOrLater && (
        <button
          type="button"
          className="btn-primary"
          onClick={() => setShowCreateForm(true)}
        >
          Create Round
        </button>
      )}

      {showCreateForm && (
        <form onSubmit={handleCreateSubmit} className="create-form">
          <h3>Create New Round</h3>
          <div className="form-grid">
            <div className="form-group">
              <label htmlFor="createRoundNumber">Round Number (Sequence)</label>
              <input
                type="number"
                id="createRoundNumber"
                value={createRoundNumber}
                onChange={(e) =>
                  setCreateRoundNumber(parseInt(e.target.value, 10) || 0)
                }
                min="1"
                required
                disabled={creating}
              />
              <div className="help-text">
                This determines the bidding order within the round group.
              </div>
            </div>
            <div className="form-group">
              <label htmlFor="createName">Round Name</label>
              <input
                type="text"
                id="createName"
                value={createName}
                onChange={(e) => setCreateName(e.target.value)}
                required
                disabled={creating}
              />
            </div>
            <div className="form-group">
              <label htmlFor="createSlotsPerDay">Slots Per Day</label>
              <input
                type="number"
                id="createSlotsPerDay"
                value={createSlotsPerDay}
                onChange={(e) =>
                  setCreateSlotsPerDay(parseInt(e.target.value, 10) || 0)
                }
                min="0"
                required
                disabled={creating}
              />
            </div>
            <div className="form-group">
              <label htmlFor="createMaxGroups">Max Groups</label>
              <input
                type="number"
                id="createMaxGroups"
                value={createMaxGroups}
                onChange={(e) =>
                  setCreateMaxGroups(parseInt(e.target.value, 10) || 0)
                }
                min="0"
                required
                disabled={creating}
              />
            </div>
            <div className="form-group">
              <label htmlFor="createMaxTotalHours">Max Total Hours</label>
              <input
                type="number"
                id="createMaxTotalHours"
                value={createMaxTotalHours}
                onChange={(e) =>
                  setCreateMaxTotalHours(parseInt(e.target.value, 10) || 0)
                }
                min="0"
                required
                disabled={creating}
              />
            </div>
            <div className="form-group">
              <div className="checkbox-wrapper">
                <input
                  type="checkbox"
                  id="createIncludeHolidays"
                  checked={createIncludeHolidays}
                  onChange={(e) => setCreateIncludeHolidays(e.target.checked)}
                  disabled={creating}
                />
                <label htmlFor="createIncludeHolidays">Include Holidays</label>
              </div>
            </div>
            <div className="form-group">
              <div className="checkbox-wrapper">
                <input
                  type="checkbox"
                  id="createAllowOverbid"
                  checked={createAllowOverbid}
                  onChange={(e) => setCreateAllowOverbid(e.target.checked)}
                  disabled={creating}
                />
                <label htmlFor="createAllowOverbid">Allow Overbid</label>
              </div>
            </div>
          </div>
          <div className="form-actions">
            <button type="submit" className="btn-save" disabled={creating}>
              {creating ? "Creating..." : "Create"}
            </button>
            <button
              type="button"
              className="btn-secondary"
              onClick={() => {
                setShowCreateForm(false);
                setCreateRoundNumber(1);
                setCreateName("");
                setCreateSlotsPerDay(1);
                setCreateMaxGroups(1);
                setCreateMaxTotalHours(0);
                setCreateIncludeHolidays(false);
                setCreateAllowOverbid(false);
              }}
              disabled={creating}
            >
              Cancel
            </button>
          </div>
        </form>
      )}

      {rounds.length === 0 ? (
        <div className="empty-state">
          No rounds defined for this group. Create one above to get started.
        </div>
      ) : (
        <div className="rounds-list">
          {rounds
            .sort((a, b) => a.round_number - b.round_number)
            .map((round) => (
              <div key={round.round_id} className="round-card">
                <div className="card-header">
                  <div className="round-title">
                    <h4>{round.name}</h4>
                    <div className="round-number">
                      Round #{round.round_number}
                    </div>
                  </div>
                  <div className="header-actions">
                    {!isCanonicalizedOrLater &&
                      editingId !== round.round_id && (
                        <>
                          <button
                            type="button"
                            className="btn-edit"
                            onClick={() => handleEditClick(round)}
                          >
                            Edit
                          </button>
                          <button
                            type="button"
                            className="btn-danger"
                            onClick={() => handleDeleteClick(round.round_id)}
                            disabled={deletingId === round.round_id}
                          >
                            {deletingId === round.round_id
                              ? "Deleting..."
                              : "Delete"}
                          </button>
                        </>
                      )}
                  </div>
                </div>

                {editingId === round.round_id ? (
                  <form
                    onSubmit={(e) => handleEditSubmit(e, round.round_id)}
                    className="edit-form"
                  >
                    <div className="form-grid">
                      <div className="form-group">
                        <label htmlFor={`editRoundNumber-${round.round_id}`}>
                          Round Number (Sequence)
                        </label>
                        <input
                          type="number"
                          id={`editRoundNumber-${round.round_id}`}
                          value={editRoundNumber}
                          onChange={(e) =>
                            setEditRoundNumber(
                              parseInt(e.target.value, 10) || 0,
                            )
                          }
                          min="1"
                          required
                          disabled={updating}
                        />
                        <div className="help-text">
                          Changing this reorders the bidding sequence.
                        </div>
                      </div>
                      <div className="form-group">
                        <label htmlFor={`editName-${round.round_id}`}>
                          Round Name
                        </label>
                        <input
                          type="text"
                          id={`editName-${round.round_id}`}
                          value={editName}
                          onChange={(e) => setEditName(e.target.value)}
                          required
                          disabled={updating}
                        />
                      </div>
                      <div className="form-group">
                        <label htmlFor={`editSlotsPerDay-${round.round_id}`}>
                          Slots Per Day
                        </label>
                        <input
                          type="number"
                          id={`editSlotsPerDay-${round.round_id}`}
                          value={editSlotsPerDay}
                          onChange={(e) =>
                            setEditSlotsPerDay(
                              parseInt(e.target.value, 10) || 0,
                            )
                          }
                          min="0"
                          required
                          disabled={updating}
                        />
                      </div>
                      <div className="form-group">
                        <label htmlFor={`editMaxGroups-${round.round_id}`}>
                          Max Groups
                        </label>
                        <input
                          type="number"
                          id={`editMaxGroups-${round.round_id}`}
                          value={editMaxGroups}
                          onChange={(e) =>
                            setEditMaxGroups(parseInt(e.target.value, 10) || 0)
                          }
                          min="0"
                          required
                          disabled={updating}
                        />
                      </div>
                      <div className="form-group">
                        <label htmlFor={`editMaxTotalHours-${round.round_id}`}>
                          Max Total Hours
                        </label>
                        <input
                          type="number"
                          id={`editMaxTotalHours-${round.round_id}`}
                          value={editMaxTotalHours}
                          onChange={(e) =>
                            setEditMaxTotalHours(
                              parseInt(e.target.value, 10) || 0,
                            )
                          }
                          min="0"
                          required
                          disabled={updating}
                        />
                      </div>
                      <div className="form-group">
                        <div className="checkbox-wrapper">
                          <input
                            type="checkbox"
                            id={`editIncludeHolidays-${round.round_id}`}
                            checked={editIncludeHolidays}
                            onChange={(e) =>
                              setEditIncludeHolidays(e.target.checked)
                            }
                            disabled={updating}
                          />
                          <label
                            htmlFor={`editIncludeHolidays-${round.round_id}`}
                          >
                            Include Holidays
                          </label>
                        </div>
                      </div>
                      <div className="form-group">
                        <div className="checkbox-wrapper">
                          <input
                            type="checkbox"
                            id={`editAllowOverbid-${round.round_id}`}
                            checked={editAllowOverbid}
                            onChange={(e) =>
                              setEditAllowOverbid(e.target.checked)
                            }
                            disabled={updating}
                          />
                          <label htmlFor={`editAllowOverbid-${round.round_id}`}>
                            Allow Overbid
                          </label>
                        </div>
                      </div>
                    </div>
                    <div className="form-actions">
                      <button
                        type="submit"
                        className="btn-save"
                        disabled={updating}
                      >
                        {updating ? "Saving..." : "Save"}
                      </button>
                      <button
                        type="button"
                        className="btn-secondary"
                        onClick={handleEditCancel}
                        disabled={updating}
                      >
                        Cancel
                      </button>
                    </div>
                  </form>
                ) : (
                  <div className="card-body">
                    <dl>
                      <dt>Round ID:</dt>
                      <dd>{round.round_id}</dd>
                      <dt>Slots Per Day:</dt>
                      <dd>{round.slots_per_day}</dd>
                      <dt>Max Groups:</dt>
                      <dd>{round.max_groups}</dd>
                      <dt>Max Total Hours:</dt>
                      <dd>{round.max_total_hours}</dd>
                      <dt>Include Holidays:</dt>
                      <dd>{round.include_holidays ? "Yes" : "No"}</dd>
                      <dt>Allow Overbid:</dt>
                      <dd>{round.allow_overbid ? "Yes" : "No"}</dd>
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
