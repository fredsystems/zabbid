// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * User Edit View component.
 *
 * Comprehensive user editing with lifecycle awareness and capability gating.
 *
 * Features:
 * - Editable fields: name, user type, crew, seniority dates, lottery value
 * - Read-only fields: initials, bid year, area
 * - Lifecycle context always visible
 * - Capability-gated actions (delete, area reassignment)
 * - Override workflow for post-canonicalization area changes
 * - Mobile-first responsive design
 *
 * Capabilities enforced:
 * - can_delete: controls delete user button
 * - can_move_area: controls direct area change vs override requirement
 * - can_edit_seniority: always allowed (no gating needed for now)
 */

import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  ApiError,
  listAreas,
  listBidYears,
  listUsers,
  updateUser,
} from "../api";
import type { AreaInfo, ConnectionState, LiveEvent, UserInfo } from "../types";
import { OverrideAreaModal } from "./OverrideAreaModal";

interface UserEditViewProps {
  sessionToken: string | null;
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function UserEditView({
  sessionToken,
  connectionState,
}: UserEditViewProps) {
  const { bidYearId, areaId, userId } = useParams<{
    bidYearId: string;
    areaId: string;
    userId: string;
  }>();
  const navigate = useNavigate();

  // State
  const [user, setUser] = useState<UserInfo | null>(null);
  const [areas, setAreas] = useState<AreaInfo[]>([]);
  const [bidYearLifecycle, setBidYearLifecycle] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // Form fields
  const [name, setName] = useState("");
  const [userType, setUserType] = useState("");
  const [crew, setCrew] = useState("");
  const [cumulativeNatcaBuDate, setCumulativeNatcaBuDate] = useState("");
  const [natcaBuDate, setNatcaBuDate] = useState("");
  const [eodFaaDate, setEodFaaDate] = useState("");
  const [serviceComputationDate, setServiceComputationDate] = useState("");
  const [lotteryValue, setLotteryValue] = useState("");

  // Override modal
  const [showOverrideModal, setShowOverrideModal] = useState(false);

  // Parse IDs
  const bidYearIdNum = bidYearId ? parseInt(bidYearId, 10) : null;
  const areaIdNum = areaId ? parseInt(areaId, 10) : null;
  const userIdNum = userId ? parseInt(userId, 10) : null;

  // Load user and areas
  useEffect(() => {
    if (
      !sessionToken ||
      bidYearIdNum === null ||
      areaIdNum === null ||
      userIdNum === null
    ) {
      return;
    }

    const loadData = async () => {
      try {
        setLoading(true);
        setError(null);

        // Load user from users list
        const usersResponse = await listUsers(sessionToken, areaIdNum);
        const foundUser = usersResponse.users.find(
          (u) => u.user_id === userIdNum,
        );

        if (!foundUser) {
          setError("User not found");
          return;
        }

        setUser(foundUser);

        // Initialize form fields
        setName(foundUser.name);
        setUserType(foundUser.user_type);
        setCrew(foundUser.crew?.toString() ?? "");
        setCumulativeNatcaBuDate(foundUser.cumulative_natca_bu_date);
        setNatcaBuDate(foundUser.natca_bu_date);
        setEodFaaDate(foundUser.eod_faa_date);
        setServiceComputationDate(foundUser.service_computation_date);
        setLotteryValue(foundUser.lottery_value?.toString() ?? "");

        // Load areas for override modal and bid year lifecycle
        if (bidYearIdNum) {
          const areasResponse = await listAreas(bidYearIdNum);
          setAreas(areasResponse.areas);

          // Get bid year lifecycle
          const bidYears = await listBidYears();
          const bidYear = bidYears.find(
            (by) => by.bid_year_id === bidYearIdNum,
          );
          if (bidYear) {
            setBidYearLifecycle(bidYear.lifecycle_state);
          }
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load user");
      } finally {
        setLoading(false);
      }
    };

    void loadData();
  }, [sessionToken, bidYearIdNum, areaIdNum, userIdNum]);

  // Auto-refresh on connection restore
  useEffect(() => {
    // Capture stable user_id to avoid infinite loop from user object reference
    const currentUserId = user?.user_id;

    if (
      connectionState === "connected" &&
      currentUserId &&
      sessionToken &&
      areaIdNum
    ) {
      const refresh = async () => {
        try {
          const usersResponse = await listUsers(sessionToken, areaIdNum);
          const foundUser = usersResponse.users.find(
            (u) => u.user_id === currentUserId,
          );
          if (foundUser) {
            setUser(foundUser);
            setName(foundUser.name);
            setUserType(foundUser.user_type);
            setCrew(foundUser.crew?.toString() ?? "");
            setCumulativeNatcaBuDate(foundUser.cumulative_natca_bu_date);
            setNatcaBuDate(foundUser.natca_bu_date);
            setEodFaaDate(foundUser.eod_faa_date);
            setServiceComputationDate(foundUser.service_computation_date);
            setLotteryValue(foundUser.lottery_value?.toString() ?? "");
          }
        } catch (err) {
          console.error("Failed to refresh user data:", err);
        }
      };
      void refresh();
    }
  }, [connectionState, sessionToken, areaIdNum, user?.user_id]);

  // Handle save
  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!sessionToken || !user || areaIdNum === null) {
      return;
    }

    const crewNum = crew ? parseInt(crew, 10) : null;

    if (
      crewNum !== null &&
      (Number.isNaN(crewNum) || crewNum < 1 || crewNum > 7)
    ) {
      setError("Crew must be a number between 1 and 7");
      return;
    }

    try {
      setSaving(true);
      setError(null);

      await updateUser(
        sessionToken,
        user.user_id,
        user.initials,
        name,
        areaIdNum,
        userType,
        crewNum,
        cumulativeNatcaBuDate,
        natcaBuDate,
        eodFaaDate,
        serviceComputationDate,
        lotteryValue ? parseInt(lotteryValue, 10) : null,
      );

      // Refresh user data
      const usersResponse = await listUsers(sessionToken, areaIdNum);
      const updatedUser = usersResponse.users.find(
        (u) => u.user_id === user.user_id,
      );
      if (updatedUser) {
        setUser(updatedUser);
      }

      // Navigate back to user list
      navigate(`/admin/bid-year/${bidYearIdNum}/areas/${areaIdNum}/users`);
    } catch (err) {
      if (err instanceof ApiError) {
        setError(`Failed to update user: ${err.message}`);
      } else {
        setError(err instanceof Error ? err.message : "Failed to update user");
      }
    } finally {
      setSaving(false);
    }
  };

