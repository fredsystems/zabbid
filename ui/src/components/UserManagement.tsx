// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * User Management component.
 *
 * Third step in the bootstrap workflow.
 * Allows admin to populate and configure user roster.
 *
 * Functionality:
 * - CSV import (with preview)
 * - Manual user creation per area
 * - User editing (metadata, area assignment, participation flags)
 * - Show user count per area vs expected
 *
 * Completion criteria:
 * - All non-system areas have actual user count matching expected
 * - Any user-level validation warnings are visible and actionable
 */

import { useCallback, useEffect, useState } from "react";
import {
  ApiError,
  NetworkError,
  getBootstrapCompleteness,
  listAreas,
  listUsers,
  registerUser,
  updateUser,
} from "../api";
import type {
  AreaInfo,
  ConnectionState,
  GetBootstrapCompletenessResponse,
  GlobalCapabilities,
  ListUsersResponse,
  LiveEvent,
  UserInfo,
} from "../types";
import { BootstrapNavigation } from "./BootstrapNavigation";
import { CsvUserImport } from "./CsvUserImport";
import { ReadinessWidget } from "./ReadinessWidget";

interface UserManagementProps {
  sessionToken: string | null;
  capabilities: GlobalCapabilities | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function UserManagement({
  sessionToken,
  capabilities,
  connectionState,
  lastEvent,
}: UserManagementProps) {
  const [completeness, setCompleteness] =
    useState<GetBootstrapCompletenessResponse | null>(null);
  const [areas, setAreas] = useState<AreaInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const isAdmin = capabilities?.can_create_bid_year ?? false;

  const loadCompleteness = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await getBootstrapCompleteness();
      setCompleteness(response);

      // Load actual areas to get is_system_area flag
      if (response.active_bid_year_id !== null) {
        const areasResponse = await listAreas(response.active_bid_year_id);
        setAreas(areasResponse.areas);
      }
    } catch (err) {
      if (err instanceof NetworkError) {
        setError(
          "Backend is unavailable. Please ensure the server is running.",
        );
      } else {
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load bootstrap completeness",
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

    if (
      lastEvent.type === "user_registered" ||
      lastEvent.type === "user_updated" ||
      lastEvent.type === "area_created" ||
      lastEvent.type === "area_updated"
    ) {
      void loadCompleteness();
    }
  }, [lastEvent, loadCompleteness]);

