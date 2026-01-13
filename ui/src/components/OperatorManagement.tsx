// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Operator Management UI (Admin Only)
 *
 * Mobile-first interface for managing system operators.
 * Supports create, disable, enable, and delete operations.
 */

import { useCallback, useEffect, useState } from "react";
import * as api from "../api";
import { ApiError } from "../api";

interface Operator {
  operator_id: number;
  login_name: string;
  display_name: string;
  role: string;
  is_disabled: boolean;
  created_at: string;
  last_login_at: string | null;
}

interface OperatorManagementProps {
  sessionToken: string;
}

export function OperatorManagement({ sessionToken }: OperatorManagementProps) {
  const [operators, setOperators] = useState<Operator[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);

  // Create form state
  const [newLoginName, setNewLoginName] = useState("");
  const [newDisplayName, setNewDisplayName] = useState("");
  const [newRole, setNewRole] = useState("Bidder");
  const [newPassword, setNewPassword] = useState("");
  const [newPasswordConfirmation, setNewPasswordConfirmation] = useState("");
  const [createError, setCreateError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const loadOperators = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await api.listOperators(sessionToken);
      setOperators(response.operators);
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
      } else {
        setError("Failed to load operators");
      }
    } finally {
      setLoading(false);
    }
  }, [sessionToken]);

  useEffect(() => {
    loadOperators();
  }, [loadOperators]);

  const handleCreateOperator = async (e: React.FormEvent) => {
    e.preventDefault();
    setCreateError(null);
    setCreating(true);

    try {
      await api.createOperator(
        sessionToken,
        newLoginName,
        newDisplayName,
        newRole,
        newPassword,
        newPasswordConfirmation,
      );
      setNewLoginName("");
      setNewDisplayName("");
      setNewRole("Bidder");
      setNewPassword("");
      setNewPasswordConfirmation("");
      setShowCreateForm(false);
      await loadOperators();
    } catch (err) {
      if (err instanceof ApiError) {
        setCreateError(err.message);
      } else {
        setCreateError("Failed to create operator");
      }
    } finally {
      setCreating(false);
    }
  };

  const handleDisable = async (operatorId: number) => {
    if (!confirm("Are you sure you want to disable this operator?")) {
      return;
    }

    try {
      await api.disableOperator(sessionToken, operatorId);
      await loadOperators();
    } catch (err) {
      if (err instanceof ApiError) {
        alert(`Failed to disable operator: ${err.message}`);
      } else {
        alert("Failed to disable operator");
      }
    }
  };

  const handleEnable = async (operatorId: number) => {
    try {
      await api.enableOperator(sessionToken, operatorId);
      await loadOperators();
    } catch (err) {
      if (err instanceof ApiError) {
        alert(`Failed to enable operator: ${err.message}`);
      } else {
        alert("Failed to enable operator");
      }
    }
  };

  const handleDelete = async (operatorId: number, loginName: string) => {
    if (
      !confirm(
        `Are you sure you want to delete operator "${loginName}"?\n\nThis can only be done if the operator has no audit history.`,
      )
    ) {
      return;
    }

    try {
      await api.deleteOperator(sessionToken, operatorId);
      await loadOperators();
    } catch (err) {
      if (err instanceof ApiError) {
        alert(`Failed to delete operator: ${err.message}`);
      } else {
        alert("Failed to delete operator");
      }
    }
  };

  if (loading) {
    return (
      <div className="operator-management">
        <div className="loading">Loading operators...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="operator-management">
        <div className="error-message">{error}</div>
        <button
          type="button"
          onClick={loadOperators}
          className="button-primary"
        >
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="operator-management">
      <div className="operator-header">
        <h2>Operator Management</h2>
        <button
          type="button"
          onClick={() => setShowCreateForm(!showCreateForm)}
          className="button-primary"
        >
          {showCreateForm ? "Cancel" : "Create Operator"}
        </button>
      </div>

      {showCreateForm && (
        <div className="operator-create-form">
          <h3>Create New Operator</h3>
          <form onSubmit={handleCreateOperator}>
            <div className="form-group">
              <label htmlFor="new-login-name">Login Name</label>
              <input
                id="new-login-name"
                type="text"
                value={newLoginName}
                onChange={(e) => setNewLoginName(e.target.value)}
                required
                disabled={creating}
              />
            </div>

            <div className="form-group">
              <label htmlFor="new-display-name">Display Name</label>
              <input
                id="new-display-name"
                type="text"
                value={newDisplayName}
                onChange={(e) => setNewDisplayName(e.target.value)}
                required
                disabled={creating}
              />
            </div>

            <div className="form-group">
              <label htmlFor="new-role">Role</label>
              <select
                id="new-role"
                value={newRole}
                onChange={(e) => setNewRole(e.target.value)}
                disabled={creating}
              >
                <option value="Bidder">Bidder</option>
                <option value="Admin">Admin</option>
              </select>
            </div>

            <div className="form-group">
              <label htmlFor="new-password">Password</label>
              <input
                id="new-password"
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                required
                disabled={creating}
              />
            </div>

            <div className="form-group">
              <label htmlFor="new-password-confirmation">
                Confirm Password
              </label>
              <input
                id="new-password-confirmation"
                type="password"
                value={newPasswordConfirmation}
                onChange={(e) => setNewPasswordConfirmation(e.target.value)}
                required
                disabled={creating}
              />
            </div>

            {createError && <div className="error-message">{createError}</div>}

            <div className="form-actions">
              <button
                type="submit"
                disabled={creating}
                className="button-primary"
              >
                {creating ? "Creating..." : "Create"}
              </button>
              <button
                type="button"
                onClick={() => setShowCreateForm(false)}
                disabled={creating}
                className="button-cancel"
              >
                Cancel
              </button>
            </div>
          </form>
        </div>
      )}

      <div className="operator-list">
        {operators.length === 0 ? (
          <div className="empty-state">No operators found</div>
        ) : (
          operators.map((operator) => (
            <div
              key={operator.operator_id}
              className={`operator-card ${operator.is_disabled ? "disabled" : ""}`}
            >
              <div className="operator-card-header">
                <div className="operator-name">{operator.display_name}</div>
                <div className="operator-status">
                  {operator.is_disabled ? (
                    <span className="status-badge status-disabled">
                      Disabled
                    </span>
                  ) : (
                    <span className="status-badge status-active">Active</span>
                  )}
                </div>
              </div>

              <div className="operator-card-body">
                <div className="operator-detail">
                  <span className="detail-label">Login:</span>
                  <span className="detail-value">{operator.login_name}</span>
                </div>
                <div className="operator-detail">
                  <span className="detail-label">Role:</span>
                  <span className="detail-value">{operator.role}</span>
                </div>
                <div className="operator-detail">
                  <span className="detail-label">ID:</span>
                  <span className="detail-value">{operator.operator_id}</span>
                </div>
                <div className="operator-detail">
                  <span className="detail-label">Last Login:</span>
                  <span className="detail-value">
                    {operator.last_login_at
                      ? new Date(operator.last_login_at).toLocaleString()
                      : "Never"}
                  </span>
                </div>
              </div>

              <div className="operator-card-actions">
                {operator.is_disabled ? (
                  <button
                    type="button"
                    onClick={() => handleEnable(operator.operator_id)}
                    className="button-success"
                  >
                    Enable
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={() => handleDisable(operator.operator_id)}
                    className="button-warning"
                  >
                    Disable
                  </button>
                )}
                <button
                  type="button"
                  onClick={() =>
                    handleDelete(operator.operator_id, operator.login_name)
                  }
                  className="button-error"
                >
                  Delete
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
