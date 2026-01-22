// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * Main application component with authentication and routing.
 *
 * Phase 15 implements:
 * - Public landing page at /
 * - Gated admin UI at /admin
 * - Bootstrap authentication flow
 * - Session-based authentication
 */

import { useCallback, useEffect, useState } from "react";
import {
  BrowserRouter,
  Navigate,
  Route,
  Routes,
  useNavigate,
} from "react-router-dom";
import * as api from "./api";
import { AreaRoundGroupAssignmentWrapper } from "./components/AreaRoundGroupAssignmentWrapper";
import { AreaSetup } from "./components/AreaSetup";
import { AreaView } from "./components/AreaView";
import { BidScheduleSetup } from "./components/BidScheduleSetup";
import { BidYearSetup } from "./components/BidYearSetup";
import { BootstrapCompleteness } from "./components/BootstrapCompleteness";
import { BootstrapOverview } from "./components/BootstrapOverview";
import { ConnectionStatus } from "./components/ConnectionStatus";
import { Navigation } from "./components/Navigation";
import { NoBidReview } from "./components/NoBidReview";
import { NoBidReviewWrapper } from "./components/NoBidReviewWrapper";
import { OperatorManagement } from "./components/OperatorManagement";
import { ReadinessReview } from "./components/ReadinessReview";
import { RoundGroupManagement } from "./components/RoundGroupManagement";
import { RoundGroupSetupWrapper } from "./components/RoundGroupSetupWrapper";
import { RoundManagement } from "./components/RoundManagement";
import { UserDetailView } from "./components/UserDetailView";
import { UserEditView } from "./components/UserEditView";
import { UserListView } from "./components/UserListView";
import { UserManagement } from "./components/UserManagement";
import type { GlobalCapabilities, LiveEvent } from "./types";
import { useLiveEvents } from "./useLiveEvents";
import "./styles/main.scss";

interface AuthState {
  isAuthenticated: boolean;
  sessionToken: string | null;
  loginName: string | null;
  displayName: string | null;
  role: string | null;
  capabilities: GlobalCapabilities | null;
}

interface BootstrapState {
  isBootstrapMode: boolean;
  bootstrapToken: string | null;
}

export function App() {
  return (
    <BrowserRouter>
      <AppRoutes />
    </BrowserRouter>
  );
}