  if (loading) {
    return <div className="loading">Loading user management...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <h2>Unable to Load User Management</h2>
        <p>{error}</p>
      </div>
    );
  }

  if (!completeness) {
    return <div className="error">No completeness data available</div>;
  }

  const activeBidYearInfo = completeness.bid_years.find((by) => by.is_active);
  const lifecycleState = activeBidYearInfo?.lifecycle_state ?? "Draft";
  const activeBidYearId = completeness.active_bid_year_id;

  // Merge AreaInfo with AreaCompletenessInfo to get both is_system_area and expected_user_count
  type MergedAreaInfo = AreaInfo & {
    expected_user_count: number | null;
    bid_year: number;
  };
  const mergedAreas: MergedAreaInfo[] = areas.map((area) => {
    const completenessInfo = completeness.areas.find(
      (a) => a.area_id === area.area_id,
    );
    return {
      ...area,
      expected_user_count: completenessInfo?.expected_user_count ?? null,
      bid_year: completenessInfo?.bid_year ?? 0,
    };
  });

  // Separate system and non-system areas
  const nonSystemAreas = mergedAreas.filter((a) => !a.is_system_area);
  const systemAreas = mergedAreas.filter((a) => a.is_system_area);

  return (
    <div className="bootstrap-completeness">
      <BootstrapNavigation currentStep="users" />
      <ReadinessWidget
        lifecycleState={lifecycleState}
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
          <h2 className="section-title">User Management</h2>
          <p className="section-description">
            Add and configure users for each area. Users can be added manually
            or imported via CSV.
          </p>

          {/* User count blockers would be rendered here if defined in BlockingReason type */}
        </section>

        {/* CSV User Import Section */}
        {isAdmin &&
          sessionToken !== null &&
          activeBidYearId !== null &&
          nonSystemAreas.length > 0 && (
            <section className="bootstrap-section">
              <h3 className="section-title">CSV User Import</h3>
              <p className="section-description">
                Import multiple users at once from CSV data. Select which rows
                to import after validation.
              </p>
              <CsvUserImport
                sessionToken={sessionToken}
                onImportComplete={() => void loadCompleteness()}
              />
            </section>
          )}

        {/* User Management by Area */}
        {activeBidYearId !== null && nonSystemAreas.length > 0 && (
          <section className="bootstrap-section">
            <h3 className="section-title">Users by Area</h3>
            <p className="section-description">
              Manage users for each operational area. Each area shows current
              vs. expected user count.
            </p>
            {nonSystemAreas
              .sort((a, b) => a.area_code.localeCompare(b.area_code))
              .map((area) => (
                <UserManagementForArea
                  key={`users-${area.bid_year}-${area.area_id}`}
                  area={area}
                  isAdmin={isAdmin}
                  sessionToken={sessionToken}
                  onError={setError}
                  onRefresh={loadCompleteness}
                />
              ))}
          </section>
        )}

        {/* System Areas (e.g., No Bid) */}
        {systemAreas.length > 0 && (
          <section className="bootstrap-section">
            <h3 className="section-title">System Areas</h3>
            <p className="section-description">
              System areas like "No Bid" can have users added manually. Users in
              these areas will be reviewed in the next step.
            </p>
            {systemAreas.map((area) => (
              <UserManagementForArea
                key={`users-${area.bid_year}-${area.area_id}`}
                area={area}
                isAdmin={isAdmin}
                sessionToken={sessionToken}
                onError={setError}
                onRefresh={loadCompleteness}
              />
            ))}
          </section>
        )}

        {error && (
          <div className="error-banner">
            <strong>Error:</strong> {error}
          </div>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// User Management for Area Component
// ============================================================================

type MergedAreaInfo = AreaInfo & {
  expected_user_count: number | null;
  bid_year: number;
};

interface UserManagementForAreaProps {
  area: MergedAreaInfo;
  isAdmin: boolean;
  sessionToken: string | null;
  onError: (error: string) => void;
  onRefresh: () => void;
}

// Get all areas for user reassignment dropdown
interface AreaForReassignment {
  area_id: number;
  area_code: string;
  area_name: string | null;
  is_system_area: boolean;
}

function UserManagementForArea({
  area,
  isAdmin,
  sessionToken,
  onError,
  onRefresh,
}: UserManagementForAreaProps) {
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [allAreas, setAllAreas] = useState<AreaForReassignment[]>([]);
  const [loading, setLoading] = useState(true);
  const [showCreateForm, setShowCreateForm] = useState(false);

  // Load all areas for reassignment dropdown
  useEffect(() => {
    const loadAllAreas = async () => {
      if (!sessionToken || !area.bid_year) {
        return;
      }

      try {
        const areasResponse = await listAreas(area.bid_year);
        setAllAreas(areasResponse.areas);
      } catch (err) {
        console.error("Failed to load areas:", err);
        setAllAreas([]);
      }
    };

    void loadAllAreas();
  }, [sessionToken, area.bid_year]);

  useEffect(() => {
    const loadUsers = async () => {
      if (!sessionToken) {
        setUsers([]);
        setLoading(false);
        return;
      }

      try {
        setLoading(true);
        const response: ListUsersResponse = await listUsers(
          sessionToken,
          area.area_id,
        );
        setUsers(response.users);
      } catch (err) {
        console.error("Failed to load users:", err);
        setUsers([]);
      } finally {
        setLoading(false);
      }
    };

    void loadUsers();
  }, [area.area_id, sessionToken]);

  const refreshUsers = async () => {
    if (!sessionToken) {
      setUsers([]);
      return;
    }

    try {
      setLoading(true);
      const response: ListUsersResponse = await listUsers(
        sessionToken,
        area.area_id,
      );
      setUsers(response.users);
    } catch (err) {
      console.error("Failed to load users:", err);
      setUsers([]);
    } finally {
      setLoading(false);
    }
  };

  const countMatch =
    area.expected_user_count !== null &&
    users.length === area.expected_user_count;
  const countMismatch =
    area.expected_user_count !== null &&
    users.length !== area.expected_user_count;

  return (
    <div className="area-user-management">
      <div className="area-user-header">
        <h4>
          {area.area_code}
          {area.area_name && ` - ${area.area_name}`}
        </h4>
        <div className="user-count-info">
          <span className={countMatch ? "count-match" : ""}>
            {users.length} user{users.length !== 1 ? "s" : ""}
          </span>
          {area.expected_user_count !== null && (
            <span className={countMismatch ? "count-mismatch" : ""}>
              {" "}
              / {area.expected_user_count} expected
            </span>
          )}
        </div>
      </div>

      {loading && <p className="loading-text">Loading users...</p>}

      {!loading && users.length === 0 && (
        <p className="empty-state">No users in this area yet.</p>
      )}

      {!loading && users.length > 0 && (
        <div className="users-list">
          {users.map((user) => (
            <UserItem
              key={user.user_id}
              user={user}
              sessionToken={sessionToken}
              isAdmin={isAdmin}
              allAreas={allAreas}
              onRefresh={() => {
                void refreshUsers();
                onRefresh();
              }}
              onError={onError}
            />
          ))}
        </div>
      )}

      {isAdmin && !showCreateForm && (
        <button
          type="button"
          onClick={() => setShowCreateForm(true)}
          className="btn-create"
        >
          + Add User to {area.area_code}
        </button>
      )}

      {isAdmin && showCreateForm && (
        <CreateUserForm
          areaId={area.area_id}
          areaCode={area.area_code}
          sessionToken={sessionToken}
          onSuccess={async () => {
            setShowCreateForm(false);
            await refreshUsers();
            await onRefresh();
          }}
          onCancel={() => setShowCreateForm(false)}
          onError={onError}
        />
      )}
    </div>
  );
}

// ============================================================================
// User Item Component
// ============================================================================

interface UserItemProps {
  user: UserInfo;
  sessionToken: string | null;
  isAdmin: boolean;
  allAreas: AreaForReassignment[];
  onRefresh: () => void;
  onError: (error: string) => void;
}

function UserItem({
  user,
  sessionToken,
  isAdmin,
  allAreas,
  onRefresh,
  onError,
}: UserItemProps) {
  const [isEditing, setIsEditing] = useState(false);

  if (isEditing) {
    return (
      <EditUserForm
        user={user}
        sessionToken={sessionToken}
        allAreas={allAreas}
        onSuccess={() => {
          setIsEditing(false);
          onRefresh();
        }}
        onCancel={() => setIsEditing(false)}
        onError={onError}
      />
    );
  }

  return (
    <div className="user-item">
      <div className="user-item-header">
        <div className="user-title-group">
          <h5>
            {user.initials} - {user.name}
          </h5>
          <div className="user-meta">
            <span className="user-type">{user.user_type}</span>
            {user.crew !== null && (
              <span className="user-crew">Crew {user.crew}</span>
            )}
          </div>
        </div>
        {isAdmin && (
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
  );
}

// ============================================================================
// Create User Form Component
// ============================================================================

interface CreateUserFormProps {
  areaId: number;
  areaCode: string;
  sessionToken: string | null;
  onSuccess: () => void;
  onCancel: () => void;
  onError: (error: string) => void;
}

function CreateUserForm({
  areaId,
  areaCode,
  sessionToken,
  onSuccess,
  onCancel,
  onError,
}: CreateUserFormProps) {
  const [initials, setInitials] = useState("");
  const [name, setName] = useState("");
  const [userType, setUserType] = useState("CPC");
  const [crew, setCrew] = useState<number | null>(null);
  const [cumulativeNatcaBuDate, setCumulativeNatcaBuDate] = useState("");
  const [natcaBuDate, setNatcaBuDate] = useState("");
  const [eodFaaDate, setEodFaaDate] = useState("");
  const [serviceComputationDate, setServiceComputationDate] = useState("");
  const [lotteryValue, setLotteryValue] = useState("");
  const [creating, setCreating] = useState(false);

  const handleCreate = async () => {
    if (!sessionToken) {
      onError("Session token missing");
      return;
    }

    if (!initials.trim() || !name.trim()) {
      onError("Initials and name are required");
      return;
    }

    const lotteryNum = lotteryValue ? Number.parseInt(lotteryValue, 10) : null;

    try {
      setCreating(true);
      onError("");
      await registerUser(
        sessionToken,
        initials.trim(),
        name.trim(),
        areaId,
        areaCode,
        userType,
        crew,
        cumulativeNatcaBuDate,
        natcaBuDate,
        eodFaaDate,
        serviceComputationDate,
        lotteryNum,
      );
      onSuccess();
    } catch (err) {
      if (err instanceof ApiError) {
        onError(`Failed to create user: ${err.message}`);
      } else if (err instanceof NetworkError) {
        onError("Backend is unavailable. Please try again later.");
      } else {
        onError("Failed to create user");
      }
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="create-form">
      <h4>Add User to {areaCode}</h4>
      <div className="form-row">
        <label htmlFor="initials">Initials:</label>
        <input
          id="initials"
          type="text"
          value={initials}
          onChange={(e) => setInitials(e.target.value)}
          disabled={creating}
          maxLength={10}
        />
      </div>
      <div className="form-row">
        <label htmlFor="name">Name:</label>
        <input
          id="name"
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          disabled={creating}
        />
      </div>
      <div className="form-row">
        <label htmlFor="user-type">User Type:</label>
        <select
          id="user-type"
          value={userType}
          onChange={(e) => setUserType(e.target.value)}
          disabled={creating}
        >
          <option value="CPC">CPC</option>
          <option value="CPC-IT">CPC-IT</option>
          <option value="Dev-R">Dev-R</option>
          <option value="Dev-D">Dev-D</option>
        </select>
      </div>
      <div className="form-row">
        <label htmlFor="crew">Crew (optional):</label>
        <select
          id="crew"
          value={crew ?? ""}
          onChange={(e) => {
            const val = e.target.value;
            setCrew(val === "" ? null : Number.parseInt(val, 10));
          }}
          disabled={creating}
        >
          <option value="">None</option>
          <option value="1">1</option>
          <option value="2">2</option>
          <option value="3">3</option>
          <option value="4">4</option>
          <option value="5">5</option>
          <option value="6">6</option>
          <option value="7">7</option>
        </select>
      </div>
      <div className="form-row">
        <label htmlFor="cumulative-natca-bu-date">
          Cumulative NATCA BU Date:
        </label>
        <input
          id="cumulative-natca-bu-date"
          type="date"
          min="1960-01-01"
          max="2100-12-31"
          value={cumulativeNatcaBuDate}
          onChange={(e) => setCumulativeNatcaBuDate(e.target.value)}
          disabled={creating}
        />
      </div>
      <div className="form-row">
        <label htmlFor="natca-bu-date">NATCA BU Date:</label>
        <input
          id="natca-bu-date"
          type="date"
          min="1960-01-01"
          max="2100-12-31"
          value={natcaBuDate}
          onChange={(e) => setNatcaBuDate(e.target.value)}
          disabled={creating}
        />
      </div>
      <div className="form-row">
        <label htmlFor="eod-faa-date">EOD/FAA Date:</label>
        <input
          id="eod-faa-date"
          type="date"
          min="1960-01-01"
          max="2100-12-31"
          value={eodFaaDate}
          onChange={(e) => setEodFaaDate(e.target.value)}
          disabled={creating}
        />
      </div>
      <div className="form-row">
        <label htmlFor="service-computation-date">
          Service Computation Date:
        </label>
        <input
          id="service-computation-date"
          type="date"
          min="1960-01-01"
          max="2100-12-31"
          value={serviceComputationDate}
          onChange={(e) => setServiceComputationDate(e.target.value)}
          disabled={creating}
        />
      </div>
      <div className="form-row">
        <label htmlFor="lottery-value">Lottery Value (optional):</label>
        <input
          id="lottery-value"
          type="number"
          min="0"
          value={lotteryValue}
          onChange={(e) => setLotteryValue(e.target.value)}
          disabled={creating}
        />
      </div>
      <div className="form-actions">
        <button
          type="button"
          onClick={handleCreate}
          disabled={creating}
          className="btn-save"
        >
          {creating ? "Creating..." : "Create User"}
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
    </div>
  );
}

// ============================================================================
// Edit User Form Component
// ============================================================================

interface EditUserFormProps {
  user: UserInfo;
  sessionToken: string | null;
  allAreas: AreaForReassignment[];
  onSuccess: () => void;
  onCancel: () => void;
  onError: (error: string) => void;
}

function EditUserForm({
  user,
  sessionToken,
  allAreas,
  onSuccess,
  onCancel,
  onError,
}: EditUserFormProps) {
  const [name, setName] = useState(user.name);
  const [areaId, setAreaId] = useState(user.area_id);
  const [userType, setUserType] = useState(user.user_type);
  const [crew, setCrew] = useState<number | null>(user.crew);
  const [cumulativeNatcaBuDate, setCumulativeNatcaBuDate] = useState(
    user.cumulative_natca_bu_date,
  );
  const [natcaBuDate, setNatcaBuDate] = useState(user.natca_bu_date);
  const [eodFaaDate, setEodFaaDate] = useState(user.eod_faa_date);
  const [serviceComputationDate, setServiceComputationDate] = useState(
    user.service_computation_date,
  );
  const [lotteryValue, setLotteryValue] = useState(
    user.lottery_value?.toString() ?? "",
  );
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    if (!sessionToken) {
      onError("Session token missing");
      return;
    }

    if (!name.trim()) {
      onError("Name is required");
      return;
    }

    const lotteryNum = lotteryValue ? Number.parseInt(lotteryValue, 10) : null;

    try {
      setSaving(true);
      onError("");
      await updateUser(
        sessionToken,
        user.user_id,
        user.initials,
        name.trim(),
        areaId,
        userType,
        crew,
        cumulativeNatcaBuDate,
        natcaBuDate,
        eodFaaDate,
        serviceComputationDate,
        lotteryNum,
      );
      onSuccess();
    } catch (err) {
      if (err instanceof ApiError) {
        onError(`Failed to update user: ${err.message}`);
      } else if (err instanceof NetworkError) {
        onError("Backend is unavailable. Please try again later.");
      } else {
        onError("Failed to update user");
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="user-item edit-mode">
      <div className="form-row">
        <label htmlFor={`edit-name-${user.user_id}`}>Name:</label>
        <input
          id={`edit-name-${user.user_id}`}
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          disabled={saving}
        />
      </div>
      <div className="form-row">
        <label htmlFor={`edit-area-${user.user_id}`}>Area:</label>
        <select
          id={`edit-area-${user.user_id}`}
          value={areaId}
          onChange={(e) => setAreaId(Number.parseInt(e.target.value, 10))}
          disabled={saving}
        >
          {allAreas.map((area) => (
            <option key={area.area_id} value={area.area_id}>
              {area.area_code}
              {area.area_name ? ` - ${area.area_name}` : ""}
              {area.is_system_area ? " (System)" : ""}
            </option>
          ))}
        </select>
      </div>
      <div className="form-row">
        <label htmlFor={`edit-user-type-${user.user_id}`}>User Type:</label>
        <select
          id={`edit-user-type-${user.user_id}`}
          value={userType}
          onChange={(e) => setUserType(e.target.value)}
          disabled={saving}
        >
          <option value="CPC">CPC</option>
          <option value="CPC-IT">CPC-IT</option>
          <option value="Dev-R">Dev-R</option>
          <option value="Dev-D">Dev-D</option>
        </select>
      </div>
      <div className="form-row">
        <label htmlFor={`edit-crew-${user.user_id}`}>Crew (optional):</label>
        <select
          id={`edit-crew-${user.user_id}`}
          value={crew ?? ""}
          onChange={(e) => {
            const val = e.target.value;
            setCrew(val === "" ? null : Number.parseInt(val, 10));
          }}
          disabled={saving}
        >
          <option value="">None</option>
          <option value="1">1</option>
          <option value="2">2</option>
          <option value="3">3</option>
          <option value="4">4</option>
          <option value="5">5</option>
          <option value="6">6</option>
          <option value="7">7</option>
        </select>
      </div>
      <div className="form-row">
        <label htmlFor={`edit-cumulative-natca-${user.user_id}`}>
          Cumulative NATCA BU Date:
        </label>
        <input
          id={`edit-cumulative-natca-${user.user_id}`}
          type="date"
          min="1960-01-01"
          max="2100-12-31"
          value={cumulativeNatcaBuDate}
          onChange={(e) => setCumulativeNatcaBuDate(e.target.value)}
          disabled={saving}
        />
      </div>
      <div className="form-row">
        <label htmlFor={`edit-natca-${user.user_id}`}>NATCA BU Date:</label>
        <input
          id={`edit-natca-${user.user_id}`}
          type="date"
          min="1960-01-01"
          max="2100-12-31"
          value={natcaBuDate}
          onChange={(e) => setNatcaBuDate(e.target.value)}
          disabled={saving}
        />
      </div>
      <div className="form-row">
        <label htmlFor={`edit-eod-${user.user_id}`}>EOD/FAA Date:</label>
        <input
          id={`edit-eod-${user.user_id}`}
          type="date"
          min="1960-01-01"
          max="2100-12-31"
          value={eodFaaDate}
          onChange={(e) => setEodFaaDate(e.target.value)}
          disabled={saving}
        />
      </div>
      <div className="form-row">
        <label htmlFor={`edit-service-computation-${user.user_id}`}>
          Service Computation Date:
        </label>
        <input
          id={`edit-service-computation-${user.user_id}`}
          type="date"
          min="1960-01-01"
          max="2100-12-31"
          value={serviceComputationDate}
          onChange={(e) => setServiceComputationDate(e.target.value)}
          disabled={saving}
        />
      </div>
      <div className="form-row">
        <label htmlFor={`edit-lottery-${user.user_id}`}>
          Lottery Value (optional):
        </label>
        <input
          id={`edit-lottery-${user.user_id}`}
          type="number"
          min="0"
          value={lotteryValue}
          onChange={(e) => setLotteryValue(e.target.value)}
          disabled={saving}
        />
      </div>
      <div className="form-actions">
        <button
          type="button"
          onClick={handleSave}
          disabled={saving}
          className="btn-save"
        >
          {saving ? "Saving..." : "Save"}
        </button>
        <button
          type="button"
          onClick={onCancel}
          disabled={saving}
          className="btn-cancel"
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

// Note: User count blocker rendering removed - would need to be added to
// the BlockingReason discriminated union type if needed

// ============================================================================
// CSV Import Section Component
// ============================================================================
