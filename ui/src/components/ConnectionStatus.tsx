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
  const getStatusColor = (): string => {
    switch (state) {
      case "connected":
        return "#10b981"; // green
      case "connecting":
        return "#f59e0b"; // amber
      case "disconnected":
        return "#f97316"; // orange
      case "error":
        return "#ef4444"; // red
    }
  };

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

  const color = getStatusColor();
  const text = getStatusText();
  const description = getStatusDescription();

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: "0.5rem",
        padding: "0.5rem 1rem",
        borderRadius: "0.375rem",
        backgroundColor:
          state === "connected" ? "transparent" : "rgba(0, 0, 0, 0.05)",
        fontSize: "0.875rem",
      }}
    >
      <div
        style={{
          width: "0.5rem",
          height: "0.5rem",
          borderRadius: "50%",
          backgroundColor: color,
          animation: state === "connecting" ? "pulse 2s infinite" : undefined,
        }}
      />
      <div>
        <div style={{ fontWeight: 500 }}>{text}</div>
        {description && (
          <div style={{ fontSize: "0.75rem", color: "#6b7280" }}>
            {description}
          </div>
        )}
      </div>
      <style>
        {`
          @keyframes pulse {
            0%, 100% {
              opacity: 1;
            }
            50% {
              opacity: 0.5;
            }
          }
        `}
      </style>
    </div>
  );
}
