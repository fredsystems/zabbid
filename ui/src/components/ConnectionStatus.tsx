// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Connection status indicator component.
 * Displays the current backend connectivity state with visual feedback.
 */

import type { ConnectionState } from "../types";

interface ConnectionStatusProps {
  /** Current connection state */
  state: ConnectionState;
}

/**
 * Visual indicator of backend connection status.
 *
 * Displays:
 * - Connecting: Yellow indicator with "Connecting..." message
 * - Connected: Green indicator with "Connected" message
 * - Disconnected: Orange indicator with "Reconnecting..." message
 * - Error: Red indicator with "Connection Error" message
 */
export function ConnectionStatus({ state }: ConnectionStatusProps) {
  const getStatusText = (): string => {
    switch (state) {
      case "connected":
        return "Connected";
      case "connecting":
        return "Connecting...";
      case "disconnected":
        return "Reconnecting...";
      case "error":
        return "Connection Error";
    }
  };

  const getStatusDescription = (): string | null => {
    switch (state) {
      case "connected":
        return null;
      case "connecting":
        return "Establishing connection to backend";
      case "disconnected":
        return "Connection lost, attempting to reconnect automatically";
      case "error":
        return "Unable to connect to backend. Please check that the server is running.";
    }
  };

  const text = getStatusText();
  const description = getStatusDescription();

  const statusClass = `connection-status ${state}`;
  const indicatorClass = `connection-indicator ${state === "connecting" ? "pulse" : ""}`;

  return (
    <div className={statusClass}>
      <div className={indicatorClass} />
      <div>
        <div className="connection-text">{text}</div>
        {description && (
          <div className="connection-description">{description}</div>
        )}
      </div>
    </div>
  );
}
