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
  getBootstrapCompleteness,
  listUsers,
  NetworkError,
  registerUser,
  updateUser,
} from "../api";
import type {
  AreaCompletenessInfo,
  BlockingReason,
  ConnectionState,
  GetBootstrapCompletenessResponse,
  GlobalCapabilities,
  LiveEvent,
  ListUsersResponse,
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
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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
      lastEvent.type === "user_created" ||
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

  const activeBidYearNum = completeness.active_bid_year;
  const activeAreas = completeness.areas.filter(
    (a) => a.bid_year === activeBidYearNum,
  );

  // Separate system and non-system areas
  const nonSystemAreas = activeAreas.filter((a) => !a.is_system_area);
  const systemAreas = activeAreas.filter((a) => a.is_system_area);

  // Find user count blockers
  const userCountBlockers = completeness.blocking_reasons.filter(
    (br) =>
      br.reason_type === "UserCountMismatch" ||
      br.reason_type === "UnexpectedUsers",
  );

  return (
    <div className="bootstrap-completeness">
      <BootstrapNavigation currentStep="users" />
      <ReadinessWidget
        lifecycleState={completeness.lifecycle_state}
        isReady={completeness.is_ready}
        blockingReasons={completeness.blocking_reasons}
      />

      <div className="bootstrap-content">
        <section className="bootstrap-section">
          <h2 className="section-title">User Management</h2>
          <p className="section-description">
            Add and configure users for each area. Users can be added manually
            or imported via CSV.
          </p>

          {userCountBlockers.length > 0 && (
            <div className="blockers-list">
              <h4>User Count Issues:</h4>
              <ul>
                {userCountBlockers.map((br, idx) => (
                  <li key={idx}>{renderBlockingReason(br)}</li>
                ))}
              </ul>
            </div>
          )}
        </section>

        {/* CSV User Import Section */}
        {isAdmin &&
          sessionToken !== null &&
          activeBidYearNum !== null &&
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
        {activeBidYearNum !== null && nonSystemAreas.length > 0 && (
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

        {/* System Areas (Display Only) */}
        {systemAreas.length > 0 && (
          <section className="bootstrap-section">
            <h3 className="section-title">System Areas</h3>
            <p className="section-description">
              System areas are managed automatically. Users in these areas will
              be reviewed in the next step.
            </p>
            {systemAreas.map((area) => (
              <UserManagementForArea
                key={`users-${area.bid_year}-${area.area_id}`}
                area={area}
                isAdmin={false}
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

interface UserManagementForAreaProps {
  area: AreaCompletenessInfo;
  isAdmin: boolean;
  sessionToken: string | null;
  onError: (error: string) => void;
  onRefresh: () => Promise<void>;
}

function UserManagementForArea({
  area,
  isAdmin,
  sessionToken,
  onError,
  onRefresh,
}: UserManagementForAreaProps) {
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [showCreateForm, setShowCreateForm] = useState(false);

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
              areaId={area.area_id}
              isAdmin={isAdmin}
              sessionToken={sessionToken}
              onRefresh={async () => {
                await refreshUsers();
                await onRefresh();
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
  areaId: number;
  isAdmin: boolean;
  sessionToken: string | null;
  onRefresh: () => Promise<void>;
  onError: (error: string) => void;
}

function UserItem({
  user,
  areaId,
  isAdmin,
  sessionToken,
  onRefresh,
  onError,
}: UserItemProps) {
  const [isEditing, setIsEditing] = useState(false);

  if (isEditing) {
    return (
      <EditUserForm
        user={user}
        areaId={areaId}
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
        <label htmlFor="initials">
          Initials:
          <input
            id="initials"
            type="text"
            value={initials}
            onChange={(e) => setInitials(e.target.value)}
            disabled={creating}
            maxLength={10}
          />
        </label>
      </div>
      <div className="form-row">
        <label htmlFor="name">
          Name:
          <input
            id="name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={creating}
          />
        </label>
      </div>
      <div className="form-row">
        <label htmlFor="user-type">
          User Type:
          <select
            id="user-type"
            value={userType}
            onChange={(e) => setUserType(e.target.value)}
            disabled={creating}
          >
            <option value="CPC">CPC</option>
            <option value="NONCPC">NONCPC</option>
          </select>
        </label>
      </div>
      <div className="form-row">
        <label htmlFor="crew">
          Crew (optional):
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
        </label>
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
  areaId: number;
  sessionToken: string | null;
  onSuccess: () => void;
  onCancel: () => void;
  onError: (error: string) => void;
}

function EditUserForm({
  user,
  areaId,
  sessionToken,
  onSuccess,
  onCancel,
  onError,
}: EditUserFormProps) {
  const [name, setName] = useState(user.name);
  const [userType, setUserType] = useState(user.user_type);
  const [crew, setCrew] = useState<number | null>(user.crew);
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
        <label htmlFor={`name-${user.user_id}`}>
          Name:
          <input
            id={`name-${user.user_id}`}
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={saving}
          />
        </label>
      </div>
      <div className="form-row">
        <label htmlFor={`user-type-${user.user_id}`}>
          User Type:
          <select
            id={`user-type-${user.user_id}`}
            value={userType}
            onChange={(e) => setUserType(e.target.value)}
            disabled={saving}
          >
            <option value="CPC">CPC</option>
            <option value="NONCPC">NONCPC</option>
          </select>
        </label>
      </div>
      <div className="form-row">
        <label htmlFor={`crew-${user.user_id}`}>
          Crew:
          <select
            id={`crew-${user.user_id}`}
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
        </label>
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

// ============================================================================
// Blocking Reason Renderer
// ============================================================================

function renderBlockingReason(br: BlockingReason): string {
  switch (br.reason_type) {
    case "UserCountMismatch": {
      const { bid_year, area_code, expected, actual } = br.details;
      return `Area ${area_code} (Bid Year ${bid_year}): Expected ${expected} users, found ${actual}`;
    }
    case "UnexpectedUsers": {
      const { bid_year, user_count, sample_initials } = br.details;
      const userList = sample_initials
        .slice(0, 5)
        .map((i: string) => `"${i}"`)
        .join(", ");
      return `Bid Year ${bid_year} has ${user_count} unexpected users (e.g. ${userList})`;
    }
    default:
      return `Unknown blocking reason: ${br.reason_type}`;
  }
}