  // Handle override success
  const handleOverrideSuccess = async () => {
    setShowOverrideModal(false);

    if (!sessionToken || areaIdNum === null) {
      return;
    }

    // Refresh user data
    try {
      const usersResponse = await listUsers(sessionToken, areaIdNum);
      const updatedUser = usersResponse.users.find(
        (u) => u.user_id === user?.user_id,
      );
      if (updatedUser) {
        setUser(updatedUser);
      }
    } catch (err) {
      console.error("Failed to refresh after override:", err);
    }
  };

  // Render helpers
  const isPostCanonicalization =
    bidYearLifecycle === "Canonicalized" ||
    bidYearLifecycle === "BiddingActive" ||
    bidYearLifecycle === "BiddingClosed";

  if (bidYearIdNum === null || areaIdNum === null || userIdNum === null) {
    return (
      <div className="error">
        <h2>Invalid Parameters</h2>
        <p>Required parameters are missing or invalid.</p>
        <button type="button" onClick={() => navigate("/admin")}>
          Back to Overview
        </button>
      </div>
    );
  }

  if (!sessionToken) {
    return (
      <div className="error">
        <h2>Not Authenticated</h2>
        <p>Please log in to edit users.</p>
        <button type="button" onClick={() => navigate("/login")}>
          Go to Login
        </button>
      </div>
    );
  }

  if (loading) {
    return <div className="loading">Loading user details...</div>;
  }

  if (error && !user) {
    return (
      <div className="error">
        <h2>Unable to Load User</h2>
        <p>{error}</p>
        <button
          type="button"
          onClick={() =>
            navigate(`/admin/bid-year/${bidYearIdNum}/areas/${areaIdNum}/users`)
          }
        >
          Back to User List
        </button>
      </div>
    );
  }

  if (!user) {
    return (
      <div className="error">
        <h2>User Not Found</h2>
        <button
          type="button"
          onClick={() =>
            navigate(`/admin/bid-year/${bidYearIdNum}/areas/${areaIdNum}/users`)
          }
        >
          Back to User List
        </button>
      </div>
    );
  }

