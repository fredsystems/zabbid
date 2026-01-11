// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * React hook for managing WebSocket connection to live event stream.
 * Handles connection state, automatic reconnection, and event distribution.
 */

import { useEffect, useRef, useState } from "react";
import type { ConnectionState, LiveEvent } from "./types";

/**
 * Configuration for WebSocket connection behavior.
 */
interface LiveEventsConfig {
  /** Initial reconnection delay in milliseconds */
  initialReconnectDelay?: number;
  /** Maximum reconnection delay in milliseconds */
  maxReconnectDelay?: number;
  /** Multiplier for exponential backoff */
  reconnectBackoffMultiplier?: number;
}

/**
 * Hook for subscribing to live events from the backend.
 *
 * This hook:
 * - Establishes WebSocket connection on mount
 * - Automatically reconnects with exponential backoff
 * - Provides connection state visibility
 * - Distributes received events to callback
 * - Cleans up connection on unmount
 *
 * @param onEvent - Callback invoked when events are received
 * @param config - Optional configuration for connection behavior
 * @returns Current connection state
 */
export function useLiveEvents(
  onEvent: (event: LiveEvent) => void,
  config: LiveEventsConfig = {},
): ConnectionState {
  const {
    initialReconnectDelay = 1000,
    maxReconnectDelay = 30000,
    reconnectBackoffMultiplier = 2,
  } = config;

  const [connectionState, setConnectionState] =
    useState<ConnectionState>("connecting");

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | null>(null);
  const reconnectDelayRef = useRef<number>(initialReconnectDelay);
  const shouldConnectRef = useRef<boolean>(true);

  useEffect(() => {
    shouldConnectRef.current = true;

    const connect = () => {
      if (!shouldConnectRef.current) {
        return;
      }

      // Clear any existing connection
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }

      setConnectionState("connecting");

      // Determine WebSocket URL based on current location
      const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
      const host = window.location.host;
      const url = `${protocol}//${host}/api/live`;

      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        console.log("[LiveEvents] WebSocket connected");
        setConnectionState("connected");
        // Reset reconnect delay on successful connection
        reconnectDelayRef.current = initialReconnectDelay;
      };

      ws.onmessage = (event) => {
        try {
          const liveEvent = JSON.parse(event.data) as LiveEvent;
          console.log("[LiveEvents] Received event:", liveEvent);
          onEvent(liveEvent);
        } catch (error) {
          console.error("[LiveEvents] Failed to parse event:", error);
        }
      };

      ws.onerror = (error) => {
        console.error("[LiveEvents] WebSocket error:", error);
        setConnectionState("error");
      };

      ws.onclose = (event) => {
        console.log("[LiveEvents] WebSocket closed:", event.code, event.reason);
        wsRef.current = null;

        if (!shouldConnectRef.current) {
          // Clean shutdown, don't reconnect
          setConnectionState("disconnected");
          return;
        }

        // Connection lost, attempt reconnect
        setConnectionState("disconnected");

        const delay = reconnectDelayRef.current;
        console.log(`[LiveEvents] Reconnecting in ${delay}ms...`);

        reconnectTimeoutRef.current = window.setTimeout(() => {
          // Increase delay for next attempt (exponential backoff)
          reconnectDelayRef.current = Math.min(
            reconnectDelayRef.current * reconnectBackoffMultiplier,
            maxReconnectDelay,
          );
          connect();
        }, delay);
      };
    };

    // Initial connection
    connect();

    // Cleanup on unmount
    return () => {
      shouldConnectRef.current = false;

      // Clear reconnect timeout
      if (reconnectTimeoutRef.current !== null) {
        clearTimeout(reconnectTimeoutRef.current);
        reconnectTimeoutRef.current = null;
      }

      // Close WebSocket connection
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [
    onEvent,
    initialReconnectDelay,
    maxReconnectDelay,
    reconnectBackoffMultiplier,
  ]);

  return connectionState;
}
