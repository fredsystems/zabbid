// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Main application component with routing structure.
 * Provides navigation between bootstrap overview, area view, user list, and user detail.
 */

import { BrowserRouter, Link, Route, Routes } from "react-router-dom";
import { AreaView } from "./components/AreaView";
import { BootstrapOverview } from "./components/BootstrapOverview";
import { UserDetailView } from "./components/UserDetailView";
import { UserListView } from "./components/UserListView";
import "./App.css";

export function App() {
  return (
    <BrowserRouter>
      <div className="app">
        <header className="app-header">
          <h1>Zabbid Operator UI</h1>
          <nav>
            <Link to="/">Bootstrap Overview</Link>
          </nav>
        </header>
        <main className="app-main">
          <Routes>
            <Route path="/" element={<BootstrapOverview />} />
            <Route path="/bid-year/:year/areas" element={<AreaView />} />
            <Route
              path="/bid-year/:year/area/:areaId/users"
              element={<UserListView />}
            />
            <Route
              path="/bid-year/:year/area/:areaId/user/:initials"
              element={<UserDetailView />}
            />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}