  return (
    <div className="user-edit-view">
      <div className="view-header">
        <h2>Edit User: {user.initials}</h2>
        <button
          type="button"
          onClick={() =>
            navigate(`/admin/bid-year/${bidYearIdNum}/areas/${areaIdNum}/users`)
          }
        >
          Back to User List
        </button>
      </div>

      {/* Lifecycle context */}
      {bidYearLifecycle && (
        <div className="lifecycle-context">
          <span className="lifecycle-badge">
            {bidYearLifecycle}
            {isPostCanonicalization && " 🔒"}
          </span>
          {isPostCanonicalization && (
            <p className="lifecycle-note">
              Some editing restrictions apply after canonicalization.
            </p>
          )}
        </div>
      )}

      <form onSubmit={handleSave} className="user-edit-form">
        <section className="form-section">
          <h3>User Information</h3>

          <div className="form-row">
            <label htmlFor="user-initials">Initials (read-only):</label>
            <input
              id="user-initials"
              type="text"
              value={user.initials}
              disabled
            />
          </div>

          <div className="form-row">
            <label htmlFor="user-name">Name:</label>
            <input
              id="user-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              disabled={saving}
              required
            />
          </div>

          <div className="form-row">
            <label htmlFor="user-type">User Type:</label>
            <select
              id="user-type"
              value={userType}
              onChange={(e) => setUserType(e.target.value)}
              disabled={saving}
              required
            >
              <option value="CPC">CPC</option>
              <option value="CPC-IT">CPC-IT</option>
              <option value="Dev-R">Dev-R</option>
              <option value="Dev-D">Dev-D</option>
            </select>
          </div>

          <div className="form-row">
            <label htmlFor="user-crew">Crew (1-7, optional):</label>
            <input
              id="user-crew"
              type="number"
              min="1"
              max="7"
              value={crew}
              onChange={(e) => setCrew(e.target.value)}
              disabled={saving}
            />
          </div>
        </section>

        <section className="form-section">
          <h3>Seniority Information</h3>
          <p className="form-note">
            Seniority fields can be edited at any lifecycle state.
          </p>

          <div className="form-row">
            <label htmlFor="cumulative-natca-bu-date">
              Cumulative NATCA BU Date:
            </label>
            <input
              id="cumulative-natca-bu-date"
              type="date"
              value={cumulativeNatcaBuDate}
              onChange={(e) => setCumulativeNatcaBuDate(e.target.value)}
              disabled={saving}
              required
            />
          </div>

          <div className="form-row">
            <label htmlFor="natca-bu-date">NATCA BU Date:</label>
            <input
              id="natca-bu-date"
              type="date"
              value={natcaBuDate}
              onChange={(e) => setNatcaBuDate(e.target.value)}
              disabled={saving}
              required
            />
          </div>

          <div className="form-row">
            <label htmlFor="eod-faa-date">EOD/FAA Date:</label>
            <input
              id="eod-faa-date"
              type="date"
              value={eodFaaDate}
              onChange={(e) => setEodFaaDate(e.target.value)}
              disabled={saving}
              required
            />
          </div>

          <div className="form-row">
            <label htmlFor="service-computation-date">
              Service Computation Date:
            </label>
            <input
              id="service-computation-date"
              type="date"
              value={serviceComputationDate}
              onChange={(e) => setServiceComputationDate(e.target.value)}
              disabled={saving}
              required
            />
          </div>

          <div className="form-row">
            <label htmlFor="lottery-value">Lottery Value (optional):</label>
            <input
              id="lottery-value"
              type="number"
              value={lotteryValue}
              onChange={(e) => setLotteryValue(e.target.value)}
              disabled={saving}
            />
          </div>
        </section>

        {error && (
          <div className="error-message">
            <p>{error}</p>
          </div>
        )}

        <div className="form-actions">
          <button type="submit" disabled={saving || !name} className="btn-save">
            {saving ? "Saving..." : "Save Changes"}
          </button>
          <button
            type="button"
            onClick={() =>
              navigate(
                `/admin/bid-year/${bidYearIdNum}/areas/${areaIdNum}/users`,
              )
            }
            disabled={saving}
            className="btn-cancel"
          >
            Cancel
          </button>
        </div>
      </form>

      <section className="admin-actions">
        <h3>Administrative Actions</h3>

        <div className="action-group">
          <h4>Area Assignment</h4>
          {user.capabilities.can_move_area ? (
            <p className="action-note">
              Direct area changes are allowed (pre-canonicalization).
            </p>
          ) : (
            <>
              <p className="action-note">
                Direct area changes are not allowed after canonicalization. Use
                the override workflow below.
              </p>
              <button
                type="button"
                onClick={() => setShowOverrideModal(true)}
                className="btn-action"
              >
                Change Area (Override Required)
              </button>
            </>
          )}
        </div>

        <div className="action-group">
          <h4>Delete User</h4>
          {user.capabilities.can_delete ? (
            <p className="action-note">Deletion moves user to "No Bid" area.</p>
          ) : (
            <p className="action-disabled">
              Users cannot be deleted after canonicalization.
            </p>
          )}
          <button
            type="button"
            disabled={!user.capabilities.can_delete}
            className="btn-delete"
            title={
              user.capabilities.can_delete
                ? "Delete this user (moves to No Bid)"
                : "Users cannot be deleted after canonicalization"
            }
          >
            Delete User
          </button>
        </div>
      </section>

      {showOverrideModal && (
        <OverrideAreaModal
          sessionToken={sessionToken}
          userId={user.user_id}
          userInitials={user.initials}
          currentAreaId={user.area_id}
          availableAreas={areas}
          onSuccess={handleOverrideSuccess}
          onCancel={() => setShowOverrideModal(false)}
        />
      )}
    </div>
  );
}
