// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Round Group Setup Wrapper component.
 *
 * Fifth step in the bootstrap workflow.
 * Shows round groups and rounds inline with card-based layout.
 *
 * Purpose: Configure round groups and rounds.
 *
 * Functionality:
 * - List round groups with card layout
 * - Create/edit/delete round groups
 * - Show rounds per group inline (expandable cards)
 * - Create/edit/delete rounds within each group
 *
 * Completion criteria:
 * - At least one round group exists
 * - Each round group has at least one round defined
 */

import { useCallback, useEffect, useState } from "react";
import {
  ApiError,
  createRound,
  createRoundGroup,
  getActiveBidYear,
  getBootstrapCompleteness,
  listBidYears,
  listRoundGroups,
  listRounds,
  NetworkError,
  updateRound,
  updateRoundGroup,
} from "../api";
import type {
  BidYearInfo,
  ConnectionState,
  GetBootstrapCompletenessResponse,
  GlobalCapabilities,
  LiveEvent,
  RoundGroupInfo,
  RoundInfo,
} from "../types";
import { BootstrapNavigation } from "./BootstrapNavigation";
import { ReadinessWidget } from "./ReadinessWidget";

interface RoundGroupSetupWrapperProps {
  sessionToken: string | null;
  capabilities: GlobalCapabilities | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function RoundGroupSetupWrapper({
  sessionToken,
  capabilities,
  connectionState,
  lastEvent,
}: RoundGroupSetupWrapperProps) {
  const [completeness, setCompleteness] =
    useState<GetBootstrapCompletenessResponse | null>(null);
  const [bidYearId, setBidYearId] = useState<number | null>(null);
  const [lifecycleState, setLifecycleState] = useState<string | null>(null);
  const [roundGroups, setRoundGroups] = useState<RoundGroupInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const isAdmin = capabilities?.can_create_bid_year ?? false;

  const loadData = useCallback(async () => {
    if (!sessionToken) {
      setLoading(false);
      return;
    }

    try {
      setLoading(true);
      setError(null);

      const [completenessResponse, activeBidYearResponse] = await Promise.all([
        getBootstrapCompleteness(),
        getActiveBidYear(),
      ]);

      setCompleteness(completenessResponse);

      if (
        activeBidYearResponse.bid_year_id === null ||
        activeBidYearResponse.year === null
      ) {
        setError("No active bid year. Please set one first.");
        setLoading(false);
        return;
      }

      const activeBidYearId = activeBidYearResponse.bid_year_id;
      setBidYearId(activeBidYearId);

      // Get full bid year info including lifecycle state
      const bidYearsResponse = await listBidYears();
      const activeBidYearInfo = bidYearsResponse.find(
        (by: BidYearInfo) => by.bid_year_id === activeBidYearId,
      );

      if (activeBidYearInfo) {
        setLifecycleState(activeBidYearInfo.lifecycle_state);
      }

      // Load round groups
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
          err instanceof Error
            ? err.message
            : "Failed to load round group setup data",
        );
      }
    } finally {
      setLoading(false);
    }
  }, [sessionToken]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  useEffect(() => {
    if (connectionState === "connected") {
      void loadData();
    }
  }, [connectionState, loadData]);

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
      void loadData();
    }
  }, [lastEvent, loadData]);

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
        </section>

        {roundGroups.length === 0 && isAdmin && bidYearId !== null && (
          <section className="bootstrap-section">
            <CreateRoundGroupForm
              bidYearId={bidYearId}
              sessionToken={sessionToken}
              onSuccess={loadData}
              onError={setError}
            />
          </section>
        )}

        {roundGroups.length > 0 && (
          <section className="bootstrap-section">
            <h3 className="section-title">Round Groups</h3>
            <div className="round-groups-list">
              {roundGroups.map((rg) => (
                <RoundGroupCard
                  key={rg.round_group_id}
                  roundGroup={rg}
                  isAdmin={isAdmin}
                  sessionToken={sessionToken}
                  lifecycleState={lifecycleState}
                  onRefresh={loadData}
                  onError={setError}
                />
              ))}
            </div>

            {isAdmin && bidYearId !== null && (
              <CreateRoundGroupForm
                bidYearId={bidYearId}
                sessionToken={sessionToken}
                onSuccess={loadData}
                onError={setError}
              />
            )}
          </section>
        )}

        {error && (
          <section className="bootstrap-section">
            <div className="error-banner">
              <strong>Error:</strong> {error}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Round Group Card Component
// ============================================================================

interface RoundGroupCardProps {
  roundGroup: RoundGroupInfo;
  isAdmin: boolean;
  sessionToken: string | null;
  lifecycleState: string | null;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function RoundGroupCard({
  roundGroup,
  isAdmin,
  sessionToken,
  lifecycleState,
  onRefresh,
  onError,
}: RoundGroupCardProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);
  const [rounds, setRounds] = useState<RoundInfo[]>([]);
  const [loadingRounds, setLoadingRounds] = useState(false);

  const isCanonicalizedOrLater =
    lifecycleState === "Canonicalized" ||
    lifecycleState === "BiddingActive" ||
    lifecycleState === "BiddingClosed";

  const loadRounds = useCallback(async () => {
    if (!sessionToken) return;

    try {
      setLoadingRounds(true);
      const roundsResponse = await listRounds(
        sessionToken,
        roundGroup.round_group_id,
      );
      setRounds(roundsResponse.rounds);
    } catch (err) {
      console.error("Failed to load rounds:", err);
      setRounds([]);
    } finally {
      setLoadingRounds(false);
    }
  }, [sessionToken, roundGroup.round_group_id]);

  useEffect(() => {
    if (isExpanded) {
      void loadRounds();
    }
  }, [isExpanded, loadRounds]);

  const handleToggleExpand = () => {
    setIsExpanded(!isExpanded);
  };

  if (isEditing) {
    return (
      <EditRoundGroupForm
        roundGroup={roundGroup}
        sessionToken={sessionToken}
        onSuccess={async () => {
          setIsEditing(false);
          await onRefresh();
        }}
        onCancel={() => setIsEditing(false)}
        onError={onError}
      />
    );
  }

  return (
    <div className="round-group-card">
      <div className="card-header">
        <div className="card-title-group">
          <h4>{roundGroup.name}</h4>
          <div className="card-meta">
            {roundGroup.editing_enabled && (
              <span className="meta-badge editing-enabled">
                Editing Enabled
              </span>
            )}
            <span className="meta-info">
              {rounds.length > 0
                ? `${rounds.length} round${rounds.length !== 1 ? "s" : ""}`
                : isExpanded
                  ? "No rounds"
                  : "Loading..."}
            </span>
          </div>
        </div>
        <div className="card-actions">
          <button
            type="button"
            onClick={handleToggleExpand}
            className="btn-expand"
          >
            {isExpanded ? "Hide Rounds" : "Show Rounds"}
          </button>
          {isAdmin && !isCanonicalizedOrLater && (
            <button
              type="button"
              onClick={() => setIsEditing(true)}
              className="btn-edit"
            >
              Edit
            </button>
          )}
        </div>
      </div>

      {isExpanded && (
        <div className="card-body">
          {loadingRounds ? (
            <div className="loading-text">Loading rounds...</div>
          ) : (
            <RoundsSection
              roundGroupId={roundGroup.round_group_id}
              rounds={rounds}
              isAdmin={isAdmin}
              sessionToken={sessionToken}
              lifecycleState={lifecycleState}
              onRefresh={async () => {
                await loadRounds();
                await onRefresh();
              }}
              onError={onError}
            />
          )}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Rounds Section Component
// ============================================================================

interface RoundsSectionProps {
  roundGroupId: number;
  rounds: RoundInfo[];
  isAdmin: boolean;
  sessionToken: string | null;
  lifecycleState: string | null;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function RoundsSection({
  roundGroupId,
  rounds,
  isAdmin,
  sessionToken,
  lifecycleState,
  onRefresh,
  onError,
}: RoundsSectionProps) {
  const [showCreateForm, setShowCreateForm] = useState(false);

  const isCanonicalizedOrLater =
    lifecycleState === "Canonicalized" ||
    lifecycleState === "BiddingActive" ||
    lifecycleState === "BiddingClosed";

  return (
    <div className="rounds-section">
      <div className="rounds-header">
        <h5>Rounds</h5>
        {isAdmin && !isCanonicalizedOrLater && (
          <button
            type="button"
            onClick={() => setShowCreateForm(!showCreateForm)}
            className="btn-create-small"
          >
            {showCreateForm ? "Cancel" : "Add Round"}
          </button>
        )}
      </div>

      {showCreateForm && (
        <CreateRoundForm
          roundGroupId={roundGroupId}
          sessionToken={sessionToken}
          onSuccess={async () => {
            setShowCreateForm(false);
            await onRefresh();
          }}
          onCancel={() => setShowCreateForm(false)}
          onError={onError}
        />
      )}

      {rounds.length === 0 && !showCreateForm && (
        <p className="empty-state">
          No rounds configured. Add rounds to complete this round group.
        </p>
      )}

      {rounds.length > 0 && (
        <div className="rounds-list">
          {rounds.map((round) => (
            <RoundCard
              key={round.round_id}
              round={round}
              isAdmin={isAdmin}
              sessionToken={sessionToken}
              lifecycleState={lifecycleState}
              onRefresh={onRefresh}
              onError={onError}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Round Card Component
// ============================================================================

interface RoundCardProps {
  round: RoundInfo;
  isAdmin: boolean;
  sessionToken: string | null;
  lifecycleState: string | null;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function RoundCard({
  round,
  isAdmin,
  sessionToken,
  lifecycleState,
  onRefresh,
  onError,
}: RoundCardProps) {
  const [isEditing, setIsEditing] = useState(false);

  const isCanonicalizedOrLater =
    lifecycleState === "Canonicalized" ||
    lifecycleState === "BiddingActive" ||
    lifecycleState === "BiddingClosed";

  if (isEditing) {
    return (
      <EditRoundForm
        round={round}
        sessionToken={sessionToken}
        onSuccess={async () => {
          setIsEditing(false);
          await onRefresh();
        }}
        onCancel={() => setIsEditing(false)}
        onError={onError}
      />
    );
  }

  return (
    <div className="round-card">
      <div className="round-header">
        <h6>
          Round {round.round_number}: {round.name}
        </h6>
        {isAdmin && !isCanonicalizedOrLater && (
          <button
            type="button"
            onClick={() => setIsEditing(true)}
            className="btn-edit-small"
          >
            Edit
          </button>
        )}
      </div>
      <div className="round-details">
        <dl>
          <dt>Slots per Day:</dt>
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
    </div>
  );
}

// ============================================================================
// Create Round Group Form
// ============================================================================

interface CreateRoundGroupFormProps {
  bidYearId: number;
  sessionToken: string | null;
  onSuccess: () => void;
  onError: (error: string) => void;
}

function CreateRoundGroupForm({
  bidYearId,
  sessionToken,
  onSuccess,
  onError,
}: CreateRoundGroupFormProps) {
  const [name, setName] = useState("");
  const [editingEnabled, setEditingEnabled] = useState(true);
  const [creating, setCreating] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!sessionToken) {
      onError("Session token missing");
      return;
    }

    if (!name.trim()) {
      onError("Round group name is required");
      return;
    }

    try {
      setCreating(true);
      onError("");
      await createRoundGroup(
        sessionToken,
        bidYearId,
        name.trim(),
        editingEnabled,
      );
      setName("");
      setEditingEnabled(true);
      onSuccess();
    } catch (err) {
      if (err instanceof ApiError) {
        onError(err.message);
      } else {
        onError("Failed to create round group");
      }
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="create-form">
      <h4>Create Round Group</h4>
      <form onSubmit={handleSubmit}>
        <div className="form-row">
          <label htmlFor="create-rg-name">Name:</label>
          <input
            id="create-rg-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={creating}
            placeholder="Round group name"
          />
        </div>
        <div className="checkbox-wrapper">
          <input
            type="checkbox"
            id="create-rg-editing-enabled"
            checked={editingEnabled}
            onChange={(e) => setEditingEnabled(e.target.checked)}
            disabled={creating}
          />
          <label htmlFor="create-rg-editing-enabled">
            Editing Enabled
            <span className="help-text">
              (Controls whether users can edit their bids during this round
              group's rounds)
            </span>
          </label>
        </div>
        <div className="form-actions">
          <button type="submit" disabled={creating} className="btn-create">
            {creating ? "Creating..." : "Create Round Group"}
          </button>
        </div>
      </form>
    </div>
  );
}

// ============================================================================
// Edit Round Group Form
// ============================================================================

interface EditRoundGroupFormProps {
  roundGroup: RoundGroupInfo;
  sessionToken: string | null;
  onSuccess: () => void;
  onCancel: () => void;
  onError: (error: string) => void;
}

function EditRoundGroupForm({
  roundGroup,
  sessionToken,
  onSuccess,
  onCancel,
  onError,
}: EditRoundGroupFormProps) {
  const [name, setName] = useState(roundGroup.name);
  const [editingEnabled, setEditingEnabled] = useState(
    roundGroup.editing_enabled,
  );
  const [updating, setUpdating] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!sessionToken) {
      onError("Session token missing");
      return;
    }

    if (!name.trim()) {
      onError("Round group name is required");
      return;
    }

    try {
      setUpdating(true);
      onError("");
      await updateRoundGroup(
        sessionToken,
        roundGroup.round_group_id,
        name.trim(),
        editingEnabled,
      );
      onSuccess();
    } catch (err) {
      if (err instanceof ApiError) {
        onError(err.message);
      } else {
        onError("Failed to update round group");
      }
    } finally {
      setUpdating(false);
    }
  };

  return (
    <div className="round-group-card edit-mode">
      <form onSubmit={handleSubmit}>
        <div className="form-row">
          <label htmlFor={`editName-${roundGroup.round_group_id}`}>Name:</label>
          <input
            id={`editName-${roundGroup.round_group_id}`}
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={updating}
          />
        </div>
        <div className="checkbox-wrapper">
          <input
            type="checkbox"
            id={`edit-rg-editing-${roundGroup.round_group_id}`}
            checked={editingEnabled}
            onChange={(e) => setEditingEnabled(e.target.checked)}
            disabled={updating}
          />
          <label htmlFor={`edit-rg-editing-${roundGroup.round_group_id}`}>
            Editing Enabled
            <span className="help-text">
              (Controls whether users can edit their bids during this round
              group's rounds)
            </span>
          </label>
        </div>
        <div className="form-actions">
          <button type="submit" disabled={updating} className="btn-save">
            {updating ? "Saving..." : "Save"}
          </button>
          <button
            type="button"
            onClick={onCancel}
            disabled={updating}
            className="btn-cancel"
          >
            Cancel
          </button>
        </div>
      </form>
    </div>
  );
}

// ============================================================================
// Create Round Form
// ============================================================================

interface CreateRoundFormProps {
  roundGroupId: number;
  sessionToken: string | null;
  onSuccess: () => void;
  onCancel: () => void;
  onError: (error: string) => void;
}

function CreateRoundForm({
  roundGroupId,
  sessionToken,
  onSuccess,
  onCancel,
  onError,
}: CreateRoundFormProps) {
  const [roundNumber, setRoundNumber] = useState("1");
  const [name, setName] = useState("");
  const [slotsPerDay, setSlotsPerDay] = useState("5");
  const [maxGroups, setMaxGroups] = useState("1");
  const [maxTotalHours, setMaxTotalHours] = useState("80");
  const [includeHolidays, setIncludeHolidays] = useState(false);
  const [allowOverbid, setAllowOverbid] = useState(false);
  const [creating, setCreating] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!sessionToken) {
      onError("Session token missing");
      return;
    }

    const roundNum = Number.parseInt(roundNumber, 10);
    const slotsNum = Number.parseInt(slotsPerDay, 10);
    const maxGroupsNum = Number.parseInt(maxGroups, 10);
    const maxHoursNum = Number.parseInt(maxTotalHours, 10);

    if (Number.isNaN(roundNum) || roundNum <= 0) {
      onError("Round number must be a positive number");
      return;
    }

    if (!name.trim()) {
      onError("Round name is required");
      return;
    }

    if (Number.isNaN(slotsNum) || slotsNum <= 0) {
      onError("Slots per day must be a positive number");
      return;
    }

    if (Number.isNaN(maxGroupsNum) || maxGroupsNum <= 0) {
      onError("Max groups must be a positive number");
      return;
    }

    if (Number.isNaN(maxHoursNum) || maxHoursNum <= 0) {
      onError("Max total hours must be a positive number");
      return;
    }

    try {
      setCreating(true);
      onError("");
      await createRound(
        sessionToken,
        roundGroupId,
        roundNum,
        name.trim(),
        slotsNum,
        maxGroupsNum,
        maxHoursNum,
        includeHolidays,
        allowOverbid,
      );
      setRoundNumber("1");
      setName("");
      setSlotsPerDay("5");
      setMaxGroups("1");
      setMaxTotalHours("80");
      setIncludeHolidays(false);
      setAllowOverbid(false);
      onSuccess();
    } catch (err) {
      if (err instanceof ApiError) {
        onError(err.message);
      } else {
        onError("Failed to create round");
      }
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="create-round-form">
      <form onSubmit={handleSubmit}>
        <div className="form-row">
          <label htmlFor="create-round-number">Round Number:</label>
          <input
            id="create-round-number"
            type="number"
            min="1"
            value={roundNumber}
            onChange={(e) => setRoundNumber(e.target.value)}
            disabled={creating}
          />
        </div>
        <div className="form-row">
          <label htmlFor="create-round-name">Round Name:</label>
          <input
            id="create-round-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={creating}
            placeholder="Round name"
          />
        </div>
        <div className="form-row">
          <label htmlFor="create-round-slots">Slots per Day:</label>
          <input
            id="create-round-slots"
            type="number"
            min="1"
            value={slotsPerDay}
            onChange={(e) => setSlotsPerDay(e.target.value)}
            disabled={creating}
          />
        </div>
        <div className="form-row">
          <label htmlFor="create-round-max-groups">Max Groups:</label>
          <input
            id="create-round-max-groups"
            type="number"
            min="1"
            value={maxGroups}
            onChange={(e) => setMaxGroups(e.target.value)}
            disabled={creating}
          />
        </div>
        <div className="form-row">
          <label htmlFor="create-round-max-hours">Max Total Hours:</label>
          <input
            id="create-round-max-hours"
            type="number"
            min="1"
            value={maxTotalHours}
            onChange={(e) => setMaxTotalHours(e.target.value)}
            disabled={creating}
          />
        </div>
        <div className="checkbox-wrapper">
          <input
            type="checkbox"
            id="create-round-holidays"
            checked={includeHolidays}
            onChange={(e) => setIncludeHolidays(e.target.checked)}
            disabled={creating}
          />
          <label htmlFor="create-round-holidays">
            Include Holidays
            <span className="help-text">
              (Whether holiday dates count toward bid allocation for this round)
            </span>
          </label>
        </div>
        <div className="checkbox-wrapper">
          <input
            type="checkbox"
            id="create-round-overbid"
            checked={allowOverbid}
            onChange={(e) => setAllowOverbid(e.target.checked)}
            disabled={creating}
          />
          <label htmlFor="create-round-overbid">
            Allow Overbid
            <span className="help-text">
              (Whether users can bid more hours than the maximum allowed)
            </span>
          </label>
        </div>
        <div className="form-actions">
          <button type="submit" disabled={creating} className="btn-save">
            {creating ? "Creating..." : "Add Round"}
          </button>
          <button
            type="button"
            onClick={onCancel}
            disabled={creating}
            className="btn-cancel"
          >
            Cancel
          </button>
        </div>
      </form>
    </div>
  );
}

// ============================================================================
// Edit Round Form
// ============================================================================

interface EditRoundFormProps {
  round: RoundInfo;
  sessionToken: string | null;
  onSuccess: () => void;
  onCancel: () => void;
  onError: (error: string) => void;
}

function EditRoundForm({
  round,
  sessionToken,
  onSuccess,
  onCancel,
  onError,
}: EditRoundFormProps) {
  const [roundNumber, setRoundNumber] = useState(round.round_number.toString());
  const [name, setName] = useState(round.name);
  const [slotsPerDay, setSlotsPerDay] = useState(
    round.slots_per_day.toString(),
  );
  const [maxGroups, setMaxGroups] = useState(round.max_groups.toString());
  const [maxTotalHours, setMaxTotalHours] = useState(
    round.max_total_hours.toString(),
  );
  const [includeHolidays, setIncludeHolidays] = useState(
    round.include_holidays,
  );
  const [allowOverbid, setAllowOverbid] = useState(round.allow_overbid);
  const [updating, setUpdating] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!sessionToken) {
      onError("Session token missing");
      return;
    }

    const roundNum = Number.parseInt(roundNumber, 10);
    const slotsNum = Number.parseInt(slotsPerDay, 10);
    const maxGroupsNum = Number.parseInt(maxGroups, 10);
    const maxHoursNum = Number.parseInt(maxTotalHours, 10);

    if (Number.isNaN(roundNum) || roundNum <= 0) {
      onError("Round number must be a positive number");
      return;
    }

    if (!name.trim()) {
      onError("Round name is required");
      return;
    }

    if (Number.isNaN(slotsNum) || slotsNum <= 0) {
      onError("Slots per day must be a positive number");
      return;
    }

    if (Number.isNaN(maxGroupsNum) || maxGroupsNum <= 0) {
      onError("Max groups must be a positive number");
      return;
    }

    if (Number.isNaN(maxHoursNum) || maxHoursNum <= 0) {
      onError("Max total hours must be a positive number");
      return;
    }

    try {
      setUpdating(true);
      onError("");
      await updateRound(
        sessionToken,
        round.round_id,
        round.round_group_id,
        roundNum,
        name.trim(),
        slotsNum,
        maxGroupsNum,
        maxHoursNum,
        includeHolidays,
        allowOverbid,
      );
      onSuccess();
    } catch (err) {
      if (err instanceof ApiError) {
        onError(err.message);
      } else {
        onError("Failed to update round");
      }
    } finally {
      setUpdating(false);
    }
  };

  return (
    <div className="round-card edit-mode">
      <form onSubmit={handleSubmit}>
        <div className="form-row">
          <label htmlFor={`editRoundNumber-${round.round_id}`}>
            Round Number:
          </label>
          <input
            id={`editRoundNumber-${round.round_id}`}
            type="number"
            min="1"
            value={roundNumber}
            onChange={(e) => setRoundNumber(e.target.value)}
            disabled={updating}
          />
        </div>
        <div className="form-row">
          <label htmlFor={`editRoundName-${round.round_id}`}>Round Name:</label>
          <input
            id={`editRoundName-${round.round_id}`}
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={updating}
          />
        </div>
        <div className="form-row">
          <label htmlFor={`editRoundSlots-${round.round_id}`}>
            Slots per Day:
          </label>
          <input
            id={`editRoundSlots-${round.round_id}`}
            type="number"
            min="1"
            value={slotsPerDay}
            onChange={(e) => setSlotsPerDay(e.target.value)}
            disabled={updating}
          />
        </div>
        <div className="form-row">
          <label htmlFor={`editRoundMaxGroups-${round.round_id}`}>
            Max Groups:
          </label>
          <input
            id={`editRoundMaxGroups-${round.round_id}`}
            type="number"
            min="1"
            value={maxGroups}
            onChange={(e) => setMaxGroups(e.target.value)}
            disabled={updating}
          />
        </div>
        <div className="form-row">
          <label htmlFor={`editRoundMaxHours-${round.round_id}`}>
            Max Total Hours:
          </label>
          <input
            id={`editRoundMaxHours-${round.round_id}`}
            type="number"
            min="1"
            value={maxTotalHours}
            onChange={(e) => setMaxTotalHours(e.target.value)}
            disabled={updating}
          />
        </div>
        <div className="checkbox-wrapper">
          <input
            type="checkbox"
            id={`edit-round-holidays-${round.round_id}`}
            checked={includeHolidays}
            onChange={(e) => setIncludeHolidays(e.target.checked)}
            disabled={updating}
          />
          <label htmlFor={`edit-round-holidays-${round.round_id}`}>
            Include Holidays
            <span className="help-text">
              (Whether holiday dates count toward bid allocation for this round)
            </span>
          </label>
        </div>
        <div className="checkbox-wrapper">
          <input
            type="checkbox"
            id={`edit-round-overbid-${round.round_id}`}
            checked={allowOverbid}
            onChange={(e) => setAllowOverbid(e.target.checked)}
            disabled={updating}
          />
          <label htmlFor={`edit-round-overbid-${round.round_id}`}>
            Allow Overbid
            <span className="help-text">
              (Whether users can bid more hours than the maximum allowed)
            </span>
          </label>
        </div>
        <div className="form-actions">
          <button type="submit" disabled={updating} className="btn-save">
            {updating ? "Saving..." : "Save"}
          </button>
          <button
            type="button"
            onClick={onCancel}
            disabled={updating}
            className="btn-cancel"
          >
            Cancel
          </button>
        </div>
      </form>
    </div>
  );
}