function AppRoutes() {
  const [authState, setAuthState] = useState<AuthState>({
    isAuthenticated: false,
    sessionToken: null,
    loginName: null,
    displayName: null,
    role: null,
    capabilities: null,
  });

  const [bootstrapState, setBootstrapState] = useState<BootstrapState>({
    isBootstrapMode: false,
    bootstrapToken: null,
  });

  const [checkingAuth, setCheckingAuth] = useState(true);

  // Check bootstrap status and session on mount
  useEffect(() => {
    const checkAuth = async () => {
      try {
        // Check if we have a stored session
        const storedToken = localStorage.getItem("session_token");
        const storedLoginName = localStorage.getItem("login_name");
        const storedDisplayName = localStorage.getItem("display_name");
        const storedRole = localStorage.getItem("role");

        if (storedToken && storedLoginName && storedDisplayName && storedRole) {
          // Verify session is still valid and fetch capabilities
          try {
            console.log("[App] Calling whoami with stored token");
            const whoamiResponse = await api.whoami(storedToken);
            console.log("[App] Whoami response:", whoamiResponse);
            setAuthState({
              isAuthenticated: true,
              sessionToken: storedToken,
              loginName: storedLoginName,
              displayName: storedDisplayName,
              role: storedRole,
              capabilities: whoamiResponse.capabilities,
            });
            console.log(
              "[App] Auth state set with capabilities:",
              whoamiResponse.capabilities,
            );
          } catch (err) {
            // Session invalid, clear storage
            console.error("[App] Whoami failed:", err);
            localStorage.removeItem("session_token");
            localStorage.removeItem("login_name");
            localStorage.removeItem("display_name");
            localStorage.removeItem("role");
          }
        }

        // Check bootstrap status
        const bootstrapStatus = await api.checkBootstrapAuthStatus();
        setBootstrapState((prev) => ({
          ...prev,
          isBootstrapMode: bootstrapStatus.is_bootstrap_mode,
        }));
      } catch (error) {
        console.error("Failed to check auth status:", error);
      } finally {
        setCheckingAuth(false);
      }
    };

    checkAuth();
  }, []);

  const handleLogin = useCallback(
    async (
      sessionToken: string,
      loginName: string,
      displayName: string,
      role: string,
    ) => {
      localStorage.setItem("session_token", sessionToken);
      localStorage.setItem("login_name", loginName);
      localStorage.setItem("display_name", displayName);
      localStorage.setItem("role", role);

      // Fetch capabilities
      try {
        const whoamiResponse = await api.whoami(sessionToken);
        setAuthState({
          isAuthenticated: true,
          sessionToken,
          loginName,
          displayName,
          role,
          capabilities: whoamiResponse.capabilities,
        });
      } catch {
        // If we can't fetch capabilities, set them to null
        setAuthState({
          isAuthenticated: true,
          sessionToken,
          loginName,
          displayName,
          role,
          capabilities: null,
        });
      }
    },
    [],
  );

  const handleLogout = useCallback(async () => {
    if (authState.sessionToken) {
      try {
        await api.logout(authState.sessionToken);
      } catch (error) {
        console.error("Logout error:", error);
      }
    }
    localStorage.removeItem("session_token");
    localStorage.removeItem("login_name");
    localStorage.removeItem("display_name");
    localStorage.removeItem("role");
    setAuthState({
      isAuthenticated: false,
      sessionToken: null,
      loginName: null,
      displayName: null,
      role: null,
      capabilities: null,
    });
  }, [authState.sessionToken]);

  const handleBootstrapLogin = useCallback((bootstrapToken: string) => {
    setBootstrapState((prev) => ({
      ...prev,
      bootstrapToken,
    }));
  }, []);

  const handleBootstrapComplete = useCallback(() => {
    setBootstrapState({
      isBootstrapMode: false,
      bootstrapToken: null,
    });
  }, []);

  if (checkingAuth) {
    return (
      <div className="app">
        <div className="centered-loading">
          <p>Loading...</p>
        </div>
      </div>
    );
  }

  return (
    <Routes>
      <Route path="/" element={<PublicLandingPage />} />
      <Route
        path="/admin/*"
        element={
          <AdminRoutes
            authState={authState}
            bootstrapState={bootstrapState}
            onLogin={handleLogin}
            onLogout={handleLogout}
            onBootstrapLogin={handleBootstrapLogin}
            onBootstrapComplete={handleBootstrapComplete}
          />
        }
      />
    </Routes>
  );
}

function PublicLandingPage() {
  return (
    <div className="app">
      <div className="landing-container">
        <h1>Welcome to ZAB Bidding</h1>
      </div>
    </div>
  );
}

interface AdminRoutesProps {
  authState: AuthState;
  bootstrapState: BootstrapState;
  onLogin: (
    sessionToken: string,
    loginName: string,
    displayName: string,
    role: string,
  ) => void;
  onLogout: () => void;
  onBootstrapLogin: (bootstrapToken: string) => void;
  onBootstrapComplete: () => void;
}

