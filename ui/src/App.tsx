// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Main application component with routing structure.
 * Provides navigation between bootstrap overview, area view, user list, and user detail.
 * Manages WebSocket connection for live state updates.
 */

import { useCallback, useState } from "react";
import { BrowserRouter, Link, Route, Routes } from "react-router-dom";
import { AreaView } from "./components/AreaView";
import { BootstrapOverview } from "./components/BootstrapOverview";
import { ConnectionStatus } from "./components/ConnectionStatus";
import { UserDetailView } from "./components/UserDetailView";
import { UserListView } from "./components/UserListView";
import type { LiveEvent } from "./types";
import { useLiveEvents } from "./useLiveEvents";
import "./App.css";

export function App() {
  const [lastEvent, setLastEvent] = useState<LiveEvent | null>(null);

  const handleLiveEvent = useCallback((event: LiveEvent) => {
    console.log("[App] Received live event:", event);
    setLastEvent(event);

    // Event is received but components will refetch data as needed
    // This is informational only - backend remains authoritative
  }, []);

  const connectionState = useLiveEvents(handleLiveEvent);

  return (
    <BrowserRouter>
      <div className="app">
        <header className="app-header">
          <div style={{ display: "flex", alignItems: "center", gap: "2rem" }}>
            <h1>Zabbid Operator UI</h1>
            <ConnectionStatus state={connectionState} />
          </div>
          <nav>
            <Link to="/">Bootstrap Overview</Link>
          </nav>
        </header>
        <main className="app-main">
          <Routes>
            <Route
              path="/"
              element={
                <BootstrapOverview
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              }
            />
            <Route
              path="/bid-year/:year/areas"
              element={
                <AreaView
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              }
            />
            <Route
              path="/bid-year/:year/area/:areaId/users"
              element={
                <UserListView
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              }
            />
            <Route
              path="/bid-year/:year/area/:areaId/user/:initials"
              element={
                <UserDetailView
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              }
            />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}