function AdminRoutes({
  authState,
  bootstrapState,
  onLogin,
  onLogout,
  onBootstrapLogin,
  onBootstrapComplete,
}: AdminRoutesProps) {
  // If in bootstrap mode and have bootstrap token, show create first admin
  if (bootstrapState.isBootstrapMode && bootstrapState.bootstrapToken) {
    return <CreateFirstAdminPage onComplete={onBootstrapComplete} />;
  }

  // If in bootstrap mode, show bootstrap login
  if (bootstrapState.isBootstrapMode) {
    return <BootstrapLoginPage onBootstrapLogin={onBootstrapLogin} />;
  }

  // If not authenticated, show normal login
  if (!authState.isAuthenticated) {
    return <LoginPage onLogin={onLogin} />;
  }

  // Authenticated - show admin UI
  return <AuthenticatedAdminApp authState={authState} onLogout={onLogout} />;
}

interface BootstrapLoginPageProps {
  onBootstrapLogin: (bootstrapToken: string) => void;
}

function BootstrapLoginPage({ onBootstrapLogin }: BootstrapLoginPageProps) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    try {
      const response = await api.bootstrapLogin(username, password);
      onBootstrapLogin(response.bootstrap_token);
    } catch (err) {
      if (err instanceof api.ApiError) {
        setError(err.message);
      } else {
        setError("Bootstrap login failed");
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="app">
      <div className="auth-container">
        <div className="auth-card">
          <h2 className="auth-title">Bootstrap Login</h2>
          <p className="auth-description">
            The system has no operators. Use the bootstrap credentials to create
            the first admin.
          </p>
          <form className="auth-form" onSubmit={handleSubmit}>
            <div className="form-group">
              <label htmlFor="bootstrap-username">Username</label>
              <input
                id="bootstrap-username"
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="admin"
                required
                autoFocus
              />
            </div>
            <div className="form-group">
              <label htmlFor="bootstrap-password">Password</label>
              <input
                id="bootstrap-password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="admin"
                required
              />
            </div>
            {error && <div className="auth-error">{error}</div>}
            <button type="submit" disabled={loading}>
              {loading ? "Logging in..." : "Bootstrap Login"}
            </button>
          </form>
        </div>
      </div>
    </div>
  );
}

interface CreateFirstAdminPageProps {
  onComplete: () => void;
}

function CreateFirstAdminPage({ onComplete }: CreateFirstAdminPageProps) {
  const navigate = useNavigate();
  const [loginName, setLoginName] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (password !== confirmPassword) {
      setError("Passwords do not match");
      return;
    }

    if (password.length < 1) {
      setError("Password cannot be empty");
      return;
    }

    setLoading(true);

    try {
      await api.createFirstAdmin(
        loginName,
        displayName,
        password,
        confirmPassword,
      );
      onComplete();
      // Redirect to login page
      navigate("/admin");
    } catch (err) {
      if (err instanceof api.ApiError) {
        setError(err.message);
      } else {
        setError("Failed to create admin operator");
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="app">
      <div className="auth-container">
        <div className="auth-card auth-card-wide">
          <h2 className="auth-title">Create Initial Admin</h2>
          <p className="auth-description">
            Create the first admin operator. After creation, you will be logged
            out and must log in with these credentials.
          </p>
          <form className="auth-form" onSubmit={handleSubmit}>
            <div className="form-group">
              <label htmlFor="first-admin-login">Login Name</label>
              <input
                id="first-admin-login"
                type="text"
                value={loginName}
                onChange={(e) => setLoginName(e.target.value)}
                required
                autoFocus
              />
            </div>
            <div className="form-group">
              <label htmlFor="first-admin-display">Display Name</label>
              <input
                id="first-admin-display"
                type="text"
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
                required
              />
            </div>
            <div className="form-group">
              <label htmlFor="first-admin-password">Password</label>
              <input
                id="first-admin-password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
            </div>
            <div className="form-group">
              <label htmlFor="first-admin-confirm">Confirm Password</label>
              <input
                id="first-admin-confirm"
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                required
              />
            </div>
            {error && <div className="auth-error">{error}</div>}
            <button type="submit" disabled={loading} className="button-success">
              {loading ? "Creating..." : "Create Admin"}
            </button>
          </form>
        </div>
      </div>
    </div>
  );
}

interface LoginPageProps {
  onLogin: (
    sessionToken: string,
    loginName: string,
    displayName: string,
    role: string,
  ) => void;
}

function LoginPage({ onLogin }: LoginPageProps) {
  const [loginName, setLoginName] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    try {
      const response = await api.login(loginName, password);
      onLogin(
        response.session_token,
        response.login_name,
        response.display_name,
        response.role,
      );
    } catch (err) {
      if (err instanceof api.ApiError) {
        // Always show generic message for authentication errors
        if (err.status === 401) {
          setError("Invalid username or password");
        } else {
          setError(err.message);
        }
      } else if (err instanceof api.NetworkError) {
        setError(err.message);
      } else {
        setError("Login failed");
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="app">
      <div className="auth-container">
        <div className="auth-card">
          <h2 className="auth-title">Operator Login</h2>
          <form className="auth-form" onSubmit={handleSubmit}>
            <div className="form-group">
              <label htmlFor="login-name">Login Name</label>
              <input
                id="login-name"
                type="text"
                value={loginName}
                onChange={(e) => setLoginName(e.target.value)}
                required
                autoFocus
              />
            </div>
            <div className="form-group">
              <label htmlFor="login-password">Password</label>
              <input
                id="login-password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
            </div>
            {error && <div className="auth-error">{error}</div>}
            <button type="submit" disabled={loading}>
              {loading ? "Logging in..." : "Login"}
            </button>
          </form>
        </div>
      </div>
    </div>
  );
}

interface AuthenticatedAdminAppProps {
  authState: AuthState;
  onLogout: () => void;
}

function AuthenticatedAdminApp({
  authState,
  onLogout,
}: AuthenticatedAdminAppProps) {
  const [lastEvent, setLastEvent] = useState<LiveEvent | null>(null);
  const navigate = useNavigate();

  const handleLiveEvent = useCallback((event: LiveEvent) => {
    console.log("[Admin App] Received live event:", event);
    setLastEvent(event);
  }, []);

  const connectionState = useLiveEvents(handleLiveEvent);

  const handleLogoutClick = async () => {
    await onLogout();
    navigate("/admin");
  };

  return (
    <div className="app">
      <header className="app-header">
        <div className="header-left">
          <h1>ZAB Bidding Operators Interface</h1>
          <ConnectionStatus state={connectionState} />
        </div>
        <div className="header-right">
          <Navigation capabilities={authState.capabilities} />
          <div className="operator-info">
            <div className="operator-details">
              <div className="operator-name">{authState.displayName}</div>
              <div className="operator-meta">
                {authState.loginName} ({authState.role})
              </div>
            </div>
            <button
              type="button"
              onClick={handleLogoutClick}
              className="button-error"
            >
              Logout
            </button>
          </div>
        </div>
      </header>
      <main className="app-main">
        <Routes>
          <Route
            index
            element={
              <BootstrapOverview
                connectionState={connectionState}
                lastEvent={lastEvent}
              />
            }
          />
          <Route
            path="bootstrap"
            element={<Navigate to="/admin/bootstrap/bid-years" replace />}
          />
          <Route
            path="bootstrap/bid-years"
            element={
              authState.role === "Admin" && authState.sessionToken ? (
                <BidYearSetup
                  sessionToken={authState.sessionToken}
                  capabilities={authState.capabilities}
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              ) : (
                <Navigate to="/admin" replace />
              )
            }
          />
          <Route
            path="bootstrap/areas"
            element={
              authState.role === "Admin" && authState.sessionToken ? (
                <AreaSetup
                  sessionToken={authState.sessionToken}
                  capabilities={authState.capabilities}
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              ) : (
                <Navigate to="/admin" replace />
              )
            }
          />
          <Route
            path="bootstrap/users"
            element={
              authState.role === "Admin" && authState.sessionToken ? (
                <UserManagement
                  sessionToken={authState.sessionToken}
                  capabilities={authState.capabilities}
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              ) : (
                <Navigate to="/admin" replace />
              )
            }
          />
          <Route
            path="bootstrap/no-bid-review"
            element={
              authState.role === "Admin" && authState.sessionToken ? (
                <NoBidReviewWrapper
                  sessionToken={authState.sessionToken}
                  capabilities={authState.capabilities}
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              ) : (
                <Navigate to="/admin" replace />
              )
            }
          />
          <Route
            path="bootstrap/round-groups"
            element={
              authState.role === "Admin" && authState.sessionToken ? (
                <RoundGroupSetupWrapper
                  sessionToken={authState.sessionToken}
                  capabilities={authState.capabilities}
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              ) : (
                <Navigate to="/admin" replace />
              )
            }
          />
          <Route
            path="bootstrap/area-round-groups"
            element={
              authState.role === "Admin" && authState.sessionToken ? (
                <AreaRoundGroupAssignmentWrapper
                  sessionToken={authState.sessionToken}
                  capabilities={authState.capabilities}
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              ) : (
                <Navigate to="/admin" replace />
              )
            }
          />
          <Route
            path="bootstrap/schedule"
            element={
              authState.role === "Admin" && authState.sessionToken ? (
                <BidScheduleSetup
                  sessionToken={authState.sessionToken}
                  capabilities={authState.capabilities}
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              ) : (
                <Navigate to="/admin" replace />
              )
            }
          />
          <Route
            path="bootstrap/readiness"
            element={
              authState.role === "Admin" && authState.sessionToken ? (
                <ReadinessReview
                  sessionToken={authState.sessionToken}
                  capabilities={authState.capabilities}
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              ) : (
                <Navigate to="/admin" replace />
              )
            }
          />
          <Route
            path="bid-year/:bidYearId/no-bid-review"
            element={
              <NoBidReview
                sessionToken={authState.sessionToken}
                connectionState={connectionState}
                lastEvent={lastEvent}
              />
            }
          />
          <Route
            path="bid-year/:bidYearId/areas"
            element={
              <AreaView
                connectionState={connectionState}
                lastEvent={lastEvent}
              />
            }
          />
          <Route
            path="bid-year/:bidYearId/areas/:areaId/users"
            element={
              <UserListView
                sessionToken={authState.sessionToken}
                connectionState={connectionState}
                lastEvent={lastEvent}
              />
            }
          />
          <Route
            path="bid-year/:bidYearId/areas/:areaId/users/:userId"
            element={
              <UserDetailView
                connectionState={connectionState}
                lastEvent={lastEvent}
              />
            }
          />
          <Route
            path="bid-year/:bidYearId/areas/:areaId/users/:userId/edit"
            element={
              <UserEditView
                sessionToken={authState.sessionToken}
                connectionState={connectionState}
                lastEvent={lastEvent}
              />
            }
          />
          <Route
            path="round-groups"
            element={
              authState.role === "Admin" && authState.sessionToken ? (
                <RoundGroupManagement
                  sessionToken={authState.sessionToken}
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              ) : (
                <Navigate to="/admin" replace />
              )
            }
          />
          <Route
            path="round-groups/:roundGroupId/rounds"
            element={
              authState.role === "Admin" && authState.sessionToken ? (
                <RoundManagement
                  sessionToken={authState.sessionToken}
                  connectionState={connectionState}
                  lastEvent={lastEvent}
                />
              ) : (
                <Navigate to="/admin" replace />
              )
            }
          />
          <Route
            path="operators"
            element={
              authState.role === "Admin" && authState.sessionToken ? (
                <OperatorManagement
                  sessionToken={authState.sessionToken}
                  capabilities={authState.capabilities}
                />
              ) : (
                <Navigate to="/admin" replace />
              )
            }
          />
          <Route path="*" element={<Navigate to="/admin" replace />} />
        </Routes>
      </main>
    </div>
  );
}
